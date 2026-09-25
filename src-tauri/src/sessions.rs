// Claude Code sessions, for the dev pack: the coding Claw'd's hook feed,
// brought over. Claude Code's hooks POST every event to 127.0.0.1:4317; this
// turns them into one state per session (working, thinking, needs you, done,
// error, idle) and publishes the list to the page as "sessions".
//
// Ported from clawdbot (src-tauri/src/hooks.rs and state.rs), whose mapping
// was grounded in 676 captured hook payloads; the Cowork watcher stays
// behind for now. Rules kept from there:
// - answer `200 {}` at once, always: a slow or broken pet must never stall a
//   Claude turn, and anything malformed is dropped, never Claude's problem
// - "needs you" never decays on a timer; only the next real event clears it
// - working shows as "thinking" once no tool has run for a beat
//
// His own big brain runs never show up here: they run with
// `--setting-sources project`, so the user's hooks don't fire for them.

use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

pub const HOOK_ADDR: &str = "127.0.0.1:4317";
const HOOK_URL: &str = "http://127.0.0.1:4317/event";
const MAX_BODY: u64 = 4 * 1024 * 1024; // tool_response can embed whole files
const DETAIL_MAX: usize = 120;
const DONE_TO_IDLE: Duration = Duration::from_secs(10);
const ERROR_TO_IDLE: Duration = Duration::from_secs(30);
const ORPHAN_DROP: Duration = Duration::from_secs(10 * 60);
const ORPHAN_DROP_OPEN_TOOLS: Duration = Duration::from_secs(15 * 60);
const THINK_LAG: Duration = Duration::from_secs(3);
const THINK_STALE: Duration = Duration::from_secs(3 * 60);
const UNENGAGED_DROP: Duration = Duration::from_secs(2 * 60);

/// One hook POST, every field optional: hooks are an internal surface.
#[derive(Debug, Clone, Deserialize)]
pub struct HookEvent {
    pub hook_event_name: Option<String>,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub tool_use_id: Option<String>,
    pub prompt: Option<String>,
    pub message: Option<String>,
    pub notification_type: Option<String>,
    pub error: Option<String>,
    pub is_interrupt: Option<bool>,
    pub last_assistant_message: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum S {
    NeedsInput,
    Error,
    Working,
    Done,
    Idle,
}

impl S {
    fn rank(self) -> u8 {
        match self {
            S::NeedsInput => 5,
            S::Error => 4,
            S::Working => 3,
            S::Done => 2,
            S::Idle => 1,
        }
    }
    fn name(self) -> &'static str {
        match self {
            S::NeedsInput => "needs_input",
            S::Error => "error",
            S::Working => "working",
            S::Done => "done",
            S::Idle => "idle",
        }
    }
}

struct Session {
    cwd: String,
    state: S,
    kind: String,
    detail: String,
    since: Instant,
    last_event_at: Instant,
    last_tool_at: Instant,
    open_tools: Vec<(String, String)>,
    engaged: bool,
}

impl Session {
    fn new(cwd: String, state: S) -> Self {
        let now = Instant::now();
        Session {
            cwd,
            state,
            kind: String::new(),
            detail: String::new(),
            since: now,
            last_event_at: now,
            // checked: an Instant counts from boot on Windows, and `now - D`
            // panics in the first D seconds (a pet started at login would die)
            last_tool_at: now.checked_sub(THINK_LAG).unwrap_or(now),
            open_tools: Vec::new(),
            engaged: false,
        }
    }
    fn set(&mut self, state: S, kind: &str, detail: Option<String>) {
        if self.state != state || self.kind != kind {
            self.state = state;
            self.kind = kind.to_string();
            self.since = Instant::now();
        }
        if let Some(d) = detail {
            if !d.is_empty() {
                self.detail = truncate(&d);
            }
        }
        self.last_event_at = Instant::now();
    }
    fn touch(&mut self) {
        self.last_event_at = Instant::now();
    }
}

#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct SessionInfo {
    pub id: String,
    /// the project: the working folder's name
    pub project: String,
    pub cwd: String,
    /// needs_input, error, working, thinking, done, idle
    pub state: String,
    pub kind: String,
    pub detail: String,
    pub since_ms: u64,
    pub tools: Vec<String>,
}

