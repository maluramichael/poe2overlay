//! Keeps the overlay glued to the game window: same position and size,
//! visible only while the game is in front, clickable only over the panel.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};
use windows::core::w;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetClientRect, GetCursorPos, GetForegroundWindow, IsIconic};

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
        let (mut visible, mut ignoring, mut last_rect) = (false, true, RECT::default());
        let _ = win.set_ignore_cursor_events(true);
        loop {
            // ponytail: 100 ms polling, switch to SetWinEventHook if the lag ever shows
            thread::sleep(Duration::from_millis(100));
            let game = unsafe { FindWindowW(None, w!("Path of Exile 2")) }.ok();
            let (hit, user_hidden) = {
                let s = shared.lock().unwrap();
                (s.hit, s.user_hidden)
            };
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
