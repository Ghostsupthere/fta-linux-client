//! The Extras tab: the optional pieces of the install (the Murloc race patch, the RealmDesk and Body Shape addons, the addon fixes and
//! the character texture setting). The work is done by `FTA-Linux/optional.sh`, the same script a person can run by
//! hand; this module only asks it for the state of each piece and runs `remove` or `restore` for one of them.
//! Nothing is ever deleted by it: a piece that is taken out is moved aside by the script.

use std::path::Path;
use std::process::Command;

/// The pieces the script knows, in the order the tab shows them. Only these ids are ever passed to it.
pub const IDS: &[&str] = &["murloc", "realmdesk", "bodyshape", "fixes", "texture"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// In the client and in use.
    Installed,
    /// Taken out here, or left out at install time: updates will not add it again.
    Removed,
    /// Never added to this client.
    Absent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    pub id: String,
    pub state: State,
    pub title: String,
    pub detail: String,
}

impl Piece {
    /// The verb the button of this piece runs, if there is one: installed pieces can be taken out, removed ones put back.
    pub fn action(&self) -> Option<&'static str> {
        match self.state {
            State::Installed => Some("remove"),
            State::Removed => Some("restore"),
            State::Absent => None,
        }
    }
}

fn script(client: &Path) -> std::path::PathBuf {
    client.join("FTA-Linux/optional.sh")
}

/// Parses the script's `list` output: id, state, title and description separated by tabs, one piece per line.
pub fn parse_list(text: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for line in text.lines() {
        let mut f = line.split('\t');
        let (Some(id), Some(state), Some(title), Some(detail)) = (f.next(), f.next(), f.next(), f.next()) else { continue };
        if !IDS.contains(&id) {
            continue;
        }
        let state = match state {
            "installed" => State::Installed,
            "removed" => State::Removed,
            "absent" => State::Absent,
            _ => continue,
        };
        pieces.push(Piece { id: id.to_string(), state, title: title.to_string(), detail: detail.to_string() });
    }
    pieces
}

/// The state of every piece, or why it cannot be read (an install older than this launcher has no script).
pub fn list(client: &Path) -> Result<Vec<Piece>, String> {
    let script = script(client);
    if !script.is_file() {
        return Err("This install has no FTA-Linux/optional.sh yet. Update to the newest release to use this tab.".into());
    }
    let out = Command::new("bash").arg(&script).arg("list").output().map_err(|e| format!("Could not run optional.sh: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(parse_list(&String::from_utf8_lossy(&out.stdout)))
}

/// Runs `remove` or `restore` for one piece. The text is what the script printed, trimmed to what fits a status line.
pub fn act(client: &Path, verb: &str, id: &str) -> String {
    if !IDS.contains(&id) || !matches!(verb, "remove" | "restore") {
        return "That is not something this tab can do.".into();
    }
    let script = script(client);
    if !script.is_file() {
        return "This install has no FTA-Linux/optional.sh yet. Update to the newest release first.".into();
    }
    match Command::new("bash").arg(&script).arg(verb).arg(id).output() {
        Ok(out) => {
            let mut text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if !err.is_empty() {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&err);
            }
            // The status line is short: first the outcome (the script's first sentence of each line), joined.
            let one_line = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
            if one_line.is_empty() {
                if out.status.success() { "Done.".into() } else { "That did not work.".into() }
            } else {
                one_line
            }
        }
        Err(e) => format!("Could not run optional.sh: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_script_output() {
        let text = "murloc\tinstalled\tMurloc race\tA race.\nrealmdesk\tremoved\tRealmDesk\tTickets.\nfixes\tabsent\tAddon fixes\tFixes.\n";
        let p = parse_list(text);
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].state, State::Installed);
        assert_eq!(p[0].action(), Some("remove"));
        assert_eq!(p[1].action(), Some("restore"));
        assert_eq!(p[2].action(), None);
    }

    #[test]
    fn ignores_lines_it_does_not_know() {
        let text = "nope\tinstalled\tX\tY\nmurloc\tweird\tMurloc\tZ\nmurloc\tinstalled\n\ntexture\tabsent\tT\tD\n";
        let p = parse_list(text);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].id, "texture");
    }

    #[test]
    fn refuses_unknown_ids_and_verbs() {
        let dir = Path::new("/nonexistent");
        assert!(act(dir, "remove", "../x").contains("not something"));
        assert!(act(dir, "delete", "murloc").contains("not something"));
    }

    /// Runs the real script from the overlay against a throwaway client folder: take the Murloc patch out and put it back.
    #[test]
    fn takes_a_piece_out_and_puts_it_back() {
        let dir = std::env::temp_dir().join(format!("fta-optional-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("FTA-Linux")).unwrap();
        std::fs::create_dir_all(dir.join("Data")).unwrap();
        std::fs::create_dir_all(dir.join("Interface")).unwrap();
        std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlay/FTA-Linux/optional.sh"), dir.join("FTA-Linux/optional.sh")).unwrap();
        std::fs::write(dir.join("Data/patch-ZD.mpq"), b"patch").unwrap();

        let state = |dir: &Path| list(dir).unwrap().into_iter().find(|p| p.id == "murloc").unwrap().state;
        assert_eq!(state(&dir), State::Installed);
        act(&dir, "remove", "murloc");
        assert_eq!(state(&dir), State::Removed);
        assert!(!dir.join("Data/patch-ZD.mpq").exists());
        assert!(dir.join("FTA-Linux/no-murloc").exists());
        act(&dir, "restore", "murloc");
        assert_eq!(state(&dir), State::Installed);
        assert_eq!(std::fs::read(dir.join("Data/patch-ZD.mpq")).unwrap(), b"patch");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