#[derive(Clone, Serialize, PartialEq, Debug, Default)]
pub struct SessionsPayload {
    /// the most pressing session's state, or "none"
    pub state: String,
    pub detail: String,
    pub project: String,
    pub sessions: Vec<SessionInfo>,
    /// false when port 4317 couldn't be opened (another Claw'd has it)
    pub listening: bool,
    /// ~/.claude/settings.json sends hooks here
    pub hooks_installed: bool,
}

fn truncate(s: &str) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= DETAIL_MAX {
        return s;
    }
    let cut: String = s.chars().take(DETAIL_MAX - 1).collect();
    format!("{cut}\u{2026}")
}

fn tool_detail(ev: &HookEvent) -> Option<String> {
    let name = ev.tool_name.as_deref().unwrap_or("");
    let scrap = ev.tool_input.as_ref().and_then(|ti| {
        ti.get("description")
            .or_else(|| ti.get("command"))
            .or_else(|| ti.get("file_path"))
            .or_else(|| ti.get("url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    });
    match (name.is_empty(), scrap) {
        (true, None) => None,
        (true, Some(s)) => Some(s),
        (false, None) => Some(name.to_string()),
        (false, Some(s)) => Some(format!("{name}: {s}")),
    }
}

fn apply_event(reg: &mut HashMap<String, Session>, ev: &HookEvent) {
    let Some(name) = ev.hook_event_name.as_deref() else { return };
    let sid = match ev.session_id.as_deref() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return,
    };
    let cwd = ev.cwd.clone().unwrap_or_default();
    if name == "SessionEnd" {
        reg.remove(&sid);
        return;
    }
    let creates = matches!(
        name,
        "SessionStart" | "UserPromptSubmit" | "PreToolUse" | "PostToolUse" | "PostToolUseFailure"
            | "PermissionRequest" | "Notification" | "Elicitation" | "Stop" | "StopFailure"
    );
    if !creates {
        if let Some(s) = reg.get_mut(&sid) {
            s.touch();
        }
        return;
    }
    let is_new = !reg.contains_key(&sid);
    let s = reg
        .entry(sid)
        .or_insert_with(|| Session::new(cwd.clone(), if name == "SessionStart" { S::Idle } else { S::Working }));
    if !cwd.is_empty() {
        s.cwd = cwd;
    }
    if name != "SessionStart" {
        s.engaged = true;
    }
    match name {
        "SessionStart" => {
            if is_new {
                s.set(S::Idle, "", None);
            } else {
                s.touch();
            }
        }
        "UserPromptSubmit" => s.set(S::Working, "", ev.prompt.clone()),
        "PreToolUse" => {
            if let Some(id) = ev.tool_use_id.clone() {
                if !s.open_tools.iter().any(|(i, _)| *i == id) {
                    s.open_tools.push((id, ev.tool_name.clone().unwrap_or_default()));
                }
            }
            s.last_tool_at = Instant::now();
            s.set(S::Working, "", tool_detail(ev));
        }
        "PostToolUse" | "PostToolUseFailure" => {
            if let Some(id) = ev.tool_use_id.as_deref() {
                s.open_tools.retain(|(i, _)| i != id);
            }
            if name == "PostToolUseFailure" && ev.is_interrupt == Some(true) {
                s.open_tools.clear();
                s.set(S::Idle, "", None);
                return;
            }
            s.last_tool_at = Instant::now();
            match s.state {
                S::Working | S::NeedsInput => s.set(S::Working, "", tool_detail(ev)),
                // a straggler after Stop must not wedge Done back to Working
                S::Done | S::Idle | S::Error => s.touch(),
            }
        }
        "PermissionRequest" => s.set(S::NeedsInput, "permission", tool_detail(ev)),
        "Notification" => match ev.notification_type.as_deref() {
            Some("permission_prompt") => {
                let fill = if s.detail.is_empty() { ev.message.clone() } else { None };
                s.set(S::NeedsInput, "permission", fill);
            }
            Some(k @ ("idle_prompt" | "agent_needs_input" | "elicitation_dialog")) => {
                s.set(S::NeedsInput, k, ev.message.clone());
            }
            Some("agent_completed") => {
                if !matches!(s.state, S::NeedsInput | S::Error) {
                    s.set(S::Done, "", ev.message.clone());
                } else {
                    s.touch();
                }
            }
            // an unknown "Claude wants something" beats silently looking idle
            _ => s.set(S::NeedsInput, "unknown", ev.message.clone()),
        },
        "Elicitation" => {
            let detail = ev.message.clone().or_else(|| ev.prompt.clone());
            s.set(S::NeedsInput, "elicitation", detail);
        }
        "Stop" => {
            s.open_tools.clear();
            s.set(S::Done, "", ev.last_assistant_message.clone());
        }
        "StopFailure" => s.set(S::Error, "", ev.error.clone()),
        _ => {}
    }
}

fn decay(reg: &mut HashMap<String, Session>) {
    let now = Instant::now();
    for s in reg.values_mut() {
        match s.state {
            S::Done if now.duration_since(s.since) >= DONE_TO_IDLE => s.set(S::Idle, "", None),
            S::Error if now.duration_since(s.since) >= ERROR_TO_IDLE => s.set(S::Idle, "", None),
            S::Working if s.open_tools.is_empty() && now.duration_since(s.last_event_at) >= THINK_STALE => {
                s.set(S::Idle, "", None)
            }
            _ => {}
        }
    }
    reg.retain(|_, s| {
        let silent = now.duration_since(s.last_event_at);
        match s.state {
            S::NeedsInput => true,
            S::Working if !s.open_tools.is_empty() => silent < ORPHAN_DROP_OPEN_TOOLS,
            S::Idle if !s.engaged => silent < UNENGAGED_DROP,
            _ => silent < ORPHAN_DROP,
        }
    });
}

fn display_name(s: &Session, now: Instant) -> &'static str {
    if s.state == S::Working && s.open_tools.is_empty() && now.duration_since(s.last_tool_at) >= THINK_LAG {
        "thinking"
    } else {
        s.state.name()
    }
}

fn project_of(cwd: &str) -> String {
    cwd.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or("").to_string()
}

fn reduce(reg: &HashMap<String, Session>, listening: bool, hooks_installed: bool) -> SessionsPayload {
    let now = Instant::now();
    let mut sessions: Vec<SessionInfo> = reg
        .iter()
        .map(|(id, s)| SessionInfo {
            id: id.clone(),
            project: project_of(&s.cwd),
            cwd: s.cwd.clone(),
            state: display_name(s, now).into(),
            kind: s.kind.clone(),
            detail: s.detail.clone(),
            since_ms: now.duration_since(s.since).as_millis() as u64,
            tools: s.open_tools.iter().rev().map(|(_, n)| n.clone()).collect(),
        })
        .collect();
    sessions.sort_by(|a, b| a.id.cmp(&b.id));
    let top = reg.values().max_by(|a, b| {
        a.state.rank().cmp(&b.state.rank()).then_with(|| {
            if a.state == S::NeedsInput {
                now.duration_since(a.since).cmp(&now.duration_since(b.since))
            } else {
                a.last_event_at.cmp(&b.last_event_at)
            }
        })
    });
    let (state, detail, project) = match top {
        None => ("none".to_string(), String::new(), String::new()),
        Some(s) => (display_name(s, now).to_string(), s.detail.clone(), project_of(&s.cwd)),
    };
    SessionsPayload { state, detail, project, sessions, listening, hooks_installed }
}

// the emit-on-change comparison ignores the ages, or the 1 s tick would
// re-send the same picture every second
fn same(a: &SessionsPayload, b: &SessionsPayload) -> bool {
    let key = |p: &SessionsPayload| {
        (
            p.state.clone(),
            p.detail.clone(),
            p.listening,
            p.hooks_installed,
            p.sessions.iter().map(|s| (s.id.clone(), s.state.clone(), s.detail.clone(), s.tools.clone())).collect::<Vec<_>>(),
        )
    };
    key(a) == key(b)
}

// --- the hooks in ~/.claude/settings.json --------------------------------

const EVENTS: &[&str] = &[
    "SessionStart", "SessionEnd", "UserPromptSubmit", "PreToolUse", "PostToolUse", "PostToolUseFailure",
    "PermissionRequest", "Elicitation", "Notification", "Stop", "StopFailure",
];

fn settings_path() -> std::path::PathBuf {
    crate::platform::home_dir().join(".claude").join("settings.json")
}

/// Do Claude Code's hooks point here? (The coding Claw'd's install counts:
/// same port.)
pub fn hooks_installed() -> bool {
    std::fs::read_to_string(settings_path()).map_or(false, |s| s.contains(HOOK_ADDR))
}

fn ours(entry: &serde_json::Value) -> bool {
    entry.to_string().contains(HOOK_ADDR)
}

/// Add his hook to every event, keeping whatever hooks are already there. The
/// first time, the original is kept as settings.json.clawd-backup.
pub fn install_hooks() -> Result<(), String> {
    let path = settings_path();
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|_| "{}".into());
    let mut cfg: serde_json::Value =
        serde_json::from_str(raw.trim_start_matches('\u{feff}')).map_err(|e| format!("settings.json isn't valid JSON: {e}"))?;
    let backup = path.with_extension("json.clawd-backup");
    if !backup.exists() && path.exists() {
        std::fs::write(&backup, &raw).map_err(|e| e.to_string())?;
    }
    let obj = cfg.as_object_mut().ok_or("settings.json isn't an object")?;
    let hooks = obj.entry("hooks").or_insert_with(|| serde_json::json!({}));
    let hooks = hooks.as_object_mut().ok_or("settings.json's hooks isn't an object")?;
    for ev in EVENTS {
        let list = hooks.entry(ev.to_string()).or_insert_with(|| serde_json::json!([]));
        let Some(arr) = list.as_array_mut() else { continue };
        if !arr.iter().any(ours) {
            arr.push(serde_json::json!({ "hooks": [{ "type": "http", "url": HOOK_URL, "timeout": 2 }] }));
        }
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&path, body).map_err(|e| e.to_string())
}

/// Take his hooks back out (and only his).
pub fn remove_hooks() -> Result<(), String> {
    let path = settings_path();
    let Ok(raw) = std::fs::read_to_string(&path) else { return Ok(()) };
    let mut cfg: serde_json::Value = serde_json::from_str(raw.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
    if let Some(hooks) = cfg.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        for list in hooks.values_mut() {
            if let Some(arr) = list.as_array_mut() {
                arr.retain(|e| !ours(e));
            }
        }
        hooks.retain(|_, v| v.as_array().map_or(true, |a| !a.is_empty()));
    }
    if cfg.get("hooks").and_then(|h| h.as_object()).map_or(false, |h| h.is_empty()) {
        if let Some(o) = cfg.as_object_mut() {
            o.remove("hooks");
        }
    }
    let body = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&path, body).map_err(|e| e.to_string())
}

