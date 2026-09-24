// Where the Windows taskbar is, so Claw'd can live on it: stand on its top
// edge, walk along it, and tuck in behind it near the clock.
//
// Only a taskbar along the bottom of the screen counts (Windows 11's only
// supported position). Anything else, or a failed query, means "no taskbar"
// and he simply stays wherever he was put.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::Shell::{SHAppBarMessage, ABE_BOTTOM, ABM_GETTASKBARPOS, APPBARDATA};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowRect, SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

/// Physical screen pixels.
#[derive(Clone, Copy, Debug)]
pub struct Taskbar {
    /// the taskbar's top edge: the line his feet stand on
    pub top: i32,
    pub left: i32,
    pub right: i32,
    /// left edge of the notification area (tray icons and clock); his home
    /// spot is just left of it
    pub tray_left: i32,
}

pub fn query() -> Option<Taskbar> {
    // SAFETY: plain Win32 queries with owned out-structs
    unsafe {
        let mut abd = APPBARDATA { cbSize: std::mem::size_of::<APPBARDATA>() as u32, ..Default::default() };
        if SHAppBarMessage(ABM_GETTASKBARPOS, &mut abd) == 0 || abd.uEdge != ABE_BOTTOM {
            return None;
        }
        let rc = abd.rc;
        if rc.right - rc.left < 200 {
            return None;
        }
        // Windows 11's taskbar window is taller than the bar it draws (the
        // rect starts ~36px above the visible edge at 150%). The desktop work
        // area ends exactly at the visible edge, so that is the line his feet
        // go on. An auto-hidden bar gives a full-height work area: then he
        // stands on the bottom of the screen, which is right too.
        let mut work = RECT::default();
        let visible_top = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut work as *mut RECT as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .ok()
        .map(|_| work.bottom)
        .filter(|&b| b >= rc.top && b <= rc.bottom)
        .unwrap_or(rc.top);
        let tray = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null())
            .ok()
            .and_then(|bar| FindWindowExW(Some(bar), None, w!("TrayNotifyWnd"), PCWSTR::null()).ok())
            .and_then(|n| {
                let mut r = RECT::default();
                GetWindowRect(n, &mut r).ok().map(|_| r)
            });
        // a notification area that isn't on this bar (or wasn't found) gets
        // a sensible guess: the clock end of the bar
        let tray_left = tray
            .map(|r| r.left)
            .filter(|&l| l > rc.left + (rc.right - rc.left) / 2 && l < rc.right)
            .unwrap_or(rc.right - (rc.right - rc.left) / 6);
        Some(Taskbar { top: visible_top, left: rc.left, right: rc.right, tray_left })
    }
}
