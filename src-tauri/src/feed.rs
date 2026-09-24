// How Claude reaches Claw'd. Two doors into the same channel, carrying the
// same item format (state.rs Item), because which one a given Claude can use
// depends on the machine and the surface it runs in:
//
//   folder — Claude writes a JSON file into <clawd dir>\inbox. Anything that
//            can write a file can do this, including Cowork once the folder
//            is shared with it. The default, and the one the skill uses.
//   http   — POST http://127.0.0.1:<port>/items with a bearer token, for a
//            Claude that can run curl (Claude Code, a local MCP tool).
//
// The clawd dir is %USERPROFILE%\Clawd rather than Documents on purpose:
// work laptops commonly redirect Documents into OneDrive, and nothing here —
// message snippets from Slack and email — should be synced anywhere.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::state::{now_ms, Item, PetEvent, PetStatePayload};
use crate::PoisonTolerant;

const MAX_BODY: u64 = 1024 * 1024;
const POLL: Duration = Duration::from_secs(2);
/// a file this fresh that fails to parse may still be being written
const SETTLE: Duration = Duration::from_secs(5);
const DONE_KEEP: Duration = Duration::from_secs(2 * 24 * 3600);

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct FeedConfig {
    /// override for %USERPROFILE%\Clawd
    pub clawd_dir: Option<String>,
    pub http_port: u16,
    /// generated on first run; also written to <clawd dir>\http.json so the
    /// skill can read it
    pub http_token: String,
    /// how often Claude is expected to sweep, until a heartbeat says otherwise
    pub sweep_minutes: u32,
    /// how long a waiting item may sit before he asks again
    pub nudge_minutes: u32,
    /// hosts beyond the built-in list that item links may open (see
    /// main.rs open_link), e.g. "yourcompany.zoom.us"
    pub extra_link_hosts: Vec<String>,
}

impl Default for FeedConfig {
    fn default() -> Self {
        FeedConfig {
            clawd_dir: None,
            http_port: 4318, // 4317 belongs to the coding Claw'd
            http_token: String::new(),
            sweep_minutes: 15,
            nudge_minutes: 20,
            extra_link_hosts: Vec::new(),
        }
    }
}

pub fn clawd_dir(cfg: &FeedConfig) -> PathBuf {
    if let Some(d) = cfg.clawd_dir.as_deref().filter(|d| !d.trim().is_empty()) {
        return PathBuf::from(d);
    }
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Clawd")
}

/// 128 bits from the standard library's OS-seeded hasher keys. Enough to stop
/// a web page in a browser from posting to the loopback port, which is the
/// threat this token exists for.
pub fn new_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(i ^ now_ms());
        h.write_u32(std::process::id());
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

/// Accepts a single item, an array of items, or {"items": [...]}: whatever
/// shape the writer found natural.
pub fn parse_batch(text: &str) -> Result<Vec<Item>, String> {
    let text = text.trim_start_matches('\u{feff}'); // PowerShell writes a BOM
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let list = match v {
        serde_json::Value::Array(a) => a,
        serde_json::Value::Object(ref o) if o.get("items").map_or(false, |i| i.is_array()) => {
            o["items"].as_array().cloned().unwrap_or_default()
        }
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => return Err("expected an item, a list of items, or {\"items\": [...]}".into()),
    };
    let mut items = Vec::with_capacity(list.len());
    let mut bad = 0;
    for v in list {
        match serde_json::from_value::<Item>(v) {
            Ok(it) if !it.id.trim().is_empty() => items.push(it),
            _ => bad += 1,
        }
    }
    if items.is_empty() && bad > 0 {
        return Err(format!("{bad} item(s) without a usable \"id\""));
    }
    Ok(items)
}

const SKILL: &str = include_str!("../../skill/clawd/SKILL.md");

