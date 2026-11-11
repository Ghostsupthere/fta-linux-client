//! The Login tab: choosing the picture and music of the game's login screen.
//!
//! A login screen is a pack, one `login-<id>.mpq` file, kept in `Data/LoginScreens/` (or in the folder named by
//! `FTA_LOGIN_PACKS`). The game reads every `Data/patch-??.mpq` and lets the one that sorts last win, so the chosen
//! pack is made available as `Data/patch-ZZ.mpq`: a symbolic link to the pack. "Default" removes that link and
//! the game's own login screen comes back. Nothing in a pack is ever copied or edited, and a `patch-ZZ.mpq` that
//! is not a link to one of these packs (somebody's own file) is never touched.

use std::path::{Path, PathBuf};

/// Where the login-screen packs can be downloaded (one archive with all ten; extract it inside the game's Data
/// folder so the files end up in Data/LoginScreens/).
pub const PACKS_URL: &str = "https://drive.google.com/file/d/13WPuTZ77LCYdlWmD3Ia8pWvgxXBau5E-/view?usp=sharing";

/// The screens in the order the tab shows them. The first (empty id) is the game's own screen.
pub const SCREENS: &[(&str, &str)] = &[
    ("", "Default"),
    ("northrend", "Wrath of the Lich King"),
    ("cataclysm", "Cataclysm"),
    ("pandaria", "Mists of Pandaria"),
    ("warlords", "Warlords of Draenor"),
    ("legion", "Legion"),
    ("legion-alt", "Legion (alternate)"),
    ("battleforazeroth", "Battle for Azeroth"),
    ("shadowlands", "Shadowlands"),
    ("dragonflight", "Dragonflight"),
    ("warwithin", "The War Within"),
];

/// What the tab shows about one screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: &'static str,
    pub title: &'static str,
    /// The pack file is there (always true for the default screen).
    pub available: bool,
    /// This is the screen in use now.
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub entries: Vec<Entry>,
    /// `Data/patch-ZZ.mpq` exists but is not a link to one of our packs: it is left alone.
    pub foreign_file: bool,
    pub packs_dir: PathBuf,
}

/// Where the packs are looked for.
pub fn packs_dir(client: &Path) -> PathBuf {
    match std::env::var_os("FTA_LOGIN_PACKS") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => client.join("Data/LoginScreens"),
    }
}

fn link_path(client: &Path) -> PathBuf {
    client.join("Data/patch-ZZ.mpq")
}

fn pack_file(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("login-{id}.mpq"))
}

/// The id of the pack that `Data/patch-ZZ.mpq` links to: None for no link, Some(None) for something else.
fn current(client: &Path) -> Option<Option<&'static str>> {
    let link = link_path(client);
    let meta = std::fs::symlink_metadata(&link).ok()?;
    if !meta.file_type().is_symlink() {
        return Some(None);
    }
    let name = std::fs::read_link(&link).ok()?.file_name()?.to_str()?.to_string();
    Some(SCREENS.iter().skip(1).find(|(id, _)| name == format!("login-{id}.mpq")).map(|(id, _)| *id))
}

/// A pack is a real MPQ file (the magic bytes `MPQ\x1a`), not an empty or partial download.
fn looks_like_pack(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else { return false };
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic).is_ok() && &magic == b"MPQ\x1a" && f.metadata().map(|m| m.len() > 1024).unwrap_or(false)
}

pub fn state(client: &Path) -> State {
    state_in(client, &packs_dir(client))
}

fn state_in(client: &Path, dir: &Path) -> State {
    let cur = current(client);
    let entries = SCREENS
        .iter()
        .map(|(id, title)| Entry {
            id,
            title,
            available: id.is_empty() || looks_like_pack(&pack_file(dir, id)),
            active: if id.is_empty() { cur.is_none() } else { cur == Some(Some(*id)) },
        })
        .collect();
    State { entries, foreign_file: cur == Some(None), packs_dir: dir.to_path_buf() }
}

/// Makes `id` the login screen ("" for the default). Returns a message for the person.
pub fn select(client: &Path, id: &str) -> Result<String, String> {
    select_in(client, &packs_dir(client), id)
}

