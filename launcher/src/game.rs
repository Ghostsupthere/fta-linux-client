//! The Game page: realm address and the basic display options.
//!
//! The realm lives in two places and both are kept in step: `Data/<locale>/realmlist.wtf` (what players edit and
//! what the game reads at start) and `SET realmList` in `WTF/Config.wtf`. The display options are game variables
//! in `Config.wtf`. As on the Settings page, only what the user changed is written, with a backup first.

use crate::cfgfile::{write_atomic, Doc, Style};
use crate::settings::backup_files;
use std::path::{Path, PathBuf};

pub const RESOLUTION_PRESETS: &[&str] = &["1280x720", "1600x900", "1920x1080", "2560x1440", "3840x2160"];
/// Anti-aliasing choices: the game's `gxMultisample` is the sample count, 1 meaning off.
pub const MSAA_CHOICES: &[u8] = &[1, 2, 4, 8];

#[derive(Clone, PartialEq, Debug)]
pub struct Values {
    pub realm: String,
    pub windowed: bool,
    pub vsync: bool,
    pub resolution: String,
    pub msaa: u8,
}

pub struct Game {
    client: PathBuf,
    pub values: Values,
    saved: Values,
}

fn wtf_path(client: &Path) -> PathBuf {
    client.join("WTF/Config.wtf")
}

/// `Data/<locale>/realmlist.wtf`: the locale from Config.wtf, else the one locale folder that has the file.
fn realmlist_path(client: &Path, wtf: &Doc) -> Option<PathBuf> {
    if let Some(locale) = wtf.get("locale") {
        // The locale is a value from a file we do not control: only a plain folder name is followed.
        if locale.len() == 4 && locale.bytes().all(|b| b.is_ascii_alphabetic()) {
            let dir = client.join("Data").join(&locale);
            if dir.is_dir() {
                return Some(dir.join("realmlist.wtf"));
            }
        }
    }
    let mut found: Vec<PathBuf> = std::fs::read_dir(client.join("Data"))
        .ok()?
        .flatten()
        .map(|e| e.path().join("realmlist.wtf"))
        .filter(|p| p.is_file())
        .collect();
    (found.len() == 1).then(|| found.remove(0))
}

/// Why `realm` cannot be used, if it cannot. A realm address is a host name or IP address, with an optional port.
pub fn realm_problem(realm: &str) -> Option<&'static str> {
    if realm.is_empty() {
        return Some("Enter the server address.");
    }
    if realm.len() > 253 {
        return Some("That address is too long.");
    }
    if !realm.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':' | b'_' | b'[' | b']')) {
        return Some("Use only letters, numbers, dots, dashes and a port (like 192.168.1.5 or play.example.org:3724).");
    }
    None
}

/// Parses `1920x1080`.
pub fn parse_resolution(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.split_once('x')?;
    if !(w.bytes().all(|b| b.is_ascii_digit()) && h.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let (w, h): (u32, u32) = (w.parse().ok()?, h.parse().ok()?);
    ((640..=16384).contains(&w) && (480..=16384).contains(&h)).then_some((w, h))
}

pub fn resolution_problem(text: &str) -> Option<&'static str> {
    parse_resolution(text).is_none().then_some("Write the size as width x height, like 1920x1080.")
}

fn flag(value: Option<String>, default: bool) -> bool {
    match value {
        Some(v) => crate::catalog::parse_bool(&v),
        None => default,
    }
}

impl Game {
    pub fn load(client: &Path) -> Game {
        let wtf = Doc::load(Style::Wtf, &wtf_path(client));
        let realm_file = realmlist_path(client, &wtf).map(|p| Doc::load(Style::Realmlist, &p));
        let realm = realm_file
            .as_ref()
            .and_then(|d| d.get("realmlist"))
            .or_else(|| wtf.get("realmList"))
            .unwrap_or_default();
        let msaa = wtf
            .get("gxMultisample")
            .and_then(|v| v.trim().parse::<u8>().ok())
            .filter(|n| MSAA_CHOICES.contains(n))
            .unwrap_or(1);
        let values = Values {
            realm,
            windowed: flag(wtf.get("gxWindow"), true),
            vsync: flag(wtf.get("gxVSync"), true),
            resolution: wtf.get("gxResolution").unwrap_or_default(),
            msaa,
        };
        Game { client: client.to_path_buf(), saved: values.clone(), values }
    }

