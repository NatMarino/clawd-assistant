// Claw'd the assistant, native shell: transparent always-on-top pet window,
// click-through hit-test poller, drag/resize commands, position/scale
// persistence, and the inbox pipeline (feed.rs folder + HTTP -> state.rs
// store -> "pet-state" events into the webview). The window code is the
// coding Claw'd's, unchanged; the pipeline is new.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod brain;
mod feed;
mod platform;
mod state;
mod taskbar;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tauri::{LogicalSize, Manager, PhysicalPosition, WindowEvent};

/// Poison-tolerant locking. A panic while a lock is held must not take the
/// poller or saver thread down with it on their next `unwrap`: a dead poller
/// freezes click-through in whatever state it was last in, silently.
pub(crate) trait PoisonTolerant<T> {
    fn lock_or_recover(&self) -> std::sync::MutexGuard<'_, T>;
}

impl<T> PoisonTolerant<T> for Mutex<T> {
    fn lock_or_recover(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

const BASE_SIZE: f64 = 160.0; // logical px at scale 1.0
// Extra window size beyond the canvas: WebView2's viewport under-reports the
// window by ~12px on the right/top, clipping canvas-edge art (dream cloud,
// thinking dots). With padding, the clipped strip falls on empty transparent
// space and the full canvas always renders. The frontend reports hit-test
// bounds page-relative, so click-through stays correct.
const PAD: f64 = 24.0;
// Standing headroom above the canvas for the session popover: the window is
// always this much taller than the sprite area, so opening the popover never
// resizes or moves anything — it just renders into invisible space. The
// click-through poller keeps the empty headroom non-interactive.
const POP_HEADROOM: f64 = 360.0;
// The window is never narrower than this. Empirical (user-eyeballed): the
// on-screen composite clips MORE of the right edge than the reported
// viewport suggests, and neither PrintWindow nor GDI captures can see that
// clip — so the popover gets a wide berth instead of a tight calculation.
const POP_MIN_W: f64 = 400.0;
// Standing room BELOW the canvas for the speech bubble (the frontend anchors
// #pet-wrap this far above the window bottom in Tauri; keep in sync with the
// CSS). Sized for the large bubble: 14px text, title + 3 wrapped detail lines.
const BUBBLE_ROOM: f64 = 110.0;

fn window_size(scale: f64) -> LogicalSize<f64> {
    LogicalSize::new(
        (BASE_SIZE * scale + PAD).max(POP_MIN_W),
        BASE_SIZE * scale + PAD + POP_HEADROOM + BUBBLE_ROOM,
    )
}

/// Clickable region reported by the frontend in PHYSICAL pixels,
/// window-relative (the frontend multiplies by devicePixelRatio, which is
/// the exact CSS->physical factor including webview zoom — the monitor
/// scale factor alone is wrong whenever zoom != 1).
#[derive(Default, Clone, Copy)]
struct OpaqueBounds {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Serialize, Deserialize, Clone)]
struct PetConfig {
    /// window position, physical px (what outer_position reports)
    x: Option<i32>,
    y: Option<i32>,
    scale: f64,
    /// where Claude's items come from (absent = defaults)
    #[serde(default)]
    feed: feed::FeedConfig,
    /// living on the taskbar (feet on its top edge) rather than wherever he
    /// was dropped. On by default; dropping him away from the taskbar turns
    /// it off, dropping him near it (or the tray's "Back to the taskbar")
    /// turns it back on.
    #[serde(default = "yes")]
    on_taskbar: bool,
    /// start at login, hidden, and come out when Claude opens (the app, or
    /// Claude Code). On by default; the gear switches it off.
    #[serde(default = "yes")]
    start_with_claude: bool,
}

fn yes() -> bool {
    true
}

impl Default for PetConfig {
    fn default() -> Self {
        Self { x: None, y: None, scale: 1.75, feed: Default::default(), on_taskbar: true, start_with_claude: true }
    }
}

/// %APPDATA%\ClawdAssistant: its own folder, so this pet and the coding
/// Claw'd can run side by side without sharing position or state.
pub(crate) fn app_data_dir() -> Option<PathBuf> {
    platform::app_data_dir()
}

fn config_path() -> Option<PathBuf> {
    app_data_dir().map(|d| d.join("config.json"))
}

fn load_config() -> PetConfig {
    // a byte-order mark (Notepad, PowerShell 5) must not cost the whole
    // config: an unreadable file silently falls back to every default
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

fn save_config(cfg: &PetConfig) {
    if let Some(p) = config_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(cfg) {
            let _ = std::fs::write(p, json);
        }
    }
}

/// An in-flight app-driven drag: the cursor's offset from the window origin
/// (physical px), captured when the grab started. The poller thread moves
/// the window with set_position while this is Some.
#[derive(Clone, Copy)]
struct DragGrab {
    dx: f64,
    dy: f64,
}

/// A walk in progress: the window's x slides from `from` to `to` (physical
/// px) over `ms`, driven by the poller thread so it stays smooth whatever the
/// webview is doing.
#[derive(Clone, Copy)]
struct Walk {
    from: f64,
    to: f64,
    start: std::time::Instant,
    ms: f64,
}

struct AppState {
    bounds: Mutex<OpaqueBounds>,
    cfg: Mutex<PetConfig>,
    dirty: AtomicBool,
    drag: Mutex<Option<DragGrab>>,
    walk: Mutex<Option<Walk>>,
    /// where his centre and feet really are, measured by the page:
    /// (pet scale, x, y) as physical offsets from the window's top-left
    anchor: Mutex<Option<(f64, f64, f64)>>,
    brain: Arc<brain::Brain>,
    /// started at login and still hidden, waiting for Claude to open
    waiting: AtomicBool,
}

// --- living on the taskbar ---------------------------------------------
//
// Every sprite stands with its feet on canvas unit 140 (both skins), the
// canvas is 160 units tall, and its bottom sits BUBBLE_ROOM above the window
// bottom. So the feet are a fixed distance from the window's top edge, and
// putting him on the taskbar is one set_position.
const FEET_UNIT: f64 = 140.0;
// a drop this close to the taskbar line (logical px) counts as "on it"
const SNAP_PX: f64 = 90.0;

fn feet_from_top(pet_scale: f64, sf: f64) -> f64 {
    (window_size(pet_scale).height - BUBBLE_ROOM - (BASE_SIZE - FEET_UNIT) * pet_scale) * sf
}

fn half_width(pet_scale: f64, sf: f64) -> f64 {
    window_size(pet_scale).width * sf / 2.0
}

/// His centre x and feet y, as offsets from the window's top-left
/// (physical px). The page measures them (report_anchor) because the web
/// view does not sit exactly where the window arithmetic says it should;
/// until it has, the arithmetic stands in.
fn anchor(state: &AppState, pet_scale: f64, sf: f64) -> (f64, f64) {
    match *state.anchor.lock_or_recover() {
        Some((s, x, y)) if (s - pet_scale).abs() < 1e-6 => (x, y),
        _ => (half_width(pet_scale, sf), feet_from_top(pet_scale, sf)),
    }
}

/// Where along the taskbar his centre may go: clear of both ends, and never
/// over the tray icons and clock.
fn taskbar_range(tb: &taskbar::Taskbar, pet_scale: f64, sf: f64) -> (f64, f64, f64) {
    let body = 64.0 * pet_scale * sf; // half his body plus the arm nubs
    let min = tb.left as f64 + body;
    let max = (tb.tray_left as f64 - body).max(min);
    let home = (tb.tray_left as f64 - body - 12.0 * sf).clamp(min, max);
    (min, max, home)
}

/// His centre and feet on screen right now (physical px).
fn buddy_point(win: &tauri::WebviewWindow, state: &AppState) -> Option<(f64, f64)> {
    let pos = win.outer_position().ok()?;
    let sf = win.scale_factor().unwrap_or(1.0);
    let s = state.cfg.lock_or_recover().scale.clamp(0.5, 3.0);
    let (ax, ay) = anchor(state, s, sf);
    Some((pos.x as f64 + ax, pos.y as f64 + ay))
}

/// The ground of the display he's on (or of the point he's being put at).
fn ground(win: &tauri::WebviewWindow, state: &AppState, at: Option<(f64, f64)>) -> Option<taskbar::Taskbar> {
    let (x, y) = at.or_else(|| buddy_point(win, state))?;
    taskbar::query_at(win, x, y)
}

/// Stand him on the taskbar of the display he's on, with his centre at
/// `center_x` (clamped to the walkable range), or at that display's home
/// when None.
fn place_on_taskbar(win: &tauri::WebviewWindow, state: &AppState, center_x: Option<f64>) -> bool {
    let here = buddy_point(win, state);
    let at = match (center_x, here) {
        (Some(cx), Some((_, fy))) => Some((cx, fy)),
        _ => here,
    };
    let Some(tb) = ground(win, state, at) else { return false };
    let sf = win.scale_factor().unwrap_or(1.0);
    let s = state.cfg.lock_or_recover().scale.clamp(0.5, 3.0);
    let (min, max, home) = taskbar_range(&tb, s, sf);
    let cx = center_x.unwrap_or(home).clamp(min, max);
    let (ax, ay) = anchor(state, s, sf);
    let x = (cx - ax).round() as i32;
    let y = (tb.top as f64 - ay).round() as i32;
    let _ = win.set_position(PhysicalPosition::new(x, y));
    let mut cfg = state.cfg.lock_or_recover();
    cfg.on_taskbar = true;
    state.dirty.store(true, Ordering::Relaxed);
    true
}

#[derive(Serialize)]
struct TaskbarInfo {
    /// he is living on the taskbar right now
    on: bool,
    /// his centre and the walkable range, physical px
    center_x: f64,
    min_x: f64,
    max_x: f64,
    home_x: f64,
    /// physical px per CSS px, for turning distances into walking time
    sf: f64,
    /// which display he's on: its index, name, and whether it's the main one
    display: usize,
    display_name: String,
    primary: bool,
}

#[tauri::command]
fn taskbar_info(window: tauri::WebviewWindow, state: tauri::State<AppState>) -> Option<TaskbarInfo> {
    let tb = ground(&window, &state, None)?;
    let sf = window.scale_factor().unwrap_or(1.0);
    let (s, on) = {
        let cfg = state.cfg.lock_or_recover();
        (cfg.scale.clamp(0.5, 3.0), cfg.on_taskbar)
    };
    let pos = window.outer_position().ok()?;
    let (min_x, max_x, home_x) = taskbar_range(&tb, s, sf);
    Some(TaskbarInfo {
        on,
        center_x: pos.x as f64 + anchor(&state, s, sf).0,
        min_x,
        max_x,
        home_x,
        sf,
        display: tb.display,
        display_name: tb.display_name,
        primary: tb.primary,
    })
}

/// Put him (back) on the taskbar: at `center_x`, or at home.
#[tauri::command]
fn go_to_taskbar(window: tauri::WebviewWindow, state: tauri::State<AppState>, center_x: Option<f64>) -> bool {
    *state.walk.lock_or_recover() = None;
    place_on_taskbar(&window, &state, center_x)
}

/// Start walking to `center_x` at `speed` physical px per second. Returns
/// how long it will take (ms); the poller emits "walk-done" on arrival.
#[tauri::command]
fn walk_to(window: tauri::WebviewWindow, state: tauri::State<AppState>, center_x: f64, speed: f64) -> f64 {
    let Some(tb) = ground(&window, &state, None) else { return 0.0 };
    let Ok(pos) = window.outer_position() else { return 0.0 };
    let sf = window.scale_factor().unwrap_or(1.0);
    let s = state.cfg.lock_or_recover().scale.clamp(0.5, 3.0);
    let (min, max, _) = taskbar_range(&tb, s, sf);
    let to = center_x.clamp(min, max) - anchor(&state, s, sf).0;
    let from = pos.x as f64;
    let ms = ((to - from).abs() / speed.max(10.0) * 1000.0).max(1.0);
    *state.walk.lock_or_recover() = Some(Walk { from, to, start: std::time::Instant::now(), ms });
    ms
}

/// The page's measurement of his centre and feet, in physical px from the
/// web view's top-left, at pet scale `scale`. Stored as offsets from the
/// window's top-left; on the taskbar he is re-stood straight away, around
/// the same centre, so the correction is invisible.
#[tauri::command]
fn report_anchor(window: tauri::WebviewWindow, state: tauri::State<AppState>, x: f64, y: f64, scale: f64) {
    let (Ok(outer), Ok(inner)) = (window.outer_position(), window.inner_position()) else { return };
    let sf = window.scale_factor().unwrap_or(1.0);
    let before = anchor(&state, scale, sf).0;
    let ax = (inner.x - outer.x) as f64 + x;
    let ay = (inner.y - outer.y) as f64 + y;
    *state.anchor.lock_or_recover() = Some((scale, ax, ay));
    let idle = state.walk.lock_or_recover().is_none() && state.drag.lock_or_recover().is_none();
    if idle && state.cfg.lock_or_recover().on_taskbar {
        let _ = place_on_taskbar(&window, &state, Some(outer.x as f64 + before));
    }
}

/// What he learned when he got to know you (the interview after hatching),
/// written to the Clawd folder as intro.json for Claude to turn into
/// prefs.json. Also where he remembers your name across reinstalls.
#[tauri::command]
fn save_intro(state: tauri::State<AppState>, json: String) -> bool {
    let dir = feed::clawd_dir(&state.cfg.lock_or_recover().feed);
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) else { return false };
    let pretty = serde_json::to_string_pretty(&v).unwrap_or(json);
    let tmp = dir.join("intro.json.tmp");
    std::fs::write(&tmp, pretty).is_ok() && std::fs::rename(&tmp, dir.join("intro.json")).is_ok()
}

