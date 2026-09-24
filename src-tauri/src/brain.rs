// "The big brain": Claude Code, run by Claw'd himself in the background.
//
// The Claude desktop app ships a Claude Code program, and people who use
// Claude Code have their own per-user install. Run headless (`claude -p`) in
// the Clawd folder it sees the user's claude.ai connectors (Gmail, Slack,
// Calendar…), so Claw'd can set himself up, check your apps on his own timer,
// and answer requests in his own chat, with no Claude window involved.
//
// Safety, by construction:
//   - Runs never load the user's own Claude Code settings or hooks
//     (--setting-sources project, and the Clawd folder has no project).
//   - Files can only be read and written inside the Clawd folder.
//   - Anything that goes to another person (send, post, reply, invite,
//     share, delete…) is never pre-allowed. The run stops at it, the page
//     shows the exact draft, and only a tap re-runs it with that one tool.
//   - No shell (Bash) at all.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::PoisonTolerant;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// %APPDATA%\ClawdAssistant\brain.log: one line per check and run (which
/// program, which model, how it went, any error text), so a problem on a
/// machine we can't see can be read back. Never the prompts or the replies.
/// Kept small: over 256 KB it starts again.
pub fn log(line: &str) {
    log_to("brain.log", line);
}

/// The same, to any log file in %APPDATA%\ClawdAssistant (page.log is the
/// webview's: its errors and the clicks that led up to them).
pub fn log_to(file: &str, line: &str) {
    use std::io::Write;
    let Some(path) = crate::app_data_dir().map(|d| d.join(file)) else { return };
    if std::fs::metadata(&path).map(|m| m.len() > 256 * 1024).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let when = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(f, "{when} {}", line.replace('\n', " / "));
    }
}

/// Tools the big brain may never use without a tap, matched on the part of
/// the tool name after the server ("mcp__claude_ai_Gmail__send_email" ->
/// "send_email"). Drafting, reading, searching and updating your own things
/// stay allowed.
pub fn reaches_other_people(tool: &str) -> bool {
    let short = tool.rsplit("__").next().unwrap_or(tool).to_ascii_lowercase();
    const WORDS: &[&str] = &[
        "send", "post", "reply", "forward", "invite", "share", "publish", "comment", "delete",
        "remove", "trash", "respond",
    ];
    // Slack-style "chat_postMessage", "postmessage"
    if short.contains("postmessage") || short.contains("sendmessage") {
        return true;
    }
    let parts: Vec<&str> = short.split(|c: char| c == '_' || c == '-').collect();
    // an event with guests goes out to people; a plain draft does not
    if short.contains("draft") && !parts.contains(&"send") {
        return false;
    }
    if (short.contains("create") || short.contains("update")) && short.contains("event") {
        return true;
    }
    parts.iter().any(|p| WORDS.contains(p))
}

// --- finding the program -------------------------------------------------

fn version_key(p: &Path) -> Vec<u64> {
    // .../claude-code/2.1.281/claude.exe -> [2, 1, 281]
    p.parent()
        .and_then(|d| d.file_name())
        .map(|n| n.to_string_lossy().split('.').map(|x| x.parse().unwrap_or(0)).collect())
        .unwrap_or_default()
}

fn newest_in(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path().join("claude.exe"))
        .filter(|p| p.is_file())
        .collect();
    found.sort_by_key(|p| version_key(p));
    found.pop()
}

/// The user's own Claude Code first (on PATH, or the per-user install), then
/// the copy inside the Claude desktop app, then the Store app's copy.
pub fn find_claude() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir.join("claude.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from);
    if let Some(p) = home.as_ref().map(|h| h.join(".local").join("bin").join("claude.exe")) {
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(p) = std::env::var_os("APPDATA").and_then(|a| newest_in(&PathBuf::from(a).join("Claude").join("claude-code"))) {
        return Some(p);
    }
    let packages = std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("Packages"))?;
    std::fs::read_dir(packages)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("Claude_"))
        .find_map(|e| newest_in(&e.path().join("LocalCache").join("Roaming").join("Claude").join("claude-code")))
}

