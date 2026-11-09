//! The settings model behind the Settings page: loads the current values from the client's files, tracks edits,
//! and writes only what changed, with a backup of the originals.

use crate::catalog::{bool_text, parse_bool, PRESETS, SETTINGS};
use crate::cfgfile::{write_atomic, Doc, Style};
use crate::catalog::Store as Where;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Settings {
    client: PathBuf,
    /// Current values, one per catalog entry (what the checkboxes show).
    pub values: Vec<bool>,
    /// What the files say right now.
    saved: Vec<bool>,
}

fn cfg_path(client: &Path) -> PathBuf {
    client.join("WarcraftXL.cfg")
}
fn wtf_path(client: &Path) -> PathBuf {
    client.join("WTF/Config.wtf")
}

impl Settings {
    pub fn load(client: &Path) -> Settings {
        let cfg = Doc::load(Style::Cfg, &cfg_path(client));
        let wtf = Doc::load(Style::Wtf, &wtf_path(client));
        let saved: Vec<bool> = SETTINGS
            .iter()
            .map(|s| {
                let doc = if s.store == Where::Cfg { &cfg } else { &wtf };
                doc.get(s.key).map(|v| parse_bool(&v)).unwrap_or(s.default)
            })
            .collect();
        Settings { client: client.to_path_buf(), values: saved.clone(), saved }
    }

    pub fn is_dirty(&self) -> bool {
        self.values != self.saved
    }

    pub fn toggle(&mut self, index: usize) {
        if let Some(v) = self.values.get_mut(index) {
            *v = !*v;
        }
    }

    /// Loads a preset's values into the page (nothing is written until `apply`).
    pub fn load_preset(&mut self, index: usize) -> Option<&'static str> {
        let preset = PRESETS.get(index)?;
        for (key, on) in preset.values {
            if let Some(i) = SETTINGS.iter().position(|s| s.key == *key) {
                self.values[i] = *on;
            }
        }
        Some(preset.name)
    }

    /// Writes every changed setting. Returns a short message for the status line.
    pub fn apply(&mut self) -> Result<String, String> {
        if !self.is_dirty() {
            return Ok("Nothing to save.".to_string());
        }
        let cfg_file = cfg_path(&self.client);
        let wtf_file = wtf_path(&self.client);
        let mut cfg = Doc::load(Style::Cfg, &cfg_file);
        let mut wtf = Doc::load(Style::Wtf, &wtf_file);
        let (mut cfg_changed, mut wtf_changed) = (false, false);
        for (i, s) in SETTINGS.iter().enumerate() {
            if self.values[i] == self.saved[i] {
                continue;
            }
            let value = bool_text(self.values[i]);
            if s.store == Where::Cfg {
                cfg_changed |= cfg.set(s.key, value);
            } else {
                wtf_changed |= wtf.set(s.key, value);
            }
        }
        let to_write: Vec<(&Path, &Doc, &str)> = [
            (cfg_changed, cfg_file.as_path(), &cfg, "WarcraftXL.cfg"),
            (wtf_changed, wtf_file.as_path(), &wtf, "Config.wtf"),
        ]
        .into_iter()
        .filter(|(changed, ..)| *changed)
        .map(|(_, path, doc, name)| (path, doc, name))
        .collect();

        let backup = self.backup(&to_write.iter().map(|(p, ..)| *p).collect::<Vec<_>>())?;
        for (path, doc, name) in &to_write {
            write_atomic(path, &doc.to_text()).map_err(|e| format!("Could not write {name}: {e}"))?;
        }
        self.saved = self.values.clone();
        Ok(match backup {
            Some(dir) => format!("Saved. Restart the game to use the changes. Previous files kept in {}", dir.display()),
            None => "Saved. Restart the game to use the changes.".to_string(),
        })
    }

    fn backup(&self, files: &[&Path]) -> Result<Option<PathBuf>, String> {
        backup_files(&self.client, files)
    }
}