    pub fn is_dirty(&self) -> bool {
        self.values != self.saved
    }

    /// The first problem with the values on the page, if any (Apply stays off until it is fixed).
    pub fn problem(&self) -> Option<&'static str> {
        realm_problem(&self.values.realm).or_else(|| {
            // An untouched resolution that the files never had is not an error.
            if self.values.resolution == self.saved.resolution && self.values.resolution.is_empty() {
                None
            } else {
                resolution_problem(&self.values.resolution)
            }
        })
    }

    pub fn apply(&mut self) -> Result<String, String> {
        if !self.is_dirty() {
            return Ok("Nothing to save.".to_string());
        }
        if let Some(p) = self.problem() {
            return Err(p.to_string());
        }
        let wtf_file = wtf_path(&self.client);
        let mut wtf = Doc::load(Style::Wtf, &wtf_file);
        let realm_file = realmlist_path(&self.client, &wtf);
        let mut realm = realm_file.as_ref().map(|p| Doc::load(Style::Realmlist, p));

        let (v, s) = (&self.values, &self.saved);
        let mut wtf_changed = false;
        let mut realm_changed = false;
        if v.realm != s.realm {
            wtf_changed |= wtf.set("realmList", &v.realm);
            if let Some(doc) = realm.as_mut() {
                realm_changed |= doc.set("realmlist", &v.realm);
            }
        }
        if v.windowed != s.windowed {
            wtf_changed |= wtf.set("gxWindow", if v.windowed { "1" } else { "0" });
        }
        if v.vsync != s.vsync {
            wtf_changed |= wtf.set("gxVSync", if v.vsync { "1" } else { "0" });
        }
        if v.resolution != s.resolution {
            wtf_changed |= wtf.set("gxResolution", &v.resolution);
        }
        if v.msaa != s.msaa {
            wtf_changed |= wtf.set("gxMultisample", &v.msaa.to_string());
        }

        let mut touched: Vec<&Path> = Vec::new();
        if wtf_changed {
            touched.push(&wtf_file);
        }
        if let (true, Some(p)) = (realm_changed, realm_file.as_ref()) {
            touched.push(p);
        }
        let backup = backup_files(&self.client, &touched)?;
        if wtf_changed {
            write_atomic(&wtf_file, &wtf.to_text()).map_err(|e| format!("Could not write Config.wtf: {e}"))?;
        }
        if let (true, Some(p), Some(doc)) = (realm_changed, realm_file.as_ref(), realm.as_ref()) {
            write_atomic(p, &doc.to_text()).map_err(|e| format!("Could not write realmlist.wtf: {e}"))?;
        }
        self.saved = self.values.clone();
        Ok(match backup {
            Some(dir) => format!("Saved. Restart the game to use the changes. Previous files kept in {}", dir.display()),
            None => "Saved. Restart the game to use the changes.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fta-launcher-game-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("WTF")).unwrap();
        fs::create_dir_all(dir.join("Data/enUS")).unwrap();
        fs::create_dir_all(dir.join("Interface")).unwrap();
        dir
    }

    #[test]
    fn realm_validation() {
        assert!(realm_problem("192.168.50.215").is_none());
        assert!(realm_problem("play.example.org:3724").is_none());
        assert!(realm_problem("").is_some());
        assert!(realm_problem("bad host").is_some());
        assert!(realm_problem("x\"; rm -rf").is_some());
        assert!(realm_problem("a\nb").is_some());
    }

    #[test]
    fn resolution_validation() {
        assert_eq!(parse_resolution("1920x1080"), Some((1920, 1080)));
        for bad in ["", "1920", "1920x", "x1080", "1920X1080", "10x10", "99999x1080", "-1920x1080", "1920x1080x2"] {
            assert!(parse_resolution(bad).is_none(), "{bad}");
        }
        for p in RESOLUTION_PRESETS {
            assert!(parse_resolution(p).is_some());
        }
    }

    #[test]
    fn apply_updates_both_realm_files_and_only_what_changed() {
        let dir = scratch("apply");
        let wtf = "SET locale \"enUS\"\r\nSET realmList \"1.2.3.4\"\r\nSET gxWindow \"1\"\r\nSET gxResolution \"1280x720\"\r\nSET accountName \"secret\"\r\n";
        fs::write(dir.join("WTF/Config.wtf"), wtf).unwrap();
        fs::write(dir.join("Data/enUS/realmlist.wtf"), "set realmlist 1.2.3.4\r\n").unwrap();

        let mut g = Game::load(&dir);
        assert_eq!(g.values.realm, "1.2.3.4");
        assert!(g.values.windowed && g.values.vsync, "missing vsync uses the default");
        assert!(!g.is_dirty());
        g.values.realm = "wow.example.org".into();
        g.values.windowed = false;
        g.values.resolution = "1920x1080".into();
        g.values.msaa = 4;
        assert!(g.is_dirty());
        g.apply().unwrap();
        assert!(!g.is_dirty());

        assert_eq!(
            fs::read_to_string(dir.join("WTF/Config.wtf")).unwrap(),
            "SET locale \"enUS\"\r\nSET realmList \"wow.example.org\"\r\nSET gxWindow \"0\"\r\nSET gxResolution \"1920x1080\"\r\nSET accountName \"secret\"\r\nSET gxMultisample \"4\"\r\n"
        );
        assert_eq!(fs::read_to_string(dir.join("Data/enUS/realmlist.wtf")).unwrap(), "set realmlist wow.example.org\r\n");
        let b: Vec<_> = fs::read_dir(dir.join("FTA-Linux/settings-backup")).unwrap().flatten().collect();
        assert_eq!(b.len(), 1);
        assert_eq!(fs::read_to_string(b[0].path().join("Config.wtf")).unwrap(), wtf);
        let again = Game::load(&dir);
        assert_eq!(again.values, g.values);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn display_change_leaves_realmlist_alone() {
        let dir = scratch("display");
        fs::write(dir.join("WTF/Config.wtf"), "SET locale \"enUS\"\nSET gxVSync \"0\"\n").unwrap();
        fs::write(dir.join("Data/enUS/realmlist.wtf"), "set realmlist a.b\n").unwrap();
        let mut g = Game::load(&dir);
        g.values.vsync = true;
        g.apply().unwrap();
        let b = fs::read_dir(dir.join("FTA-Linux/settings-backup")).unwrap().flatten().next().unwrap();
        assert!(!b.path().join("realmlist.wtf").exists());
        assert_eq!(fs::read_to_string(dir.join("Data/enUS/realmlist.wtf")).unwrap(), "set realmlist a.b\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_values_are_refused_and_nothing_is_written() {
        let dir = scratch("invalid");
        fs::write(dir.join("WTF/Config.wtf"), "SET locale \"enUS\"\nSET realmList \"a.b\"\n").unwrap();
        let mut g = Game::load(&dir);
        g.values.realm = "bad host".into();
        assert!(g.apply().is_err());
        g.values.realm = "a.b".into();
        g.values.resolution = "huge".into();
        assert!(g.apply().is_err());
        assert_eq!(fs::read_to_string(dir.join("WTF/Config.wtf")).unwrap(), "SET locale \"enUS\"\nSET realmList \"a.b\"\n");
        assert!(!dir.join("FTA-Linux").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn locale_from_a_file_cannot_point_outside_data() {
        let dir = scratch("locale");
        fs::write(dir.join("WTF/Config.wtf"), "SET locale \"../..\"\n").unwrap();
        let wtf = Doc::load(Style::Wtf, &wtf_path(&dir));
        // falls back to the single real locale folder, which has no file yet
        assert!(realmlist_path(&dir, &wtf).is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
