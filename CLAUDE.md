# PoE2 Overlay

Open-Source-Ingame-Overlay für Path of Exile 2 (Ersatz für poeoverlay.com ohne Werbung/Overwolf). Bisher nur Kampagnen-Guide. **Kein Electron** (Michaels Vorgabe), Tauri 2 + Rust.

## Struktur

| Pfad | Zweck |
|---|---|
| `src/main.rs` | Tauri-App: Zustand (`Saved` persistiert in `%APPDATA%\de.malura.poe2overlay\state.json`), Commands, Hotkeys, Tray |
| `src/log.rs` | `Client.txt`-Parser + Tail-Thread (250 ms), folgt dem Pfad aus `Shared.log` und wechselt die Datei live, Catch-up aus den letzten 64 KB |
| `src/guide.rs` | Zonen-Matching: Pointer springt beim Zonenwechsel vor (max. 8 Schritte, nie zurück, Stadt nur als direkt nächster Schritt) |
| `src/game.rs` | Win32: Fenster `Path of Exile 2` finden, Overlay-Fenster = genau das Panel, an Offset `x,y` (physische px) im Client-Bereich, nur bei Spiel im Vordergrund sichtbar, Ziehen über die echte Cursorposition |
| `ui/` | HTML/CSS/JS (ohne Build-Schritt), `ui/guide.json` = generierte Route |
| `scripts/import_guide.py` | Route aus `../poe2` (acts/*.json) nach `ui/guide.json`; Einträge mit veralteter `league` fallen raus |
| `scripts/make_icon.py` | erzeugt `icons/icon.ico` |

## Wichtig

- Versionen exakt gepinnt (`=2.12.1` tauri, `=2.7.1` tauri-build). Tauri-Unterkrates müssen zusammenpassen: 2.11.6 + neues tauri-runtime kompiliert nicht.
- `focusable: false` im Fenster-Config: Klick ins Overlay lässt dem Spiel den Fokus. Nicht entfernen.
- Kein Klick-durch-Umschalten: `set_ignore_cursor_events(false)` entfernt `WS_EX_LAYERED` (weißer Blitz), und mit dauerhaft gesetztem `LAYERED` gehen Klicks durch das Panel ins Spiel (WebView2 zeichnet an der Layered-Fläche vorbei). Darum ist das Fenster nur so groß wie das Panel.
- Schritt erledigt per Belohnungszeile im Log (`: <char> has received +10% to [Resistances|Cold Resistance].`, `: You have received 2 Passive Skill Points.`): Tabelle `REWARDS` in `src/guide.rs` mappt Phrase im englischen Goal-Text nach `->` auf den Log-Text. Kills selbst stehen nicht im Log, Boss-Sprüche schon (`Beira of the Rotten Pack: Invader!`).
- Panelgröße kommt per ResizeObserver vom JS in CSS-px, Rust rechnet mit `scale_factor` auf physische px um und setzt die Fenstergröße.
- Log-Pfad: sobald das Spielfenster da ist, `logs\Client.txt` neben der exe des Spielprozesses (Michael spielt über Steam auf `D:\Steam\...`, der Standalone-Log unter C: ist veraltet). Prüfen mit `cargo test -- --ignored` bei laufendem Spiel.
- Log-Zeilen-Formate stehen als Tests in `src/log.rs` (aus echtem PoE2-Log, 0.5.x).
- Inhalte der Route werden in `../poe2` gepflegt, nicht hier.

## Testen

`cargo test`. E2E ohne Spiel: PowerShell-WinForms-Fenster öffnen und das Overlay mit `POE2OVERLAY_GAME_TITLE=<Titel>` darauf zeigen lassen (kollidiert so nicht mit einem laufenden Spiel). **Keine simulierten Klicks/Tasten, solange PoE2 läuft:** Michael kann jederzeit zurück ins Spiel wechseln, Eingaben landen dann im Bosskampf. Nur lesend prüfen (GetWindowRect, Styles) oder Michael testen lassen, in `state.json` `client_log` auf eine Test-Datei setzen, SCENE-Zeilen anhängen, Hotkeys per `keybd_event`. In PowerShell-P/Invoke `[NullString]::Value` statt `$null` an `FindWindow` geben.

Commits mit `--author="Michael Malura <michael@malura.de>"`, ohne Co-Authored-By.