const README: &str = "\
This folder is how Claude talks to Claw'd, the desktop assistant pet.

  inbox\\        Claude drops JSON files here. Claw'd reads each one within a
                couple of seconds and moves it to inbox\\done (or inbox\\bad if
                it could not be read). Write to a .tmp name, then rename to .json.
  prefs.json    Your preferences (VIPs, channels, work hours, rules). Written
                by Claude during setup; ask Claude to change it.
  intro.json    What Claw'd learned about you when he hatched (your name,
                VIPs, hours). Claude turns it into prefs.json.
  sent.json     Claude's own notes between checks (what it already told Claw'd).
  http.json     The local port and token, for a Claude that prefers HTTP.
  SKILL.md      Claude's instructions for looking after Claw'd. Rewritten
                every time Claw'd starts, so edits here don't stick.

Nothing in here leaves this computer. Deleting the folder is safe; Claw'd
recreates it on the next start.
";

/// Create the folder layout and publish the HTTP coordinates. Returns the
/// inbox path.
pub fn prepare_dir(cfg: &FeedConfig) -> PathBuf {
    let dir = clawd_dir(cfg);
    let inbox = dir.join("inbox");
    let _ = std::fs::create_dir_all(inbox.join("done"));
    let _ = std::fs::create_dir_all(inbox.join("bad"));
    let readme = dir.join("README.txt");
    if !readme.exists() {
        let _ = std::fs::write(&readme, README);
    }
    // the skill that matches THIS build, refreshed every start: a scheduled
    // task can follow the file even where the skill itself isn't installed
    let skill = dir.join("SKILL.md");
    if std::fs::read_to_string(&skill).map_or(true, |s| s != SKILL) {
        let _ = std::fs::write(&skill, SKILL);
    }
    let http = serde_json::json!({
        "url": format!("http://127.0.0.1:{}/items", cfg.http_port),
        "token": cfg.http_token,
    });
    let _ = std::fs::write(dir.join("http.json"), serde_json::to_string_pretty(&http).unwrap_or_default());
    inbox
}

fn move_into(file: &Path, dir: &Path) {
    let name = file.file_name().map(|n| n.to_owned()).unwrap_or_default();
    let mut target = dir.join(&name);
    if target.exists() {
        target = dir.join(format!("{}-{}", now_ms(), name.to_string_lossy()));
    }
    if std::fs::rename(file, &target).is_err() {
        // a file we can neither parse nor move would be re-read forever
        let _ = std::fs::remove_file(file);
    }
}

fn prune(dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let now = SystemTime::now();
    for e in rd.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .map_or(false, |age| age > DONE_KEEP);
        if old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

pub fn spawn_folder_watch(inbox: PathBuf, tx: Sender<PetEvent>) {
    std::thread::spawn(move || {
        let mut ticks: u64 = 0;
        loop {
            if ticks % 1800 == 0 {
                prune(&inbox.join("done"));
                prune(&inbox.join("bad"));
            }
            ticks += 1;
            let mut files: Vec<PathBuf> = std::fs::read_dir(&inbox)
                .map(|rd| {
                    rd.flatten()
                        .map(|e| e.path())
                        .filter(|p| {
                            p.is_file()
                                && p.extension().map_or(false, |x| x.eq_ignore_ascii_case("json"))
                        })
                        .collect()
                })
                .unwrap_or_default();
            files.sort(); // names are usually timestamps: oldest batch first
            for f in files {
                let text = match std::fs::read_to_string(&f) {
                    Ok(t) => t,
                    Err(_) => continue, // locked by the writer; next tick
                };
                match parse_batch(&text) {
                    Ok(items) => {
                        let _ = tx.send(PetEvent::Items(items, "folder"));
                        move_into(&f, &inbox.join("done"));
                    }
                    Err(e) => {
                        let fresh = std::fs::metadata(&f)
                            .and_then(|m| m.modified())
                            .ok()
                            .and_then(|t| SystemTime::now().duration_since(t).ok())
                            .map_or(false, |age| age < SETTLE);
                        if fresh {
                            continue;
                        }
                        let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        eprintln!("clawd: {name}: {e}");
                        let _ = tx.send(PetEvent::FeedError(format!("{name}: {e}")));
                        move_into(&f, &inbox.join("bad"));
                    }
                }
            }
            std::thread::sleep(POLL);
        }
    });
}

pub fn spawn_http(
    port: u16,
    token: String,
    tx: Sender<PetEvent>,
    store: Arc<Mutex<PetStatePayload>>,
) -> Result<(), String> {
    let addr = format!("127.0.0.1:{port}");
    let server = tiny_http::Server::http(&addr).map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let (tx, store, token) = (tx.clone(), store.clone(), token.clone());
            // per-request thread: a client that sends headers and stalls must
            // not park the accept loop
            let _ = std::thread::Builder::new()
                .name("feed-http".into())
                .spawn(move || serve(request, &tx, &store, &token));
        }
    });
    Ok(())
}

