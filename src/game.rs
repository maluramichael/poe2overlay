//! Keeps the overlay glued to the game window: same position and size,
//! visible only while the game is in front, clickable only over the panel.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};
use windows::core::{w, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetClientRect, GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId, IsIconic};

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

pub fn spawn_tracker(win: WebviewWindow, shared: Arc<Mutex<Shared>>) {
    thread::spawn(move || {
        let (mut visible, mut ignoring, mut last_rect, mut last_game) = (false, true, RECT::default(), None);
        let _ = win.set_ignore_cursor_events(true);
        loop {
            // ponytail: 100 ms polling, switch to SetWinEventHook if the lag ever shows
            thread::sleep(Duration::from_millis(100));
            let game = unsafe { FindWindowW(None, w!("Path of Exile 2")) }.ok();
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
            if over_panel == ignoring {
                let _ = win.set_ignore_cursor_events(!over_panel);
                ignoring = !over_panel;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs a running game: cargo test -- --ignored"]
    fn finds_log_of_running_game() {
        let game = unsafe { FindWindowW(None, w!("Path of Exile 2")) }.expect("game not running");
        let log = game_log(game).expect("no Client.txt next to the game exe");
        println!("{}", log.display());
    }
}