// --- the server and the reducer thread -----------------------------------

pub struct SessionsStore(pub std::sync::Mutex<SessionsPayload>);

static STARTED: AtomicBool = AtomicBool::new(false);

/// Start listening (once). Called at startup when the dev pack is on, and
/// when it's switched on.
pub fn ensure_started(handle: &AppHandle) {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let (tx, rx) = mpsc::channel::<HookEvent>();
    let listening = spawn_server(tx).is_ok();
    let handle = handle.clone();
    std::thread::spawn(move || {
        let mut reg: HashMap<String, Session> = HashMap::new();
        let mut last_hooks_check = Instant::now();
        let mut hooks = hooks_installed();
        publish(&handle, reduce(&reg, listening, hooks));
        loop {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(ev) => {
                    apply_event(&mut reg, &ev);
                    while let Ok(ev) = rx.try_recv() {
                        apply_event(&mut reg, &ev);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {}
            }
            decay(&mut reg);
            if last_hooks_check.elapsed() > Duration::from_secs(10) {
                hooks = hooks_installed();
                last_hooks_check = Instant::now();
            }
            publish(&handle, reduce(&reg, listening, hooks));
        }
    });
}

fn publish(handle: &AppHandle, payload: SessionsPayload) {
    let store = handle.state::<SessionsStore>();
    let mut cur = store.0.lock().unwrap_or_else(|e| e.into_inner());
    if same(&cur, &payload) {
        return;
    }
    *cur = payload.clone();
    drop(cur);
    let _ = handle.emit_to("pet", "sessions", payload);
}

fn spawn_server(tx: Sender<HookEvent>) -> Result<(), String> {
    let mut last_err = String::new();
    let mut server = None;
    for attempt in 0..3 {
        match tiny_http::Server::http(HOOK_ADDR) {
            Ok(s) => {
                server = Some(s);
                break;
            }
            Err(e) => {
                last_err = e.to_string();
                if attempt < 2 {
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }
    }
    let server = server.ok_or(last_err)?;
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let tx = tx.clone();
            // one thread per request: a client that stalls mid-body must not
            // park the accept loop and silence every session
            let _ = std::thread::Builder::new().name("hook-serve".into()).spawn(move || serve(request, &tx));
        }
    });
    Ok(())
}

