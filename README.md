# PoE2 Overlay

Open-source in-game overlay for Path of Exile 2. No ads, no Overwolf, no account.

Right now it does one thing: a **campaign guide** that sits on top of the game and follows you through the acts.

- Reads `Client.txt` and jumps to the right step when you enter a zone.
- Moves on by itself when you get a step's permanent reward (resistances, spirit, life, mana, passive points).
- Shows the current step (goal, how-to, notes) and the next two steps.
- Knows where you are: in town, hideout or the wrong zone it says **Go to <zone>** for the current step.
- **Mini mode:** one line with zone, what to do there and where to go next.
- Shows character level vs. area level and turns red outside the XP penalty window.
- **Settings** in game (`Shift+Space`): hotkeys, language, route mode, regexes.
- **Regex list** (`Ctrl+Alt+R`): click a saved regex (vendor, stash, from a build guide) and it is on the clipboard, focus goes back to the game.
- **Route mode** like on the website: everything, required + buffs, or required only.
- German and English.

The route comes from [poe2speedrun.malura.de](https://poe2speedrun.malura.de/). The overlay fetches `/overlay/guide.json` on every start, so a route update there reaches everyone without a new release. Offline it uses the last fetched copy, or the one built in.

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
| `Shift+Space` | Settings |
| `Ctrl+Alt+R` | Regex list (click or 1 to 9 copies) |

All of them can be changed in the settings.

Drag the panel by its header (mini mode: by the line, a click opens the full panel). In the lower half of the screen it opens upwards. A hotkey taken by another app (e.g. Exiled Exchange 2) shows in red and is retried every 5 seconds. Tray icon: settings, reset guide, quit.

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

Refresh the built-in fallback route: `curl -o ui/guide.json https://poe2speedrun.malura.de/overlay/guide.json`.

## License

MIT
