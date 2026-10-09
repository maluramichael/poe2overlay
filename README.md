# PoE2 Overlay

Open-source in-game overlay for Path of Exile 2. No ads, no Overwolf, no account.

Right now it does one thing: a **campaign guide** that sits on top of the game and follows you through the acts.

- Reads `Client.txt` and jumps to the right step when you enter a zone.
- Moves on by itself when you get a step's permanent reward (resistances, spirit, life, mana, passive points).
- Shows the current step (goal, how-to, notes) and the next two steps.
- Knows where you are: in town, hideout or the wrong zone it says **Go to <zone>** for the current step.
- **Mini mode:** one line with zone, what to do there and where to go next.
- Shows character level vs. area level and turns red outside the XP penalty window.
- German and English (tray menu).

The route comes from [poe2speedrun.malura.de](https://poe2speedrun.malura.de/) (patch 0.5.5).

## How it works

Same approach as Exiled Exchange 2 and ExileXP, without Electron:

| Part | How |
|---|---|
| Window | Tauri 2 (Rust + system WebView2). Transparent, borderless, always on top, never takes focus. |
| Follow the game | Every 100 ms: find the window titled `Path of Exile 2`, match its client area, show only while the game is in front. |
| Click-through | Mouse passes through to the game everywhere except over the panel. |
| Zone detection | Tails `logs/Client.txt` and reads `[SCENE] Set Source [<zone>]`, `Generating level N area "<id>"` and `is now level N`. No memory reading, no injection. |
| Progress | Moves forward to the next step in the entered zone (max 8 steps ahead, town steps only when they come next, a portal back from such a town step undoes it). Saved in `%APPDATA%\de.malura.poe2overlay\state.json`. |

Release build: one 8 MB exe. Windows 10/11 already ships WebView2.

The game must run in **Windowed** or **Windowed Fullscreen**. Exclusive fullscreen hides every overlay.

## Hotkeys

| Keys | Action |
|---|---|
| `Ctrl+Alt+Right` / `Ctrl+Alt+Left` | Next / previous step |
| `Ctrl+Alt+C` | Toggle mini mode |
| `Ctrl+Alt+G` | Hide / show overlay |

Drag the panel by its header (mini mode: by the line, a click opens the full panel). Tray icon: language, reset guide, quit.

## Client.txt not found?

While the game runs, the overlay reads the `logs\Client.txt` next to the game's exe, wherever it is installed. Before that it uses the newest log from the usual standalone and Steam paths. To force a path, quit the overlay and add it to `state.json`:

```json
{ "client_log": "D:\\Games\\Path of Exile 2\\logs\\Client.txt" }
```

## Build

```
cargo run              # debug
cargo build --release  # target/release/poe2overlay.exe
cargo test
```

Update the route from a checkout of the guide repo: `uv run scripts/import_guide.py ../poe2`.

## License

MIT