fn select_in(client: &Path, dir: &Path, id: &str) -> Result<String, String> {
    let title = SCREENS.iter().find(|(i, _)| *i == id).map(|(_, t)| *t).ok_or("Unknown login screen.")?;
    let link = link_path(client);
    if current(client) == Some(None) {
        return Err("Data/patch-ZZ.mpq is a file of your own, not one of these login screens. It was left alone.".into());
    }
    if id.is_empty() {
        match std::fs::remove_file(&link) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Could not remove the link: {e}")),
        }
        return Ok("The default login screen will show the next time the game starts.".into());
    }
    let pack = pack_file(dir, id);
    if !looks_like_pack(&pack) {
        return Err(format!("{title} is not installed: {} is missing.", pack.display()));
    }
    // A relative link keeps working if the client folder is moved; a pack kept elsewhere needs the full path.
    let data = client.join("Data");
    let target = match pack.strip_prefix(&data) {
        Ok(rel) => rel.to_path_buf(),
        Err(_) => pack.canonicalize().map_err(|e| format!("Could not find the pack: {e}"))?,
    };
    // Written beside the link and renamed over it, so the game never sees a missing or half-made link.
    let tmp = data.join(format!(".patch-ZZ.mpq.tmp-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    std::os::unix::fs::symlink(&target, &tmp).map_err(|e| format!("Could not create the link: {e}"))?;
    if let Err(e) = std::fs::rename(&tmp, &link) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("Could not switch the login screen: {e}"));
    }
    Ok(format!("{title} will show the next time the game starts."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fta-launcher-login-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Data/LoginScreens")).unwrap();
        fs::create_dir_all(dir.join("Interface")).unwrap();
        dir
    }

    fn fake_pack(client: &Path, id: &str) {
        let mut bytes = b"MPQ\x1a".to_vec();
        bytes.resize(4096, 0);
        fs::write(pack_file(&client.join("Data/LoginScreens"), id), bytes).unwrap();
    }

    #[test]
    fn the_download_link_is_one_the_launcher_may_open() {
        assert!(PACKS_URL.starts_with("https://drive.google.com/file/d/"));
        assert!(crate::ALLOWED_LINKS.contains(&PACKS_URL));
    }

    #[test]
    fn nothing_installed_means_only_the_default() {
        let c = scratch("none");
        let s = state(&c);
        assert!(s.entries[0].active && s.entries[0].available);
        assert!(s.entries[1..].iter().all(|e| !e.available && !e.active));
        assert!(!s.foreign_file);
    }

    #[test]
    fn selecting_links_the_pack_and_default_removes_the_link() {
        let c = scratch("select");
        fake_pack(&c, "legion");
        select(&c, "legion").unwrap();
        let s = state(&c);
        assert!(s.entries.iter().find(|e| e.id == "legion").unwrap().active);
        assert!(!s.entries[0].active);
        // a relative link into the client's own Data folder
        assert_eq!(fs::read_link(link_path(&c)).unwrap(), PathBuf::from("LoginScreens/login-legion.mpq"));
        assert!(link_path(&c).exists(), "the link resolves");
        // switching replaces it in place
        fake_pack(&c, "warwithin");
        select(&c, "warwithin").unwrap();
        assert!(state(&c).entries.iter().find(|e| e.id == "warwithin").unwrap().active);
        select(&c, "").unwrap();
        assert!(fs::symlink_metadata(link_path(&c)).is_err());
        assert!(state(&c).entries[0].active);
        // selecting the default again is fine
        select(&c, "").unwrap();
    }

    #[test]
    fn a_missing_or_partial_pack_is_refused() {
        let c = scratch("missing");
        assert!(select(&c, "legion").unwrap_err().contains("not installed"));
        fs::write(pack_file(&c.join("Data/LoginScreens"), "legion"), b"not a pack").unwrap();
        assert!(select(&c, "legion").is_err());
        assert!(!state(&c).entries.iter().find(|e| e.id == "legion").unwrap().available);
        assert!(select(&c, "nope").is_err());
    }

    #[test]
    fn a_file_of_the_users_own_is_never_touched() {
        let c = scratch("foreign");
        fake_pack(&c, "legion");
        fs::write(link_path(&c), b"somebody's own patch").unwrap();
        assert!(state(&c).foreign_file);
        assert!(select(&c, "legion").is_err());
        assert!(select(&c, "").is_err());
        assert_eq!(fs::read(link_path(&c)).unwrap(), b"somebody's own patch");
    }

    #[test]
    fn a_pack_kept_outside_the_client_is_linked_by_full_path() {
        let c = scratch("outside");
        let elsewhere = scratch("elsewhere-packs");
        let mut bytes = b"MPQ\x1a".to_vec();
        bytes.resize(4096, 0);
        fs::write(elsewhere.join("login-legion.mpq"), bytes).unwrap();
        select_in(&c, &elsewhere, "legion").unwrap();
        assert!(fs::read_link(link_path(&c)).unwrap().is_absolute());
        assert!(state_in(&c, &elsewhere).entries.iter().find(|e| e.id == "legion").unwrap().active);
    }
}
