// Everything that differs between Windows and macOS, in one place. The rest
// of the app asks here: where his folders live, whether the mouse button is
// down, how to open a link, where the big brain's program is, how to start
// it without a console window, and how to open it in a terminal.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The user's home folder.
pub fn home_dir() -> PathBuf {
    #[cfg(windows)]
    let h = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let h = std::env::var_os("HOME");
    h.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

/// His own settings and state: %APPDATA%\ClawdAssistant on Windows,
/// ~/Library/Application Support/ClawdAssistant on a Mac.
pub fn app_data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    return std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("ClawdAssistant"));
    #[cfg(target_os = "macos")]
    return Some(home_dir().join("Library").join("Application Support").join("ClawdAssistant"));
    #[cfg(not(any(windows, target_os = "macos")))]
    return Some(home_dir().join(".config").join("ClawdAssistant"));
}

/// Is the left mouse button held right now? (The drag follows the cursor
/// until it isn't.)
pub fn left_button_down() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
        // SAFETY: a plain Win32 state query, no pointers involved
        return unsafe { (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16) & 0x8000 != 0 };
    }
    #[cfg(target_os = "macos")]
    {
        // bit 0 of NSEvent.pressedMouseButtons is the left button
        #[allow(unused_unsafe)]
        return unsafe { objc2_app_kit::NSEvent::pressedMouseButtons() } & 1 != 0;
    }
    #[allow(unreachable_code)]
    true
}

/// Is his window the one in front? (Only Windows needs the answer: resizing
/// there can drop the web view's keyboard focus.)
pub fn is_foreground(win: &tauri::WebviewWindow) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        return win.hwnd().map(|h| h == unsafe { GetForegroundWindow() }).unwrap_or(false);
    }
    #[allow(unreachable_code)]
    {
        let _ = win;
        false
    }
}

/// Hand a URL to the system (default browser, or the Claude app for
/// claude://). Never through a shell: the URL is one argument.
pub fn open_url(url: &str) -> bool {
    #[cfg(windows)]
    {
        use windows::core::{w, HSTRING, PCWSTR};
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        // SAFETY: plain Win32 call with owned, NUL-terminated wide strings
        let r = unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(url), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
        // ShellExecute reports success as a value greater than 32
        return r.0 as usize > 32;
    }
    #[cfg(target_os = "macos")]
    return Command::new("open").arg(url).spawn().is_ok();
    #[allow(unreachable_code)]
    Command::new("xdg-open").arg(url).spawn().is_ok()
}

/// The big brain program's file name.
pub const CLAUDE_EXE: &str = if cfg!(windows) { "claude.exe" } else { "claude" };

/// Folders that hold one folder per Claude Code version (the copy inside the
/// Claude desktop app), newest version wins.
pub fn claude_app_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    #[cfg(windows)]
    {
        if let Some(a) = std::env::var_os("APPDATA") {
            v.push(PathBuf::from(a).join("Claude").join("claude-code"));
        }
        // the Microsoft Store app keeps its copy in its own package cache
        if let Some(l) = std::env::var_os("LOCALAPPDATA") {
            if let Ok(rd) = std::fs::read_dir(PathBuf::from(l).join("Packages")) {
                for e in rd.flatten() {
                    if e.file_name().to_string_lossy().starts_with("Claude_") {
                        v.push(e.path().join("LocalCache").join("Roaming").join("Claude").join("claude-code"));
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    v.push(home_dir().join("Library").join("Application Support").join("Claude").join("claude-code"));
    v
}

/// Other places a person's own Claude Code install usually lives.
pub fn claude_user_paths() -> Vec<PathBuf> {
    #[allow(unused_mut)]
    let mut v = vec![home_dir().join(".local").join("bin").join(CLAUDE_EXE), home_dir().join(".claude").join("local").join(CLAUDE_EXE)];
    #[cfg(target_os = "macos")]
    {
        v.push(PathBuf::from("/opt/homebrew/bin/claude"));
        v.push(PathBuf::from("/usr/local/bin/claude"));
    }
    v
}

/// The program inside one version folder (a bare binary, or on a Mac
/// possibly inside a .app bundle).
pub fn claude_in_version_dir(dir: &Path) -> Option<PathBuf> {
    let direct = dir.join(CLAUDE_EXE);
    if direct.is_file() {
        return Some(direct);
    }
    #[cfg(target_os = "macos")]
    {
        let bundled = dir.join("claude.app").join("Contents").join("MacOS").join("claude");
        if bundled.is_file() {
            return Some(bundled);
        }
    }
    None
}

/// Start without flashing a console window (Windows); nothing to do elsewhere.
pub fn no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// Open the big brain in a terminal window of its own, in `dir`, running
/// `first_command` (/login, /mcp): the one-time things only a person can do.
pub fn open_terminal(dir: &Path, exe: &Path, first_command: &str) -> bool {
    #[cfg(windows)]
    {
        return Command::new("cmd")
            .current_dir(dir)
            .args(["/C", "start", "Claw'd - the big brain"])
            .arg(exe)
            .arg(first_command)
            .spawn()
            .is_ok();
    }
    #[cfg(target_os = "macos")]
    {
        // Terminal runs one shell line; quote both paths for it, then quote
        // that line for AppleScript
        let sh = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        let line = format!("cd {} && {} {}", sh(&dir.to_string_lossy()), sh(&exe.to_string_lossy()), sh(first_command));
        let script = format!(
            "tell application \"Terminal\" to do script \"{}\"\ntell application \"Terminal\" to activate",
            line.replace('\\', "\\\\").replace('"', "\\\"")
        );
        return Command::new("osascript").arg("-e").arg(script).spawn().is_ok();
    }
    #[allow(unreachable_code)]
    {
        let _ = (dir, exe, first_command);
        false
    }
}

/// Left edge of the clock and tray icons on the main taskbar, if it lies
/// within [left, right). Windows only: a Mac's Dock has no clock end, so his
/// home there is the right-hand end of the Dock's line.
pub fn main_tray_left(left: i32, right: i32) -> Option<i32> {
    #[cfg(windows)]
    {
        use windows::core::{w, PCWSTR};
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, GetWindowRect};
        // SAFETY: plain Win32 queries with an owned out-struct
        return unsafe {
            let bar = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()).ok()?;
            let tray = FindWindowExW(Some(bar), None, w!("TrayNotifyWnd"), PCWSTR::null()).ok()?;
            let mut r = RECT::default();
            GetWindowRect(tray, &mut r).ok()?;
            (r.left > left + (right - left) / 2 && r.left < right).then_some(r.left)
        };
    }
    #[allow(unreachable_code)]
    {
        let _ = (left, right);
        None
    }
}

/// WebView2 keeps browser shortcuts (F5 reload, Ctrl+F find, Ctrl+P print)
/// on by default; on a transparent pet they replay the boot or show browser
/// chrome. Switch them off (Windows only; WKWebView has none of these).
pub fn quiet_browser_keys(win: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        let _ = win.with_webview(|webview| {
            use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
            use windows::core::Interface;
            // SAFETY: COM calls on the controller Tauri owns, made on the
            // webview's own thread by with_webview
            unsafe {
                if let Ok(core) = webview.controller().CoreWebView2() {
                    if let Ok(settings) = core.Settings() {
                        if let Ok(s3) = settings.cast::<ICoreWebView2Settings3>() {
                            let _ = s3.SetAreBrowserAcceleratorKeysEnabled(false);
                        }
                    }
                }
            }
        });
    }
    #[cfg(not(windows))]
    let _ = win;
}