// --- what the page hears ---------------------------------------------------

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Server {
    pub name: String,
    /// connected | needs-auth | failed | pending
    pub status: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Denial {
    pub tool: String,
    pub input: serde_json::Value,
}

/// One line of a run, as the page gets it (event "brain").
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BrainEvent {
    pub job: String,
    /// init | text | tool | result | error
    pub kind: String,
    pub text: String,
    pub session: String,
    pub servers: Vec<Server>,
    pub denials: Vec<Denial>,
    pub is_error: bool,
}

impl BrainEvent {
    fn new(job: &str, kind: &str) -> Self {
        BrainEvent {
            job: job.into(),
            kind: kind.into(),
            text: String::new(),
            session: String::new(),
            servers: Vec::new(),
            denials: Vec::new(),
            is_error: false,
        }
    }
}

/// Turn one stream-json line into what the page needs (None for lines it
/// doesn't care about). Also collects the tool names from the init line.
pub fn parse_line(job: &str, line: &str, tools: &mut Vec<String>) -> Option<BrainEvent> {
    let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    match (s("type").as_str(), s("subtype").as_str()) {
        ("system", "init") => {
            let mut e = BrainEvent::new(job, "init");
            e.session = s("session_id");
            e.servers = v
                .get("mcp_servers")
                .and_then(|m| m.as_array())
                .map(|a| {
                    a.iter()
                        .map(|m| Server {
                            name: m.get("name").and_then(|x| x.as_str()).unwrap_or("").trim_start_matches("claude.ai ").to_string(),
                            status: m.get("status").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            if let Some(a) = v.get("tools").and_then(|t| t.as_array()) {
                *tools = a.iter().filter_map(|t| t.as_str().map(String::from)).collect();
            }
            Some(e)
        }
        ("assistant", _) => {
            let blocks = v.pointer("/message/content").and_then(|c| c.as_array())?;
            let mut text = String::new();
            let mut tool = String::new();
            for b in blocks {
                match b.get("type").and_then(|t| t.as_str()) {
                    Some("text") => text.push_str(b.get("text").and_then(|t| t.as_str()).unwrap_or("")),
                    Some("tool_use") => tool = b.get("name").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                    _ => {}
                }
            }
            if !text.trim().is_empty() {
                let mut e = BrainEvent::new(job, "text");
                e.text = text;
                e.session = s("session_id");
                Some(e)
            } else if !tool.is_empty() {
                let mut e = BrainEvent::new(job, "tool");
                e.text = tool;
                Some(e)
            } else {
                None
            }
        }
        ("result", sub) => {
            let mut e = BrainEvent::new(job, "result");
            e.text = s("result");
            e.session = s("session_id");
            e.is_error = v.get("is_error").and_then(|x| x.as_bool()).unwrap_or(false) || sub.starts_with("error");
            if e.is_error && e.text.is_empty() {
                e.text = sub.to_string();
            }
            e.denials = v
                .get("permission_denials")
                .and_then(|d| d.as_array())
                .map(|a| {
                    a.iter()
                        .map(|d| Denial {
                            tool: d.get("tool_name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                            input: d.get("tool_input").cloned().unwrap_or(serde_json::Value::Null),
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(e)
        }
        _ => None,
    }
}

// --- running it ----------------------------------------------------------------

#[derive(Default)]
pub struct Brain {
    /// every tool name the last run reported (connectors included), so the
    /// next run can pre-allow the safe ones
    pub tools: Mutex<Vec<String>>,
    /// one run at a time; the current child, so it can be stopped
    pub running: Mutex<Option<(String, Child)>>,
    /// held for the whole of a run: a second run waits its turn
    pub turn: Arc<Mutex<()>>,
}

/// What a run may do without asking.
fn allowed_tools(tools: &[String], also: &[String]) -> Vec<String> {
    let mut out: Vec<String> = vec![
        "Read(./**)".into(),
        "Write(./**)".into(),
        "Edit(./**)".into(),
        "Glob".into(),
        "Grep".into(),
    ];
    out.extend(tools.iter().filter(|t| t.starts_with("mcp__") && !reaches_other_people(t)).cloned());
    out.extend(also.iter().cloned()); // one confirmed send
    out
}

pub struct RunSpec {
    pub job: String,
    pub prompt: String,
    pub system: String,
    pub model: String,
    /// auto | plan
    pub mode: String,
    pub resume: Option<String>,
    /// tools the user just confirmed (one send)
    pub also_allow: Vec<String>,
    pub max_turns: u32,
}

fn command(exe: &Path, dir: &Path, spec: &RunSpec, tools: &[String]) -> Command {
    use std::os::windows::process::CommandExt;
    let mut c = Command::new(exe);
    c.current_dir(dir)
        .arg("-p")
        .arg(&spec.prompt)
        .args(["--output-format", "stream-json", "--verbose"])
        .args(["--setting-sources", "project"])
        .args(["--max-turns", &spec.max_turns.to_string()])
        .args(["--disallowedTools", "Bash"]);
    if !spec.model.is_empty() {
        c.args(["--model", &spec.model]);
    }
    if !spec.system.is_empty() {
        c.args(["--append-system-prompt", &spec.system]);
    }
    if spec.mode == "plan" {
        c.args(["--permission-mode", "plan"]);
    } else {
        c.args(["--allowedTools", &allowed_tools(tools, &spec.also_allow).join(",")]);
    }
    if let Some(r) = spec.resume.as_deref().filter(|r| !r.is_empty()) {
        c.args(["--resume", r]);
    }
    c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).creation_flags(CREATE_NO_WINDOW);
    c
}

/// Start a run on its own thread; every line goes to the page as a "brain"
/// event, ending with exactly one "result" or "error".
pub fn spawn_run(app: AppHandle, brain: Arc<Brain>, dir: PathBuf, spec: RunSpec) {
    std::thread::spawn(move || {
        let emit = |e: BrainEvent| {
            let _ = app.emit_to("pet", "brain", e);
        };
        let job = spec.job.clone();
        let fail = |text: String| {
            let mut e = BrainEvent::new(&job, "error");
            e.text = text;
            e.is_error = true;
            e
        };
        let Some(exe) = find_claude() else {
            log(&format!("run {}: no claude.exe found", spec.job));
            emit(fail("not-found".into()));
            return;
        };
        log(&format!("run {}: start ({}, {}, {})", spec.job, exe.display(), if spec.model.is_empty() { "default" } else { &spec.model }, spec.mode));
        let turn = brain.turn.clone();
        let _turn = turn.lock_or_recover();
        let tools = brain.tools.lock_or_recover().clone();
        let mut child = match command(&exe, &dir, &spec, &tools).spawn() {
            Ok(c) => c,
            Err(e) => {
                log(&format!("run {}: could not start: {e}", spec.job));
                emit(fail(format!("could not start: {e}")));
                return;
            }
        };
        let stdout = child.stdout.take();
        let mut stderr = child.stderr.take();
        *brain.running.lock_or_recover() = Some((job.clone(), child));

        let mut got_result = false;
        let mut seen_tools = Vec::new();
        if let Some(out) = stdout {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if let Some(ev) = parse_line(&job, &line, &mut seen_tools) {
                    got_result |= ev.kind == "result";
                    if ev.kind == "result" {
                        let denied: Vec<&str> = ev.denials.iter().map(|d| d.tool.as_str()).collect();
                        log(&format!("run {job}: done error={} denied=[{}]{}", ev.is_error, denied.join(", "),
                            if ev.is_error { format!(" text: {}", ev.text.chars().take(300).collect::<String>()) } else { String::new() }));
                    }
                    emit(ev);
                }
            }
        }
        if !seen_tools.is_empty() {
            *brain.tools.lock_or_recover() = seen_tools;
        }
        let mut err_text = String::new();
        if let Some(e) = stderr.as_mut() {
            let _ = e.take(8192).read_to_string(&mut err_text);
        }
        if let Some((_, mut c)) = brain.running.lock_or_recover().take() {
            let _ = c.wait();
        }
        if !got_result {
            let t = err_text.trim();
            log(&format!("run {job}: ended with no result; stderr: {}", t.chars().take(600).collect::<String>()));
            emit(fail(if t.is_empty() { "stopped".into() } else { classify(t).into() }));
        }
    });
}

/// Stderr of a failed start, boiled down to something the page can say.
fn classify(err: &str) -> &'static str {
    let e = err.to_ascii_lowercase();
    if e.contains("login") || e.contains("log in") || e.contains("api key") || e.contains("unauthor") || e.contains("oauth") {
        "not-logged-in"
    } else if e.contains("rate") && e.contains("limit") || e.contains("usage limit") {
        "rate-limited"
    } else if e.contains("network") || e.contains("enotfound") || e.contains("econn") {
        "offline"
    } else {
        "failed"
    }
}

pub fn stop(brain: &Brain) {
    if let Some((_, mut c)) = brain.running.lock_or_recover().take() {
        let _ = c.kill();
    }
}

/// Open the big brain in a window of its own, for the one-time bits only a
/// person can do: logging in, and saying yes to its apps (/mcp).
pub fn open_window(dir: &Path, first_command: &str) -> bool {
    let Some(exe) = find_claude() else { return false };
    Command::new("cmd")
        .current_dir(dir)
        .args(["/C", "start", "Claw'd - the big brain"])
        .arg(exe)
        .arg(first_command)
        .spawn()
        .is_ok()
}

#[derive(Serialize, Clone, Debug)]
pub struct Status {
    /// a Claude Code program was found at all
    pub found: bool,
    /// it answered: logged in and able to run
    pub ready: bool,
    /// not-found | not-logged-in | rate-limited | offline | failed | "" (fine)
    pub problem: String,
    pub servers: Vec<Server>,
}

/// A tiny run ("reply OK" on the smallest model) that answers: is there a big
/// brain, can it run, and which of its apps are connected or need a yes.
/// Blocks for up to `secs`; call it off the main thread.
pub fn probe(brain: &Brain, dir: &Path, model: &str, secs: u64) -> Status {
    let Some(exe) = find_claude() else {
        log("probe: no claude.exe found (PATH, ~/.local/bin, the Claude app)");
        return Status { found: false, ready: false, problem: "not-found".into(), servers: vec![] };
    };
    log(&format!("probe: {} ({model})", exe.display()));
    let spec = RunSpec {
        job: "probe".into(),
        prompt: "Reply with just OK".into(),
        system: String::new(),
        model: model.into(),
        mode: "auto".into(),
        resume: None,
        also_allow: vec![],
        max_turns: 1,
    };
    let _turn = brain.turn.lock_or_recover();
    let tools = brain.tools.lock_or_recover().clone();
    let mut child = match command(&exe, dir, &spec, &tools).spawn() {
        Ok(c) => c,
        Err(e) => {
            log(&format!("probe: could not start: {e}"));
            return Status { found: true, ready: false, problem: "failed".into(), servers: vec![] };
        }
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let out = child.stdout.take();
    std::thread::spawn(move || {
        let mut seen = Vec::new();
        if let Some(out) = out {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if let Some(e) = parse_line("probe", &line, &mut seen) {
                    let _ = tx.send((e, seen.clone()));
                }
            }
        }
    });
    let mut status = Status { found: true, ready: false, problem: String::new(), servers: vec![] };
    let deadline = std::time::Instant::now() + Duration::from_secs(secs);
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        match rx.recv_timeout(left) {
            Ok((e, seen)) => {
                if e.kind == "init" {
                    status.servers = e.servers.clone();
                    if !seen.is_empty() {
                        *brain.tools.lock_or_recover() = seen;
                    }
                }
                if e.kind == "result" {
                    status.ready = !e.is_error;
                    if e.is_error {
                        status.problem = classify(&e.text).into();
                    }
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if !status.ready && status.problem.is_empty() {
        let mut err = String::new();
        if let Some(e) = child.stderr.take() {
            let _ = child.kill();
            let _ = e.take(8192).read_to_string(&mut err);
        }
        status.problem = if err.trim().is_empty() { "failed".into() } else { classify(&err).into() };
        log(&format!("probe: no answer; stderr: {}", err.trim().chars().take(600).collect::<String>()));
    }
    let apps: Vec<String> = status.servers.iter().map(|s| format!("{}={}", s.name, s.status)).collect();
    log(&format!("probe: ready={} problem={} apps: {}", status.ready, status.problem, apps.join(", ")));
    let _ = child.kill();
    let _ = child.wait();
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_need_a_tap() {
        assert!(reaches_other_people("mcp__claude_ai_Gmail__send_email"));
        assert!(reaches_other_people("mcp__claude_ai_Slack__slack_send_message"));
        assert!(reaches_other_people("mcp__claude_ai_Slack__post_message"));
        assert!(reaches_other_people("mcp__claude_ai_Google_Calendar__create_event"));
        assert!(reaches_other_people("mcp__claude_ai_Google_Drive__share_file"));
        assert!(!reaches_other_people("mcp__claude_ai_Gmail__create_draft"));
        assert!(!reaches_other_people("mcp__claude_ai_Gmail__search_threads"));
        assert!(!reaches_other_people("mcp__claude_ai_Google_Calendar__list_events"));
        assert!(!reaches_other_people("mcp__claude_ai_Asana__update_task"));
        assert!(!reaches_other_people("mcp__claude_ai_Slack__slack_read_channel_messages"));
        assert!(!reaches_other_people("mcp__claude_ai_Gmail__get_message"));
        assert!(reaches_other_people("mcp__x__chat_postMessage"));
    }

    #[test]
    fn parses_the_stream() {
        let mut tools = Vec::new();
        let init = r#"{"type":"system","subtype":"init","session_id":"s1","tools":["Read","mcp__claude_ai_Gmail__search"],"mcp_servers":[{"name":"claude.ai Gmail","status":"needs-auth"}]}"#;
        let e = parse_line("j", init, &mut tools).unwrap();
        assert_eq!(e.kind, "init");
        assert_eq!(e.servers, vec![Server { name: "Gmail".into(), status: "needs-auth".into() }]);
        assert_eq!(tools.len(), 2);

        let text = r#"{"type":"assistant","session_id":"s1","message":{"content":[{"type":"text","text":"Done! I checked."}]}}"#;
        assert_eq!(parse_line("j", text, &mut tools).unwrap().text, "Done! I checked.");

        let tool = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"mcp__x__list","input":{}}]}}"#;
        assert_eq!(parse_line("j", tool, &mut tools).unwrap().kind, "tool");

        let result = r#"{"type":"result","subtype":"success","is_error":false,"result":"OK","session_id":"s1","permission_denials":[{"tool_name":"mcp__claude_ai_Gmail__send_email","tool_use_id":"t","tool_input":{"to":"a@b.c"}}]}"#;
        let r = parse_line("j", result, &mut tools).unwrap();
        assert_eq!(r.kind, "result");
        assert_eq!(r.denials.len(), 1);
        assert_eq!(r.denials[0].tool, "mcp__claude_ai_Gmail__send_email");

        assert!(parse_line("j", r#"{"type":"rate_limit_event"}"#, &mut tools).is_none());
        assert!(parse_line("j", "not json", &mut tools).is_none());
    }

    #[test]
    fn safe_tools_are_preallowed_and_sends_are_not() {
        let tools = vec!["mcp__a__list_events".to_string(), "mcp__a__send_email".to_string(), "Bash".to_string()];
        let allowed = allowed_tools(&tools, &[]);
        assert!(allowed.contains(&"mcp__a__list_events".to_string()));
        assert!(!allowed.contains(&"mcp__a__send_email".to_string()));
        assert!(!allowed.contains(&"Bash".to_string()));
        let with = allowed_tools(&tools, &["mcp__a__send_email".to_string()]);
        assert!(with.contains(&"mcp__a__send_email".to_string()));
    }

    /// The real thing, on this machine: cargo test live_ -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_probe_and_run() {
        let brain = Brain::default();
        let dir = std::env::temp_dir();
        let st = probe(&brain, &dir, "claude-haiku-4-5-20251001", 120);
        println!("status: {st:?}");
        assert!(st.found, "no claude.exe found");
        assert!(st.ready, "probe failed: {}", st.problem);
        // a full run with every flag the pet passes (allowed tools, system
        // prompt, working folder), read back through parse_line
        let exe = find_claude().unwrap();
        let tools = brain.tools.lock_or_recover().clone();
        let spec = RunSpec {
            job: "live".into(),
            prompt: "Job: request
Mode: auto

What is 2 plus 2? Answer in one short sentence.".into(),
            system: crate::feed::SKILL.into(),
            model: "claude-haiku-4-5-20251001".into(),
            mode: "auto".into(),
            resume: None,
            also_allow: vec![],
            max_turns: 3,
        };
        let out = command(&exe, &dir, &spec, &tools).output().expect("ran");
        let mut seen = Vec::new();
        let events: Vec<BrainEvent> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| parse_line("live", l, &mut seen))
            .collect();
        let result = events.iter().find(|e| e.kind == "result").expect("a result line");
        println!("reply: {}", result.text);
        println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
        assert!(!result.is_error);
    }

    #[test]
    fn newest_version_wins() {
        let mut v = vec![
            PathBuf::from(r"C:\x\claude-code\2.1.9\claude.exe"),
            PathBuf::from(r"C:\x\claude-code\2.1.281\claude.exe"),
            PathBuf::from(r"C:\x\claude-code\2.1.30\claude.exe"),
        ];
        v.sort_by_key(|p| version_key(p));
        assert!(v.last().unwrap().to_string_lossy().contains("2.1.281"));
    }
}
