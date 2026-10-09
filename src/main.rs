#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod game;
mod guide;
mod log;

use game::{HitRect, Shared};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Persisted between runs (state.json in the app config dir).
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct Saved {
    step: usize,
    lang: String,
    x: f64,
    y: f64,
    mini: bool,
    /// Override for non-standard install paths, e.g. "D:\Games\Path of Exile 2\logs\Client.txt".
    client_log: Option<PathBuf>,
}

impl Default for Saved {
    fn default() -> Self {
        Saved { step: 0, lang: "de".into(), x: 24.0, y: 160.0, mini: false, client_log: None }
    }
}

/// What the UI renders. Sent whole on every change, it is tiny.
#[derive(Serialize, Clone, Default)]
struct Snapshot {
    #[serde(flatten)]
    saved: Saved,
    zone: String,
    char_level: u32,
    area_level: u32,
    log_found: bool,
}

struct App {
    snap: Mutex<Snapshot>,
    guide: guide::Guide,
    shared: Arc<Mutex<Shared>>,
    state_file: PathBuf,
}

impl App {
    /// Apply a change, persist, and push the new snapshot to the UI.
    fn update(&self, app: &AppHandle, f: impl FnOnce(&mut Snapshot)) {
        let snap = {
            let mut s = self.snap.lock().unwrap();
            f(&mut s);
            s.saved.step = s.saved.step.min(self.guide.len() - 1);
            s.clone()
        };
        let _ = std::fs::write(&self.state_file, serde_json::to_string(&snap.saved).unwrap());
        let _ = app.emit("state", snap);
    }

    fn on_log(&self, app: &AppHandle, e: log::Event) {
        self.update(app, |s| match e {
            log::Event::Scene(zone) => {
                if let Some(i) = self.guide.advance(s.saved.step, &zone) {
                    s.saved.step = i;
                }
                s.zone = zone;
            }
            log::Event::Area { level, .. } => s.area_level = level,
            log::Event::Level(l) => s.char_level = l,
            log::Event::Opened => s.log_found = true,
            log::Event::Reward(r) => {
                if let Some(i) = self.guide.complete(s.saved.step, &r) {
                    s.saved.step = i;
                }
            }
        });
    }

    fn step(&self, app: &AppHandle, delta: i64) {
        self.update(app, |s| s.saved.step = (s.saved.step as i64 + delta).max(0) as usize);
    }
}

#[tauri::command]
fn snapshot(app: State<App>) -> Snapshot {
    app.snap.lock().unwrap().clone()
}

#[tauri::command]
fn step(handle: AppHandle, app: State<App>, delta: i64) {
    app.step(&handle, delta);
}

#[tauri::command]
fn toggle_mini(handle: AppHandle, app: State<App>) {
    app.update(&handle, |s| s.saved.mini = !s.saved.mini);
}

#[tauri::command]
fn save_pos(handle: AppHandle, app: State<App>, x: f64, y: f64) {
    app.update(&handle, |s| (s.saved.x, s.saved.y) = (x, y));
}

/// Panel bounds in CSS pixels; the tracker needs physical ones.
#[tauri::command]
fn set_hit_rect(window: tauri::WebviewWindow, app: State<App>, x: f64, y: f64, w: f64, h: f64) {
    let k = window.scale_factor().unwrap_or(1.0);
    let r = |v: f64| (v * k).round() as i32;
    app.shared.lock().unwrap().hit = HitRect { x: r(x), y: r(y), w: r(w), h: r(h) };
}

fn main() {
    let next = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowRight);
    let prev = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowLeft);
    let hide = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyG);
    let mini = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyC);

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |handle, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let app = handle.state::<App>();
                    if shortcut == &next {
                        app.step(handle, 1);
                    } else if shortcut == &prev {
                        app.step(handle, -1);
                    } else if shortcut == &mini {
                        app.update(handle, |s| s.saved.mini = !s.saved.mini);
                    } else if shortcut == &hide {
                        let mut s = app.shared.lock().unwrap();
                        s.user_hidden = !s.user_hidden;
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![snapshot, step, toggle_mini, save_pos, set_hit_rect])
        .setup(move |tauri_app| {
            let handle = tauri_app.handle().clone();
            for s in [next, prev, hide, mini] {
                if let Err(e) = handle.global_shortcut().register(s) {
                    eprintln!("hotkey {s:?} not available: {e}");
                }
            }

            let dir = handle.path().app_config_dir()?;
            std::fs::create_dir_all(&dir)?;
            let state_file = dir.join("state.json");
            let saved: Saved = std::fs::read_to_string(&state_file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            let pinned = saved.client_log.clone().filter(|p| p.is_file());
            let shared = Arc::new(Mutex::new(Shared {
                hit: HitRect::default(),
                user_hidden: false,
                log_pinned: pinned.is_some(),
                log: pinned.or_else(log::find_client_log),
            }));
            tauri_app.manage(App {
                snap: Mutex::new(Snapshot { saved, ..Default::default() }),
                guide: guide::Guide::parse(include_str!("../ui/guide.json"))?,
                shared: shared.clone(),
                state_file,
            });

            let (h, sh) = (handle.clone(), shared.clone());
            log::spawn_tail(move || sh.lock().unwrap().log.clone(), move |e| h.state::<App>().on_log(&h, e));
            game::spawn_tracker(tauri_app.get_webview_window("overlay").unwrap(), shared);

            let lang = MenuItem::with_id(tauri_app, "lang", "Sprache / Language", true, None::<&str>)?;
            let reset = MenuItem::with_id(tauri_app, "reset", "Guide auf Anfang setzen", true, None::<&str>)?;
            let quit = MenuItem::with_id(tauri_app, "quit", "Beenden", true, None::<&str>)?;
            TrayIconBuilder::new()
                .icon(tauri_app.default_window_icon().unwrap().clone())
                .tooltip("PoE2 Overlay")
                .menu(&Menu::with_items(tauri_app, &[&lang, &reset, &quit])?)
                .on_menu_event(|handle, event| {
                    let app = handle.state::<App>();
                    match event.id.as_ref() {
                        "lang" => app.update(handle, |s| s.saved.lang = if s.saved.lang == "de" { "en".into() } else { "de".into() }),
                        "reset" => app.update(handle, |s| s.saved.step = 0),
                        "quit" => handle.exit(0),
                        _ => {}
                    }
                })
                .build(tauri_app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
