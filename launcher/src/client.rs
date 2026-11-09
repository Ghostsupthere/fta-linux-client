//! Finding the client folder and a few facts about it.

use std::path::{Path, PathBuf};

/// A client folder is the one that holds `Data` and `Interface` (the same test the installer uses).
pub fn is_client(dir: &Path) -> bool {
    dir.join("Data").is_dir() && dir.join("Interface").is_dir()
}

/// Order: `--client DIR`, `FTA_CLIENT_DIR`, then the folder this launcher is installed in
/// (`<client>/FTA-Linux/fta-launcher`).
pub fn find(args: &[String]) -> Option<PathBuf> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--client" {
            if let Some(dir) = it.next() {
                let p = PathBuf::from(dir);
                return is_client(&p).then_some(p);
            }
        }
    }
    if let Ok(dir) = std::env::var("FTA_CLIENT_DIR") {
        let p = PathBuf::from(dir);
        if is_client(&p) {
            return Some(p);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let client = exe.parent()?.parent()?.to_path_buf();
    is_client(&client).then_some(client)
}

/// The overlay's build stamp (`FTA-Linux/VERSION.txt`), as one line, if installed.
pub fn installed_stamp(client: &Path) -> Option<String> {
    let text = std::fs::read_to_string(client.join("FTA-Linux/VERSION.txt")).ok()?;
    let line = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("  ·  ");
    (!line.is_empty()).then_some(line)
}

/// True while a `Wow64` process of this user is running. Settings files must not be edited then: the game
/// rewrites `Config.wtf` when it exits and would undo the change.
pub fn game_running() -> bool {
    let Ok(dir) = std::fs::read_dir("/proc") else { return false };
    for entry in dir.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(exe) = std::fs::read_link(entry.path().join("exe")) {
            if exe.file_name().and_then(|n| n.to_str()) == Some("Wow64") {
                return true;
            }
        }
    }
    false
}
