# PoE2 Overlay

Open-Source-Ingame-Overlay für Path of Exile 2 (Ersatz für poeoverlay.com ohne Werbung/Overwolf). Bisher nur Kampagnen-Guide. **Kein Electron** (Michaels Vorgabe), Tauri 2 + Rust.

## Struktur

| Pfad | Zweck |
|---|---|
| `src/main.rs` | Tauri-App: Zustand (`Saved` persistiert in `%APPDATA%\de.malura.poe2overlay\state.json`), Commands, Hotkeys, Tray |
| `src/log.rs` | `Client.txt`-Parser + Tail-Thread (250 ms), folgt dem Pfad aus `Shared.log` und wechselt die Datei live, Catch-up aus den letzten 64 KB |
| `src/guide.rs` | Zonen-Matching: Pointer springt beim Zonenwechsel vor (max. 8 Schritte, nie zurück, Stadt nur als direkt nächster Schritt) |
| `src/game.rs` | Win32: Fenster `Path of Exile 2` finden, Overlay auf Client-Rect legen, nur bei Spiel im Vordergrund zeigen, Klick-durch außer über dem Panel |
| `ui/` | HTML/CSS/JS (ohne Build-Schritt), `ui/guide.json` = generierte Route |
| `scripts/import_guide.py` | Route aus `../poe2` (acts/*.json) nach `ui/guide.json`; Einträge mit veralteter `league` fallen raus |
| `scripts/make_icon.py` | erzeugt `icons/icon.ico` |

## Wichtig

- Versionen exakt gepinnt (`=2.12.1` tauri, `=2.7.1` tauri-build). Tauri-Unterkrates müssen zusammenpassen: 2.11.6 + neues tauri-runtime kompiliert nicht.
- `focusable: false` im Fenster-Config: Klick ins Overlay lässt dem Spiel den Fokus. Nicht entfernen.
- Hit-Rect kommt vom JS in CSS-px, Rust rechnet mit `scale_factor` auf physische px um.
- Log-Pfad: sobald das Spielfenster da ist, `logs\Client.txt` neben der exe des Spielprozesses (Michael spielt über Steam auf `D:\Steam\...`, der Standalone-Log unter C: ist veraltet). Prüfen mit `cargo test -- --ignored` bei laufendem Spiel.
- Log-Zeilen-Formate stehen als Tests in `src/log.rs` (aus echtem PoE2-Log, 0.5.x).
- Inhalte der Route werden in `../poe2` gepflegt, nicht hier.

## Testen

`cargo test`. E2E ohne Spiel: PowerShell-WinForms-Fenster mit Titel `Path of Exile 2` öffnen, in `state.json` `client_log` auf eine Test-Datei setzen, SCENE-Zeilen anhängen, Hotkeys per `keybd_event`. In PowerShell-P/Invoke `[NullString]::Value` statt `$null` an `FindWindow` geben.

Commits mit `--author="Michael Malura <michael@malura.de>"`, ohne Co-Authored-By.
