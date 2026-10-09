//! Tails the game's Client.txt and turns the lines we care about into events.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

#[derive(Debug, PartialEq, Clone)]
pub enum Event {
    /// `[SCENE] Set Source [The Riverbank]`: display name of the zone just entered.
    Scene(String),
    /// `Generating level 14 area "G1_7" with seed ...`
    Area { id: String, level: u32 },
    /// `: Name (Class) is now level 12`
    Level(u32),
}

pub fn parse(line: &str) -> Option<Event> {
    if let Some(i) = line.find("[SCENE] Set Source [") {
        let rest = &line[i + "[SCENE] Set Source [".len()..];
        let name = rest[..rest.rfind(']')?].trim();
        // "(null)" / "(unknown)" mark loading screens and character select.
        if name.is_empty() || name.starts_with('(') {
            return None;
        }
        return Some(Event::Scene(name.to_string()));
    }
    if let Some(i) = line.find("Generating level ") {
        let rest = &line[i + "Generating level ".len()..];
        let (level, rest) = rest.split_once(" area \"")?;
        let id = &rest[..rest.find('"')?];
        return Some(Event::Area { id: id.to_string(), level: level.trim().parse().ok()? });
    }
    if let Some(i) = line.find(") is now level ") {
        // ponytail: also fires for party members, add a character-name filter if that bites
        let n = &line[i + ") is now level ".len()..];
        return Some(Event::Level(n.trim().parse().ok()?));
    }
    None
}

/// Default install locations, standalone first, then Steam on the usual drives.
pub fn find_client_log() -> Option<PathBuf> {
    let mut candidates = vec![PathBuf::from(r"C:\Program Files (x86)\Grinding Gear Games\Path of Exile 2\logs\Client.txt")];
    for drive in ['C', 'D', 'E', 'F', 'G', 'H'] {
        candidates.push(format!(r"{drive}:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2\logs\Client.txt").into());
        candidates.push(format!(r"{drive}:\SteamLibrary\steamapps\common\Path of Exile 2\logs\Client.txt").into());
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Replays the latest state from the end of the log, then follows new lines forever.
pub fn spawn_tail(path: PathBuf, on_event: impl Fn(Event) + Send + 'static) {
    thread::spawn(move || {
        let mut offset = catch_up(&path, &on_event).unwrap_or(0);
        let mut partial = String::new();
        loop {
            thread::sleep(Duration::from_millis(250));
            let Ok(mut file) = File::open(&path) else { continue };
            let Ok(len) = file.metadata().map(|m| m.len()) else { continue };
            if len < offset {
                offset = 0; // log was truncated or replaced
            }
            if len == offset || file.seek(SeekFrom::Start(offset)).is_err() {
                continue;
            }
            let mut buf = Vec::new();
            let Ok(n) = file.read_to_end(&mut buf) else { continue };
            offset += n as u64;
            partial.push_str(&String::from_utf8_lossy(&buf));
            while let Some(nl) = partial.find('\n') {
                let line: String = partial.drain(..=nl).collect();
                if let Some(e) = parse(&line) {
                    on_event(e);
                }
            }
        }
    });
}

/// Emits the last level, area and scene from the final 64 KB so the overlay
/// starts in the right place. Returns the file length to continue from.
fn catch_up(path: &Path, on_event: &impl Fn(Event)) -> Option<u64> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let start = len.saturating_sub(64 * 1024);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;
    let (mut level, mut area, mut scene) = (None, None, None);
    for e in String::from_utf8_lossy(&buf).lines().filter_map(parse) {
        match e {
            Event::Level(_) => level = Some(e),
            Event::Area { .. } => area = Some(e),
            Event::Scene(_) => scene = Some(e),
        }
    }
    [level, area, scene].into_iter().flatten().for_each(on_event);
    Some(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_lines() {
        assert_eq!(
            parse("2025/10/07 19:55:37 47359781 775aed1e [INFO Client 36300] [SCENE] Set Source [The Riverbank]"),
            Some(Event::Scene("The Riverbank".into()))
        );
        assert_eq!(parse("2025/10/07 19:55:36 47358812 775aed1e [INFO Client 36300] [SCENE] Set Source [(null)]"), None);
        assert_eq!(
            parse("2025/10/07 19:55:36 47358875 2caa1a34 [DEBUG Client 36300] Generating level 1 area \"G1_1\" with seed 1608264788"),
            Some(Event::Area { id: "G1_1".into(), level: 1 })
        );
        assert_eq!(
            parse("2025/10/07 20:01:02 47400000 cff945b9 [INFO Client 36300] : sudo_zap (Sorceress) is now level 12\r\n"),
            Some(Event::Level(12))
        );
        assert_eq!(parse("2025/10/07 20:01:02 [INFO Client 36300] Connecting to instance server"), None);
    }

    #[test]
    fn tail_follows_appended_lines() {
        let path = std::env::temp_dir().join(format!("poe2overlay-test-{}.txt", std::process::id()));
        std::fs::write(&path, "x [SCENE] Set Source [Clearfell]\n").unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        spawn_tail(path.clone(), move |e| tx.send(e).unwrap());
        let wait = || rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(wait(), Event::Scene("Clearfell".into()));
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        std::io::Write::write_all(&mut f, b"x [SCENE] Set Source [The Grel").unwrap();
        std::io::Write::write_all(&mut f, b"wood]\r\n").unwrap();
        assert_eq!(wait(), Event::Scene("The Grelwood".into()));
        let _ = std::fs::remove_file(path);
    }
}