#[tauri::command]
fn load_intro(state: tauri::State<AppState>) -> Option<serde_json::Value> {
    let dir = feed::clawd_dir(&state.cfg.lock_or_recover().feed);
    let text = std::fs::read_to_string(dir.join("intro.json")).ok()?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

// --- the big brain (brain.rs) -------------------------------------------

fn clawd_dir(state: &AppState) -> PathBuf {
    feed::clawd_dir(&state.cfg.lock_or_recover().feed)
}

/// Is there a big brain he can use, and which apps can it reach? A tiny real
/// run on the smallest model; takes a few seconds.
#[tauri::command]
async fn brain_status(state: tauri::State<'_, AppState>, model: String) -> Result<brain::Status, String> {
    let (b, dir) = (state.brain.clone(), clawd_dir(&state));
    tauri::async_runtime::spawn_blocking(move || brain::probe(&b, &dir, &model, 90))
        .await
        .map_err(|e| e.to_string())
}

/// Start a run. The skill is always its instructions, plus `extra` for the
/// job at hand; its lines arrive as "brain" events tagged `job`.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn brain_run(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    job: String,
    prompt: String,
    extra: String,
    model: String,
    mode: String,
    resume: Option<String>,
    also_allow: Vec<String>,
    max_turns: Option<u32>,
) {
    let system = if extra.trim().is_empty() { feed::SKILL.to_string() } else { format!("{}

{}", feed::SKILL, extra) };
    let spec = brain::RunSpec { job, prompt, system, model, mode, resume, also_allow, max_turns: max_turns.unwrap_or(30) };
    brain::spawn_run(app, state.brain.clone(), clawd_dir(&state), spec);
}

#[tauri::command]
fn brain_stop(state: tauri::State<AppState>) {
    brain::stop(&state.brain);
}

/// The one-time things only a person can do, in the big brain's own window:
/// "login", or "apps" (say yes to Gmail, Slack… for it).
#[tauri::command]
fn brain_open(state: tauri::State<AppState>, what: String) -> bool {
    let first = if what == "login" { "/login" } else { "/mcp" };
    brain::open_window(&clawd_dir(&state), first)
}

/// prefs.json from the Clawd folder (work hours, digest time…), or null.
#[tauri::command]
fn read_prefs(state: tauri::State<AppState>) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(clawd_dir(&state).join("prefs.json")).ok()?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

/// Items the pet makes itself (a failed run's "can you help me?"), through
/// the same store as everything Claude sends.
#[tauri::command]
fn add_local_items(tx: tauri::State<state::EventSender>, json: String) -> bool {
    match feed::parse_batch(&json) {
        Ok(items) => tx.0.lock_or_recover().send(state::PetEvent::Items(items, "local")).is_ok(),
        Err(_) => false,
    }
}

/// Out he comes: Claude just opened (or the tray called him) while he was
/// waiting hidden since login. Does nothing if he's already out.
fn reveal(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    if !state.waiting.swap(false, Ordering::Relaxed) {
        return;
    }
    if let Some(win) = app.get_webview_window("pet") {
        let _ = win.show();
    }
    use tauri::Emitter;
    let _ = app.emit_to("pet", "claude-opened", ());
}

/// Is he hidden, waiting for Claude? (The page stays quiet until he's out.)
#[tauri::command]
fn waiting_for_claude(state: tauri::State<AppState>) -> bool {
    state.waiting.load(Ordering::Relaxed)
}

#[tauri::command]
fn get_start_with_claude(state: tauri::State<AppState>) -> bool {
    state.cfg.lock_or_recover().start_with_claude
}

/// The gear's switch: start at login and come out with Claude, or not.
#[tauri::command]
fn set_start_with_claude(state: tauri::State<AppState>, on: bool) -> bool {
    state.cfg.lock_or_recover().start_with_claude = on;
    state.dirty.store(true, Ordering::Relaxed);
    platform::set_login_item(on)
}

/// A line from the page for page.log (errors, and what was clicked).
#[tauri::command]
fn page_log(line: String) {
    brain::log_to("page.log", &line.chars().take(1000).collect::<String>());
}

#[tauri::command]
fn stop_walk(state: tauri::State<AppState>) {
    *state.walk.lock_or_recover() = None;
}

fn left_button_down() -> bool {
    platform::left_button_down()
}

/// Screen position (physical px) of the buddy's center for a window whose
/// top-left is at (x, y): the sprite is CSS-centered horizontally and its
/// wrap sits BUBBLE_ROOM above the window bottom (index.html, body.tauri
/// #pet-wrap). `sf` is the monitor scale factor (1.0 on 100% displays).
fn buddy_center(x: f64, y: f64, pet_scale: f64, sf: f64) -> (f64, f64) {
    let size = window_size(pet_scale);
    (
        x + size.width * sf / 2.0,
        y + (size.height - BUBBLE_ROOM - BASE_SIZE * pet_scale / 2.0) * sf,
    )
}

/// The OS hit region before the webview's first report: the sprite's rest
/// rect plus its motion envelope and the halo, mirroring reportBounds in
/// index.html (pet.js: rest x32 y68 w96 h80 units, envelope l28 r24 u48 d4,
/// wrap 160 units centered and BUBBLE_ROOM above the window bottom). The
/// buddy is clickable from the first frame — and stays so if the frontend
/// ever fails to load — instead of an invisible 160x160 square at the
/// window's top-left.
fn boot_bounds(pet_scale: f64, sf: f64) -> OpaqueBounds {
    const HALO: f64 = 20.0;
    let size = window_size(pet_scale);
    let wrap_l = size.width / 2.0 - 80.0 * pet_scale;
    let wrap_t = size.height - BUBBLE_ROOM - BASE_SIZE * pet_scale;
    OpaqueBounds {
        x: (wrap_l + 4.0 * pet_scale - HALO) * sf,
        y: (wrap_t + 20.0 * pet_scale - HALO) * sf,
        w: (148.0 * pet_scale + 2.0 * HALO) * sf,
        h: (132.0 * pet_scale + 2.0 * HALO) * sf,
    }
}

fn on_some_monitor(monitors: &[tauri::Monitor], px: f64, py: f64) -> bool {
    monitors.iter().any(|m| {
        let (mp, ms) = (m.position(), m.size());
        px >= mp.x as f64
            && px < mp.x as f64 + ms.width as f64
            && py >= mp.y as f64
            && py < mp.y as f64 + ms.height as f64
    })
}

/// Keep the buddy visible while dragging: if the requested window origin
/// would put the buddy's center off every monitor, pull it back onto the
/// nearest one. The invisible window padding may hang off-screen freely.
fn clamp_to_monitors(monitors: &[tauri::Monitor], x: f64, y: f64, pet_scale: f64, sf: f64) -> (f64, f64) {
    let (cx, cy) = buddy_center(x, y, pet_scale, sf);
    if on_some_monitor(monitors, cx, cy) {
        return (x, y);
    }
    let mut best: Option<(f64, f64, f64)> = None; // (dist², clamped cx, clamped cy)
    for m in monitors {
        let (mp, ms) = (m.position(), m.size());
        let (l, t) = (mp.x as f64, mp.y as f64);
        let (r, b) = (l + ms.width as f64 - 1.0, t + ms.height as f64 - 1.0);
        let (qx, qy) = (cx.clamp(l, r.max(l)), cy.clamp(t, b.max(t)));
        let d2 = (qx - cx).powi(2) + (qy - cy).powi(2);
        if best.map_or(true, |(bd, _, _)| d2 < bd) {
            best = Some((d2, qx, qy));
        }
    }
    match best {
        Some((_, qx, qy)) => (x + (qx - cx), y + (qy - cy)),
        None => (x, y),
    }
}

#[tauri::command]
fn set_opaque_bounds(state: tauri::State<AppState>, x: f64, y: f64, w: f64, h: f64) {
    *state.bounds.lock_or_recover() = OpaqueBounds { x, y, w, h };
}

/// Begin an app-driven drag. The OS move loop (`start_dragging`) is
/// deliberately NOT used: tao gives even this undecorated window WS_CAPTION,
/// so Windows applied its title-bar rules to the 360px of invisible popover
/// headroom — on mouse-up it popped the window down so that empty air sat
/// below the screen top, and the buddy landed somewhere other than where it
/// was released. Moving the window ourselves means no clamp, no Snap, no
/// monitor re-evaluation, and no capture theft (JS keeps its pointerup).
///
/// `x`/`y`: the press point in physical screen px (the webview's screenX/Y
/// times devicePixelRatio). Anchoring on the press rather than on the cursor
/// at IPC time means the movement that happened while this call was in
/// flight is not lost: the buddy stays under the grabbed pixel.
#[tauri::command]
fn start_drag(window: tauri::WebviewWindow, state: tauri::State<AppState>, x: f64, y: f64) {
    // a flick can release the button before this IPC lands; a grab started
    // with the button up would glue the window to the cursor
    if !left_button_down() {
        return;
    }
    let Ok(pos) = window.outer_position() else {
        return;
    };
    *state.drag.lock_or_recover() = Some(DragGrab {
        dx: x - pos.x as f64,
        dy: y - pos.y as f64,
    });
}

/// The drag is over. Dropped with his feet near the taskbar line he settles
/// onto it and lives there; dropped anywhere else he stays put. Returns
/// whether he is on the taskbar now.
#[tauri::command]
fn end_drag(window: tauri::WebviewWindow, state: tauri::State<AppState>) -> bool {
    *state.drag.lock_or_recover() = None;
    let (Some(tb), Ok(pos)) = (ground(&window, &state, None), window.outer_position()) else {
        state.cfg.lock_or_recover().on_taskbar = false;
        return false;
    };
    let sf = window.scale_factor().unwrap_or(1.0);
    let s = state.cfg.lock_or_recover().scale.clamp(0.5, 3.0);
    let (ax, ay) = anchor(&state, s, sf);
    let feet = pos.y as f64 + ay;
    let cx = pos.x as f64 + ax;
    let near = (feet - tb.top as f64).abs() <= SNAP_PX * sf
        && cx >= tb.left as f64
        && cx <= tb.right as f64;
    if near {
        return place_on_taskbar(&window, &state, Some(cx));
    }
    state.cfg.lock_or_recover().on_taskbar = false;
    state.dirty.store(true, Ordering::Relaxed);
    false
}

#[tauri::command]
fn set_pet_scale(window: tauri::WebviewWindow, state: tauri::State<AppState>, scale: f64) {
    let s = scale.clamp(0.5, 3.0);
    // The key that got us here arrived through the pet, so it is the
    // foreground window. Test exactly that (not tao's is_focused flag, which
    // reads false while the WebView2 child holds keyboard focus): a future
    // caller running while another app is active must not touch focus at
    // all — focusing a child of an inactive window would activate it.
    let foreground = platform::is_foreground(&window);
    // his centre before the resize: set_size keeps the top-left corner, so
    // on the taskbar he is re-stood around this point afterwards
    let sf = window.scale_factor().unwrap_or(1.0);
    let old_s = state.cfg.lock_or_recover().scale.clamp(0.5, 3.0);
    let center = window.outer_position().map(|p| p.x as f64 + anchor(&state, old_s, sf).0).ok();
    let _ = window.set_size(window_size(s));
    // resizing can drop WebView2's keyboard focus; re-focus the WEBVIEW
    // (ICoreWebView2Controller::MoveFocus) so consecutive +/- presses keep
    // working. Window::set_focus cannot do this: tao returns early when the
    // window is already foreground, and it never reaches the child anyway.
    if foreground {
        let _ = AsRef::<tauri::Webview>::as_ref(&window).set_focus();
    }
    let on_taskbar = {
        let mut cfg = state.cfg.lock_or_recover();
        cfg.scale = s;
        cfg.on_taskbar
    };
    state.dirty.store(true, Ordering::Relaxed);
    if on_taskbar {
        let _ = place_on_taskbar(&window, &state, center);
    }
}

#[tauri::command]
fn get_pet_scale(state: tauri::State<AppState>) -> f64 {
    state.cfg.lock_or_recover().scale
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Hand a URL to Windows (default browser, or the Claude app for claude://).
/// ShellExecuteW rather than `cmd /C start`: cmd would treat the "&" in any
/// URL with two query parameters as a command separator.
fn shell_open(url: &str) -> bool {
    platform::open_url(url)
}

/// Where item links may lead. Items are written by Claude from content it
/// read in Slack and email, so a link is only as trustworthy as whoever sent
/// that message; the pet opens the tools people actually work in and
/// nothing else. A suffix entry matches the host and its subdomains.
const LINK_HOSTS: &[&str] = &[
    "slack.com",
    "mail.google.com",
    "calendar.google.com",
    "docs.google.com",
    "drive.google.com",
    "meet.google.com",
    "asana.com",
    "claude.ai",
    "zoom.us",
    "teams.microsoft.com",
    "outlook.office.com",
];

fn host_allowed(host: &str, extra: &[String]) -> bool {
    let host = host.to_ascii_lowercase();
    LINK_HOSTS
        .iter()
        .copied()
        .chain(extra.iter().map(|s| s.as_str()))
        .map(|h| h.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|h| !h.is_empty())
        .any(|h| host == h || host.ends_with(&format!(".{h}")))
}

fn link_allowed(url: &str, extra: &[String]) -> bool {
    if url.len() > 4096 || url.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"') {
        return false;
    }
    if url.starts_with("claude://") {
        return true;
    }
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return false; // https://slack.com@evil.example
    }
    let host = authority.split(':').next().unwrap_or("");
    host_allowed(host, extra)
}

#[tauri::command]
fn open_link(state: tauri::State<AppState>, url: String) -> bool {
    let extra = state.cfg.lock_or_recover().feed.extra_link_hosts.clone();
    if !link_allowed(&url, &extra) {
        eprintln!("clawd: refused link: {url}");
        return false;
    }
    shell_open(&url)
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Open a new Claude chat with `prompt` typed in. claude.ai/new?q= prefills
/// the composer on the web; the desktop app routes claude://claude.ai/<path>
/// to the same web routes (that is how the coding Claw'd's Cowork links
/// work). Whether ?q= survives that hop is the Phase 0 question, so the
/// frontend also puts the prompt on the clipboard.
#[tauri::command]
fn ask_claude(prompt: String) -> bool {
    let prompt: String = prompt.chars().take(4000).collect();
    shell_open(&format!("claude://claude.ai/new?q={}", percent_encode(&prompt)))
}

fn main() {
    let mut cfg = load_config();
    if cfg.feed.http_token.is_empty() {
        cfg.feed.http_token = feed::new_token();
        save_config(&cfg);
    }

    tauri::Builder::default()
        .manage(AppState {
            bounds: Mutex::new(boot_bounds(cfg.scale.clamp(0.5, 3.0), 1.0)),
            cfg: Mutex::new(cfg),
            dirty: AtomicBool::new(false),
            drag: Mutex::new(None),
            walk: Mutex::new(None),
            anchor: Mutex::new(None),
            brain: Arc::new(brain::Brain::default()),
            waiting: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            set_opaque_bounds, start_drag, end_drag, set_pet_scale, get_pet_scale, quit_app,
            open_link, ask_claude, taskbar_info, go_to_taskbar, walk_to, stop_walk, report_anchor, save_intro, load_intro,
            brain_status, brain_run, brain_stop, brain_open, read_prefs, add_local_items, page_log,
            waiting_for_claude, get_start_with_claude, set_start_with_claude,
            state::get_pet_state, state::dismiss_item, state::hand_off_item
        ])
        .setup(move |app| {
            // inbox pipeline: folder watcher + HTTP door + popover commands
            // -> one mpsc channel -> the state thread, the only writer
            let (tx, rx) = mpsc::channel::<state::PetEvent>();
            let feed_cfg = app.state::<AppState>().cfg.lock_or_recover().feed.clone();
            let inbox_dir = feed::prepare_dir(&feed_cfg);
            // the published state: shared between `get_pet_state` and the
            // HTTP door's `GET /state`
            let store = Arc::new(Mutex::new(state::PetStatePayload::default()));
            // A port that is taken is not fatal: the folder feed is the
            // primary door and keeps working. The popover says so.
            let http_ok = match feed::spawn_http(feed_cfg.http_port, feed_cfg.http_token.clone(), tx.clone(), store.clone()) {
                Ok(()) => true,
                Err(e) => {
                    eprintln!("clawd: cannot bind 127.0.0.1:{}: {e} (folder feed still works)", feed_cfg.http_port);
                    false
                }
            };
            feed::spawn_folder_watch(inbox_dir.clone(), tx.clone());
            app.manage(state::PetStateStore(store));
            app.manage(state::EventSender(Mutex::new(tx)));
            let base = state::PetStatePayload {
                sweep_minutes: feed_cfg.sweep_minutes.max(1),
                nudge_minutes: feed_cfg.nudge_minutes.max(1),
                inbox_dir: inbox_dir.to_string_lossy().into_owned(),
                http_ok,
                ..Default::default()
            };
            state::spawn_state_thread(rx, app.handle().clone(), base);

            // on a Mac he lives in the menu bar, not the Dock icon row
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let win = app.get_webview_window("pet").expect("pet window missing");
            let state = app.state::<AppState>();

            // WebView2 keeps its browser accelerators (F5/Ctrl+R reload,
            // Ctrl+F find bar, Ctrl+P print) enabled by default; on a
            // transparent pet those replay the boot wave or surface browser
            // chrome. Tauri 2.11 has no config switch, so flip it on the COM
            // settings object once the webview exists. Best effort: any
            // failure just leaves the default behaviour.
            platform::quiet_browser_keys(&win);
            let saved = state.cfg.lock_or_recover().clone();

            // Restore scale, then position (clamped: only if the saved point
            // still lands on a live monitor), then show — no flash-then-jump.
            let s = saved.scale.clamp(0.5, 3.0);
            let sf = win.scale_factor().unwrap_or(1.0);
            let _ = win.set_size(window_size(s));
            // clickable from the first frame: the sprite rect stands in for
            // the OS hit region until the webview reports the real one
            *state.bounds.lock_or_recover() = boot_bounds(s, sf);
            if let (Some(x), Some(y)) = (saved.x, saved.y) {
                // the BUDDY (not the padded window) must land on some monitor
                let (cx, cy) = buddy_center(x as f64, y as f64, s, sf);
                let on_screen = win
                    .available_monitors()
                    .map(|monitors| on_some_monitor(&monitors, cx, cy))
                    .unwrap_or(false);
                if on_screen {
                    let _ = win.set_position(PhysicalPosition::new(x, y));
                }
            }
            // living on the taskbar: back where he was along it, feet on
            // its edge (the bar may have moved, or the screen changed)
            if saved.on_taskbar {
                let cx = saved.x.map(|x| x as f64 + anchor(&state, s, sf).0);
                if !place_on_taskbar(&win, &state, cx) {
                    state.cfg.lock_or_recover().on_taskbar = false;
                }
            }
            // Started at login: stay hidden until Claude opens (or the tray
            // calls him). Keep the login entry pointing at this copy.
            platform::set_login_item(saved.start_with_claude);
            let at_login = std::env::args().any(|a| a == "--with-claude");
            if at_login && saved.start_with_claude && !platform::claude_running() {
                state.waiting.store(true, Ordering::Relaxed);
                let handle = app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(Duration::from_secs(3));
                    let state = handle.state::<AppState>();
                    if !state.waiting.load(Ordering::Relaxed) {
                        break;
                    }
                    if platform::claude_running() {
                        reveal(&handle);
                        break;
                    }
                });
            } else {
                let _ = win.show();
            }

            // the tray icon: the way to call him out, send him home, or quit
            // without hunting for him and pressing q
            {
                use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
                use tauri::Emitter;
                let show = MenuItem::with_id(app, "summon", "Call Claw’d", true, None::<&str>)?;
                let home = MenuItem::with_id(app, "home", if cfg!(target_os = "macos") { "Back to the Dock" } else { "Back to the taskbar" }, true, None::<&str>)?;
                let restart = MenuItem::with_id(app, "restart", "Start over (back to the egg)", true, None::<&str>)?;
                let sep = PredefinedMenuItem::separator(app)?;
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &home, &restart, &sep, &quit])?;
                let mut tray = TrayIconBuilder::with_id("clawd")
                    .tooltip("Claw’d")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, ev| match ev.id.as_ref() {
                        "quit" => app.exit(0),
                        other => {
                            reveal(app);
                            let _ = app.emit_to("pet", "tray", other.to_string());
                        }
                    })
                    .on_tray_icon_event(|tray, ev| {
                        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                            reveal(tray.app_handle());
                            let _ = tray.app_handle().emit_to("pet", "tray", "summon".to_string());
                        }
                    });
                if let Some(icon) = app.default_window_icon() {
                    tray = tray.icon(icon.clone());
                }
                tray.build(app)?;
            }

            // Persist position/scale, debounced: events mark dirty, a saver
            // thread writes at most once a second.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(1));
                let state = handle.state::<AppState>();
                if state.dirty.swap(false, Ordering::Relaxed) {
                    let cfg = state.cfg.lock_or_recover().clone();
                    save_config(&cfg);
                }
            });

            // Click-through hit test: Tauri's set_ignore_cursor_events is
            // unreliable per-region on Windows (tauri#11461), so poll the
            // cursor at ~60fps and toggle ignore based on the sprite bounds.
            let poller_win = win.clone();
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                // Poll fast when it can matter and back off when it plainly
                // cannot: each tick is two IPC round-trips, and this thread
                // alone was ~8% of a core continuously. A cursor further than
                // FAR_PX from the hit region cannot reach it inside SLOW_TICK
                // without a deliberate flick, and the worst case is one late
                // hover frame.
                const FAST_TICK: u64 = 16;
                const SLOW_TICK: u64 = 50;
                const FAR_PX: f64 = 320.0;
                let mut ignoring = false;
                let mut hover_sent: Option<bool> = None;
                let mut inside = true; // on an API failure the last verdict stands
                // per-drag cache: monitors and scale factor are round-trips too
                let mut drag_env: Option<(Vec<tauri::Monitor>, f64)> = None;
                let mut last_target: Option<(i32, i32)> = None;
                loop {
                    let mut far = false; // cursor nowhere near: safe to idle the poll
                    let state = handle.state::<AppState>();
                    let sample = (|| {
                        let cursor = handle.cursor_position().ok()?; // physical, global
                        let pos = poller_win.outer_position().ok()?; // physical
                        Some((cursor, pos))
                    })();

                    // a walk along the taskbar: slide x, keep y; a drag
                    // always wins over a walk
                    let walk = *state.walk.lock_or_recover();
                    if let (Some(w), None) = (walk, *state.drag.lock_or_recover()) {
                        let t = (w.start.elapsed().as_secs_f64() * 1000.0 / w.ms).min(1.0);
                        let x = (w.from + (w.to - w.from) * t).round() as i32;
                        if let Ok(pos) = poller_win.outer_position() {
                            if pos.x != x {
                                let _ = poller_win.set_position(PhysicalPosition::new(x, pos.y));
                            }
                        }
                        if t >= 1.0 {
                            *state.walk.lock_or_recover() = None;
                            use tauri::Emitter;
                            let _ = handle.emit_to("pet", "walk-done", x);
                        }
                    } else if walk.is_some() {
                        *state.walk.lock_or_recover() = None; // grabbed mid-walk
                    }
                    let walking = walk.is_some();

                    let grab = *state.drag.lock_or_recover();
                    if let Some(g) = grab {
                        // app-driven drag: the window follows the cursor at the
                        // grabbed offset. Click-through is never toggled
                        // mid-drag, and the buddy is kept on a monitor.
                        if !left_button_down() {
                            *state.drag.lock_or_recover() = None; // lost pointerup
                        } else if let Some((cursor, _)) = sample {
                            let (monitors, sf) = drag_env.get_or_insert_with(|| {
                                (
                                    poller_win.available_monitors().unwrap_or_default(),
                                    poller_win.scale_factor().unwrap_or(1.0),
                                )
                            });
                            let pet_scale = state.cfg.lock_or_recover().scale;
                            let (tx, ty) = clamp_to_monitors(monitors, cursor.x - g.dx, cursor.y - g.dy, pet_scale, *sf);
                            let target = (tx.round() as i32, ty.round() as i32);
                            if last_target != Some(target) {
                                last_target = Some(target);
                                let _ = poller_win.set_position(PhysicalPosition::new(target.0, target.1));
                            }
                        }
                        inside = true;
                    } else {
                        drag_env = None;
                        last_target = None;
                        if let Some((cursor, pos)) = sample {
                            // bounds arrive already in physical px — no scaling
                            let b = *state.bounds.lock_or_recover();
                            let (bx, by) = (pos.x as f64 + b.x, pos.y as f64 + b.y);
                            inside = cursor.x >= bx
                                && cursor.x < bx + b.w
                                && cursor.y >= by
                                && cursor.y < by + b.h;
                            // gap to the rect on each axis, 0 when overlapping
                            let gx = (bx - cursor.x).max(cursor.x - (bx + b.w)).max(0.0);
                            let gy = (by - cursor.y).max(cursor.y - (by + b.h)).max(0.0);
                            far = !inside && gx.max(gy) > FAR_PX;
                        }
                    }

                    let should_ignore = !inside;
                    if should_ignore != ignoring {
                        ignoring = should_ignore;
                        let _ = poller_win.set_ignore_cursor_events(ignoring);
                    }
                    // hover ground truth for the frontend: once the window
                    // ignores cursor events, the webview can never see the
                    // mouse leave — this event is how the "+" gets hidden
                    if hover_sent != Some(inside) {
                        hover_sent = Some(inside);
                        use tauri::Emitter;
                        let _ = handle.emit_to("pet", "pet-hover", inside);
                    }
                    std::thread::sleep(Duration::from_millis(if far && !walking { SLOW_TICK } else { FAST_TICK }));
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Moved(pos) = event {
                let state = window.state::<AppState>();
                let mut cfg = state.cfg.lock_or_recover();
                cfg.x = Some(pos.x);
                cfg.y = Some(pos.y);
                state.dirty.store(true, Ordering::Relaxed);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running clawd-assistant");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_open_work_tools_only() {
        let none: Vec<String> = vec![];
        assert!(link_allowed("https://app.slack.com/client/T1/C2", &none));
        assert!(link_allowed("https://acme.slack.com/archives/C1/p123?thread_ts=1&cid=2", &none));
        assert!(link_allowed("https://mail.google.com/mail/u/0/#inbox/abc", &none));
        assert!(link_allowed("https://app.asana.com/0/1/2", &none));
        assert!(link_allowed("claude://claude.ai/new?q=hi", &none));
        assert!(!link_allowed("https://evil.example/", &none));
        assert!(!link_allowed("https://slack.com.evil.example/", &none));
        assert!(!link_allowed("https://slack.com@evil.example/", &none));
        assert!(!link_allowed("http://app.slack.com/", &none));
        assert!(!link_allowed("file:///C:/Windows/System32/calc.exe", &none));
        assert!(!link_allowed("https://app.slack.com/ x", &none));
        assert!(link_allowed("https://acme.zoom.us/j/1", &none));
        assert!(link_allowed("https://intranet.acme.com/x", &["acme.com".into()]));
    }

    #[test]
    fn prompt_encoding_is_query_safe() {
        assert_eq!(percent_encode("a b&c=d/é"), "a%20b%26c%3Dd%2F%C3%A9");
    }
}