/// Read (capped), forward, then answer 200 {}. Forwarding first keeps each
/// session's events in order: Claude Code sends the next hook only after
/// this answer.
fn serve(mut request: tiny_http::Request, tx: &Sender<HookEvent>) {
    let is_event = *request.method() == tiny_http::Method::Post && request.url().starts_with("/event");
    let mut body = String::new();
    let read_ok = request.as_reader().take(MAX_BODY).read_to_string(&mut body).is_ok();
    if is_event && read_ok {
        if let Ok(ev) = serde_json::from_str::<HookEvent>(&body) {
            let _ = tx.send(ev);
        }
    }
    let response = tiny_http::Response::from_string("{}").with_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).expect("static header"),
    );
    let _ = request.respond(response);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(name: &str, sid: &str) -> HookEvent {
        HookEvent {
            hook_event_name: Some(name.into()),
            session_id: Some(sid.into()),
            cwd: Some("C:\\code\\my-game".into()),
            tool_name: None,
            tool_input: None,
            tool_use_id: None,
            prompt: None,
            message: None,
            notification_type: None,
            error: None,
            is_interrupt: None,
            last_assistant_message: None,
        }
    }

    #[test]
    fn a_turn_goes_working_asks_then_done() {
        let mut reg = HashMap::new();
        apply_event(&mut reg, &ev("UserPromptSubmit", "a"));
        let mut pre = ev("PreToolUse", "a");
        pre.tool_name = Some("Bash".into());
        pre.tool_use_id = Some("t1".into());
        pre.tool_input = Some(serde_json::json!({ "command": "cargo test" }));
        apply_event(&mut reg, &pre);
        let p = reduce(&reg, true, true);
        assert_eq!(p.state, "working");
        assert_eq!(p.project, "my-game");
        assert_eq!(p.sessions[0].detail, "Bash: cargo test");
        apply_event(&mut reg, &ev("PermissionRequest", "a"));
        assert_eq!(reduce(&reg, true, true).state, "needs_input");
        apply_event(&mut reg, &ev("Stop", "a"));
        assert_eq!(reduce(&reg, true, true).state, "done");
        apply_event(&mut reg, &ev("SessionEnd", "a"));
        assert_eq!(reduce(&reg, true, true).state, "none");
    }

    #[test]
    fn the_worst_session_wins_and_strays_never_create_sessions() {
        let mut reg = HashMap::new();
        apply_event(&mut reg, &ev("UserPromptSubmit", "a"));
        let mut n = ev("Notification", "b");
        n.notification_type = Some("idle_prompt".into());
        apply_event(&mut reg, &n);
        assert_eq!(reduce(&reg, true, true).state, "needs_input");
        apply_event(&mut reg, &ev("SessionEnd", "zzz"));
        apply_event(&mut reg, &ev("SomethingNew", "yyy"));
        assert_eq!(reg.len(), 2);
    }
}