/// Copies the files about to change into `FTA-Linux/settings-backup/<unix seconds>/` and keeps the newest ten.
/// Returns the folder, or None if none of the files exist yet.
pub fn backup_files(client: &Path, files: &[&Path]) -> Result<Option<PathBuf>, String> {
    let existing: Vec<&&Path> = files.iter().filter(|p| p.exists()).collect();
    if existing.is_empty() {
        return Ok(None);
    }
    let root = client.join("FTA-Linux/settings-backup");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dir = root.join(stamp.to_string());
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create the backup folder: {e}"))?;
    for p in existing {
        let name = p.file_name().ok_or("bad file name")?;
        // The same file name can come from two folders (two realmlist.wtf); the first copy wins within a second.
        let target = dir.join(name);
        if target.exists() {
            continue;
        }
        fs::copy(p, &target).map_err(|e| format!("Could not back up {}: {e}", p.display()))?;
    }
    // Keep the newest ten; names are unix seconds, so a numeric sort is chronological.
    let mut all: Vec<u64> = fs::read_dir(&root)
        .map(|rd| rd.flatten().filter_map(|e| e.file_name().to_str()?.parse().ok()).collect())
        .unwrap_or_default();
    all.sort_unstable();
    while all.len() > 10 {
        let old = all.remove(0);
        let _ = fs::remove_dir_all(root.join(old.to_string()));
    }
    Ok(Some(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fta-launcher-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("WTF")).unwrap();
        fs::create_dir_all(dir.join("Data")).unwrap();
        fs::create_dir_all(dir.join("Interface")).unwrap();
        dir
    }

    #[test]
    fn apply_changes_only_what_changed_and_backs_up() {
        let dir = scratch("apply");
        let cfg = "# keep me\r\nWXL_GOD_RAYS=1\r\nWXL_MODERN_WATER=1\nSOMETHING_ELSE=abc\r\n";
        let wtf = "SET locale \"enUS\"\nSET gxWindow \"1\"\n";
        fs::write(dir.join("WarcraftXL.cfg"), cfg).unwrap();
        fs::write(dir.join("WTF/Config.wtf"), wtf).unwrap();

        let mut s = Settings::load(&dir);
        assert!(!s.is_dirty());
        let god = SETTINGS.iter().position(|x| x.key == "WXL_GOD_RAYS").unwrap();
        let threads = SETTINGS.iter().position(|x| x.key == "M2UseThreads").unwrap();
        assert!(s.values[god]);
        assert!(s.values[threads], "missing key uses the catalog default");
        s.toggle(god);
        s.toggle(threads);
        assert!(s.is_dirty());
        s.apply().unwrap();
        assert!(!s.is_dirty());

        assert_eq!(
            fs::read_to_string(dir.join("WarcraftXL.cfg")).unwrap(),
            "# keep me\r\nWXL_GOD_RAYS=0\r\nWXL_MODERN_WATER=1\nSOMETHING_ELSE=abc\r\n"
        );
        assert_eq!(
            fs::read_to_string(dir.join("WTF/Config.wtf")).unwrap(),
            "SET locale \"enUS\"\nSET gxWindow \"1\"\nSET M2UseThreads \"0\"\n"
        );
        // exactly one backup folder holding the previous files
        let backups: Vec<_> = fs::read_dir(dir.join("FTA-Linux/settings-backup")).unwrap().flatten().collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read_to_string(backups[0].path().join("WarcraftXL.cfg")).unwrap(), cfg);
        assert_eq!(fs::read_to_string(backups[0].path().join("Config.wtf")).unwrap(), wtf);
        // reloading shows the saved values
        let again = Settings::load(&dir);
        assert!(!again.values[god] && !again.values[threads]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn untouched_file_is_not_rewritten() {
        let dir = scratch("untouched");
        fs::write(dir.join("WarcraftXL.cfg"), "WXL_GOD_RAYS=1\n").unwrap();
        fs::write(dir.join("WTF/Config.wtf"), "SET a \"1\"\n").unwrap();
        let before = fs::metadata(dir.join("WTF/Config.wtf")).unwrap().modified().unwrap();
        let mut s = Settings::load(&dir);
        let i = SETTINGS.iter().position(|x| x.key == "WXL_GOD_RAYS").unwrap();
        s.toggle(i);
        s.apply().unwrap();
        // only the cfg changed, so Config.wtf must be neither rewritten nor backed up
        assert_eq!(fs::metadata(dir.join("WTF/Config.wtf")).unwrap().modified().unwrap(), before);
        let backup = fs::read_dir(dir.join("FTA-Linux/settings-backup")).unwrap().flatten().next().unwrap();
        assert!(!backup.path().join("Config.wtf").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn preset_loads_values_without_writing() {
        let dir = scratch("preset");
        fs::write(dir.join("WarcraftXL.cfg"), "WXL_GOD_RAYS=1\n").unwrap();
        let mut s = Settings::load(&dir);
        let name = s.load_preset(PRESETS.iter().position(|p| p.name == "Safe mode").unwrap());
        assert_eq!(name, Some("Safe mode"));
        assert!(s.is_dirty());
        assert_eq!(fs::read_to_string(dir.join("WarcraftXL.cfg")).unwrap(), "WXL_GOD_RAYS=1\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn real_settings_load_when_a_client_is_given() {
        let Ok(dir) = std::env::var("FTA_CLIENT_DIR") else { return };
        let s = Settings::load(Path::new(&dir));
        assert_eq!(s.values.len(), SETTINGS.len());
        assert!(!s.is_dirty());
    }
}
