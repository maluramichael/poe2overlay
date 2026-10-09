//! Keeps the overlay glued to the game window: same position and size,
//! visible only while the game is in front, clickable only over the panel.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};
use windows::core::{HSTRING, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetClientRect, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, IsIconic, SetWindowLongPtrW,
    GWL_EXSTYLE, WS_EX_TRANSPARENT,
};

/// Panel rectangle in physical pixels, relative to the overlay window.
#[derive(Default, Clone, Copy)]
pub struct HitRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub struct Shared {
    pub hit: HitRect,
    /// Set by the user hotkey: stays hidden even while the game has focus.
    pub user_hidden: bool,
    /// Client.txt to follow. The tracker points it at the running game's log
    /// unless the user pinned a path in state.json.
    pub log: Option<PathBuf>,
    pub log_pinned: bool,
}

/// `<game dir>\logs\Client.txt` of the process that owns the window.
fn game_log(hwnd: HWND) -> Option<PathBuf> {
    let mut pid = 0;
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        ok.ok()?;
    }
    let exe = PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]));
    Some(exe.parent()?.join("logs").join("Client.txt")).filter(|p| p.is_file())
}

fn client_rect(hwnd: HWND) -> Option<RECT> {
    let mut r = RECT::default();
    let mut origin = POINT::default();
    unsafe {
        GetClientRect(hwnd, &mut r).ok()?;
        ClientToScreen(hwnd, &mut origin).ok().ok()?;
    }
    Some(RECT { left: origin.x, top: origin.y, right: origin.x + r.right, bottom: origin.y + r.bottom })
}

/// Click-through on or off by toggling only WS_EX_TRANSPARENT. Tauri's
/// set_ignore_cursor_events(false) also drops WS_EX_LAYERED, which throws
/// away the transparent surface and flashes the window white (Electron keeps
/// LAYERED on transparent windows for the same reason). Checked every tick,
/// because tao re-applies its own styles on show/hide.
fn set_click_through(hwnd: HWND, on: bool) {
    let flag = WS_EX_TRANSPARENT.0 as isize;
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if (ex & flag != 0) != on {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex ^ flag);
        }
    }
}

pub fn spawn_tracker(win: WebviewWindow, shared: Arc<Mutex<Shared>>) {
    // Test knob: point the overlay at another window while the real game runs.
    let title = HSTRING::from(std::env::var("POE2OVERLAY_GAME_TITLE").unwrap_or_else(|_| "Path of Exile 2".into()));
    thread::spawn(move || {
        let (mut visible, mut last_rect, mut last_game) = (false, RECT::default(), None);
        // Sets WS_EX_LAYERED | WS_EX_TRANSPARENT once; tao keeps both from here on.
        let _ = win.set_ignore_cursor_events(true);
        // tauri hands out its own windows-crate version of HWND
        let Ok(overlay) = win.hwnd().map(|h| HWND(h.0)) else { return };
        loop {
            // ponytail: 100 ms polling, switch to SetWinEventHook if the lag ever shows
            thread::sleep(Duration::from_millis(100));
            let game = unsafe { FindWindowW(None, &title) }.ok();
            let (hit, user_hidden) = {
                let mut s = shared.lock().unwrap();
                if game.is_some() && game != last_game && !s.log_pinned {
                    if let Some(log) = game.and_then(game_log) {
                        s.log = Some(log);
                    }
                }
                (s.hit, s.user_hidden)
            };
            last_game = game;
            let in_front = game.is_some_and(|g| unsafe { GetForegroundWindow() == g && !IsIconic(g).as_bool() });
            let rect = game.and_then(client_rect).filter(|r| r.right > r.left);
            let want_visible = in_front && rect.is_some() && !user_hidden;

            if let Some(r) = rect.filter(|r| *r != last_rect) {
                let _ = win.set_position(PhysicalPosition::new(r.left, r.top));
                let _ = win.set_size(PhysicalSize::new((r.right - r.left) as u32, (r.bottom - r.top) as u32));
                last_rect = r;
            }
            if want_visible != visible {
                let _ = if want_visible { win.show() } else { win.hide() };
                visible = want_visible;
            }

            let mut cur = POINT::default();
            let over_panel = visible
                && unsafe { GetCursorPos(&mut cur) }.is_ok()
                && (hit.x..hit.x + hit.w).contains(&(cur.x - last_rect.left))
                && (hit.y..hit.y + hit.h).contains(&(cur.y - last_rect.top));
            set_click_through(overlay, !over_panel);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::w;

    #[test]
    #[ignore = "needs a running game: cargo test -- --ignored"]
    fn finds_log_of_running_game() {
        let game = unsafe { FindWindowW(None, w!("Path of Exile 2")) }.expect("game not running");
        let log = game_log(game).expect("no Client.txt next to the game exe");
        println!("{}", log.display());
    }
}
