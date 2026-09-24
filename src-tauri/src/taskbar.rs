// Where Claw'd can stand: the taskbar edge of whichever display he's on, so
// he can live on it, walk along it, and tuck in behind it.
//
// Each display's "work area" ends exactly where its taskbar's visible edge
// begins (Windows 11's taskbar window is taller than the bar it draws, so
// its own rect would leave him floating). A display with no taskbar along
// the bottom (or an auto-hidden one) has a work area reaching the bottom of
// the screen, and he stands on the screen's edge instead, which is right.
//
// Home is just left of the clock on the main display. Other displays get the
// right-hand end, where Windows puts their clock when it shows one.

use tauri::WebviewWindow;

/// Physical screen pixels.
#[derive(Clone, Debug)]
pub struct Taskbar {
    /// the line his feet stand on
    pub top: i32,
    pub left: i32,
    pub right: i32,
    /// left edge of the clock and tray icons; home is just left of it
    pub tray_left: i32,
    /// which display: its index, its name, and whether it's the main one
    pub display: usize,
    pub display_name: String,
    pub primary: bool,
}

/// The ground for a point on screen: the display containing (x, y), or the
/// nearest one if the point is off every display.
pub fn query_at(win: &WebviewWindow, x: f64, y: f64) -> Option<Taskbar> {
    let monitors = win.available_monitors().ok()?;
    if monitors.is_empty() {
        return None;
    }
    let contains = |m: &tauri::Monitor| {
        let (p, s) = (m.position(), m.size());
        x >= p.x as f64 && x < p.x as f64 + s.width as f64 && y >= p.y as f64 && y < p.y as f64 + s.height as f64
    };
    let dist = |m: &tauri::Monitor| {
        let (p, s) = (m.position(), m.size());
        let cx = p.x as f64 + s.width as f64 / 2.0;
        let cy = p.y as f64 + s.height as f64 / 2.0;
        (cx - x).powi(2) + (cy - y).powi(2)
    };
    let idx = monitors.iter().position(contains).unwrap_or_else(|| {
        (0..monitors.len()).min_by(|&a, &b| dist(&monitors[a]).total_cmp(&dist(&monitors[b]))).unwrap_or(0)
    });
    let m = &monitors[idx];
    let wa = m.work_area();
    let left = wa.position.x;
    let right = left + wa.size.width as i32;
    let bottom = wa.position.y + wa.size.height as i32;
    if right - left < 200 {
        return None;
    }
    let primary = win
        .primary_monitor()
        .ok()
        .flatten()
        .map_or(idx == 0, |p| p.position() == m.position());
    let tray_left = primary
        .then(|| crate::platform::main_tray_left(left, right))
        .flatten()
        .unwrap_or(right - (right - left) / 8);
    Some(Taskbar {
        top: bottom,
        left,
        right,
        tray_left,
        display: idx,
        display_name: m.name().cloned().unwrap_or_default(),
        primary,
    })
}