fn header<'a>(req: &'a tiny_http::Request, name: &str) -> Option<&'a str> {
    req.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

fn serve(mut req: tiny_http::Request, tx: &Sender<PetEvent>, store: &Mutex<PetStatePayload>, token: &str) {
    let respond = |req: tiny_http::Request, code: u16, body: String| {
        let resp = tiny_http::Response::from_string(body)
            .with_status_code(code)
            .with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).expect("static header"),
            );
        let _ = req.respond(resp);
    };
    let path = req.url().split('?').next().unwrap_or("").to_string();
    let method = req.method().clone();

    if method == tiny_http::Method::Get && path == "/health" {
        return respond(req, 200, r#"{"ok":true,"app":"clawd-assistant"}"#.into());
    }
    // Browsers always send Origin on a cross-site POST; curl and friends
    // don't. Refusing it outright means no web page can drive the pet even
    // if the token leaked.
    if header(&req, "Origin").is_some() {
        return respond(req, 403, r#"{"error":"browser requests are not accepted"}"#.into());
    }
    let presented = header(&req, "Authorization")
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| header(&req, "X-Clawd-Token"))
        .unwrap_or("")
        .trim()
        .to_string();
    if token.is_empty() || presented != token {
        return respond(req, 401, r#"{"error":"missing or wrong token (see http.json in the Clawd folder)"}"#.into());
    }

    match (method, path.as_str()) {
        (tiny_http::Method::Post, "/items") => {
            let mut body = String::new();
            if req.as_reader().take(MAX_BODY).read_to_string(&mut body).is_err() {
                return respond(req, 400, r#"{"error":"unreadable body"}"#.into());
            }
            match parse_batch(&body) {
                Ok(items) => {
                    let n = items.len();
                    let _ = tx.send(PetEvent::Items(items, "http"));
                    respond(req, 200, format!(r#"{{"accepted":{n}}}"#))
                }
                Err(e) => respond(req, 400, serde_json::json!({ "error": e }).to_string()),
            }
        }
        (tiny_http::Method::Get, "/state") => {
            let snap = store.lock_or_recover().clone();
            respond(req, 200, serde_json::to_string(&snap).unwrap_or_else(|_| "{}".into()))
        }
        _ => respond(req, 404, r#"{"error":"not found"}"#.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_all_three_shapes() {
        assert_eq!(parse_batch(r#"{"id":"a"}"#).unwrap().len(), 1);
        assert_eq!(parse_batch(r#"[{"id":"a"},{"id":"b"}]"#).unwrap().len(), 2);
        assert_eq!(parse_batch(r#"{"items":[{"id":"a"}]}"#).unwrap().len(), 1);
        assert_eq!(parse_batch("\u{feff}[]").unwrap().len(), 0);
    }

    #[test]
    fn rejects_garbage_and_idless() {
        assert!(parse_batch("nope").is_err());
        assert!(parse_batch(r#"[{"title":"no id"}]"#).is_err());
        // one bad apple doesn't spoil the batch
        assert_eq!(parse_batch(r#"[{"title":"no id"},{"id":"ok"}]"#).unwrap().len(), 1);
    }

    #[test]
    fn tokens_differ() {
        assert_ne!(new_token(), new_token());
        assert_eq!(new_token().len(), 32);
    }
}
