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

/// Is Claude running: the Claude app, or Claude Code in a terminal? His own
/// big brain runs (children of this process) don't count, or he'd wake
/// himself up.
pub fn claude_running() -> bool {
    let me = std::process::id();
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        };
        // SAFETY: Toolhelp calls over a locally owned entry struct whose
        // dwSize is set; the snapshot handle is closed on every path
        return unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return false };
            let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            let mut found = false;
            if Process32FirstW(snap, &mut entry).is_ok() {
                loop {
                    let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                    // "Claude.exe" (the app) or "claude.exe" (Claude Code)
                    if name.eq_ignore_ascii_case(CLAUDE_EXE) && entry.th32ParentProcessID != me {
                        found = true;
                        break;
                    }
                    if Process32NextW(snap, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
            found
        };
    }
    #[cfg(not(windows))]
    {
        // pid, parent pid, and the program (a full path on macOS)
        let Ok(out) = Command::new("ps").args(["-Ao", "pid=,ppid=,comm="]).output() else { return false };
        String::from_utf8_lossy(&out.stdout).lines().any(|l| {
            let mut parts = l.split_whitespace();
            let (_pid, ppid) = (parts.next(), parts.next().and_then(|p| p.parse::<u32>().ok()));
            let comm = parts.collect::<Vec<_>>().join(" ");
            let base = comm.rsplit('/').next().unwrap_or("");
            // "Claude" (the app's main process, not its helpers) or "claude"
            base.eq_ignore_ascii_case("claude") && ppid != Some(me)
        })
    }
}

/// Start him when the user logs in (hidden until Claude opens: `--with-claude`).
/// Windows: the per-user Run key. Mac: a LaunchAgent. Returns false if it
/// couldn't be changed.
pub fn set_login_item(on: bool) -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    #[cfg(windows)]
    {
        use windows::core::w;
        use windows::Win32::System::Registry::{RegDeleteKeyValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};
        let key = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
        let name = w!("ClawdAssistant");
        // SAFETY: plain registry calls with owned, NUL-terminated wide strings
        return unsafe {
            if on {
                let line = format!("\"{}\" --with-claude", exe.display());
                let wide: Vec<u16> = line.encode_utf16().chain(std::iter::once(0)).collect();
                RegSetKeyValueW(HKEY_CURRENT_USER, key, name, REG_SZ.0, Some(wide.as_ptr() as *const _), (wide.len() * 2) as u32).is_ok()
            } else {
                let r = RegDeleteKeyValueW(HKEY_CURRENT_USER, key, name);
                r.is_ok() || r == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND
            }
        };
    }
    #[cfg(target_os = "macos")]
    {
        let dir = home_dir().join("Library").join("LaunchAgents");
        let plist = dir.join("com.clawd.assistant.plist");
        if !on {
            return std::fs::remove_file(&plist).is_ok() || !plist.exists();
        }
        let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key><string>com.clawd.assistant</string>\n  <key>ProgramArguments</key>\n  <array><string>{}</string><string>--with-claude</string></array>\n  <key>RunAtLoad</key><true/>\n  <key>ProcessType</key><string>Interactive</string>\n</dict>\n</plist>\n",
            esc(&exe.to_string_lossy())
        );
        let _ = std::fs::create_dir_all(&dir);
        if std::fs::read_to_string(&plist).map_or(false, |s| s == body) {
            return true;
        }
        return std::fs::write(&plist, body).is_ok();
    }
    #[allow(unreachable_code)]
    {
        let _ = (on, exe);
        false
    }
}
