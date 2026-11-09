//! Line-preserving editors for the two settings files the launcher manages.
//!
//! * `WarcraftXL.cfg`: `KEY=VALUE` lines, `#` or `;` comments. The client reads it with the last occurrence of
//!   a key winning, so that is the one we read and replace.
//! * `WTF/Config.wtf`: `SET name "value"` lines.
//!
//! Both files are shared with the Windows client and with the game itself, so everything we do not manage is
//! kept byte for byte: comments, ordering, unknown keys, blank lines and each line's own line ending (the real
//! files mix Windows and Unix endings).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// `KEY=VALUE`
    Cfg,
    /// `SET name "value"`
    Wtf,
}

struct Line {
    text: String,
    /// "\n", "\r\n" or "" (the last line of a file without a final newline).
    eol: String,
}

/// An in-memory copy of one settings file.
pub struct Doc {
    style: Style,
    lines: Vec<Line>,
}

/// Parses one line into (key, value) if it is a setting line.
fn parse_line(style: Style, line: &str) -> Option<(String, String)> {
    let t = line.trim();
    if t.is_empty() {
        return None;
    }
    match style {
        Style::Cfg => {
            if t.starts_with('#') || t.starts_with(';') {
                return None;
            }
            let (k, v) = t.split_once('=')?;
            let k = k.trim();
            if k.is_empty() {
                return None;
            }
            Some((k.to_string(), v.trim().to_string()))
        }
        Style::Wtf => {
            let rest = t.strip_prefix("SET ").or_else(|| t.strip_prefix("set "))?;
            let rest = rest.trim_start();
            let (k, v) = match rest.split_once(char::is_whitespace) {
                Some((k, v)) => (k, v.trim()),
                None => (rest, ""),
            };
            let v = v.strip_prefix('"').unwrap_or(v);
            let v = v.strip_suffix('"').unwrap_or(v);
            Some((k.to_string(), v.to_string()))
        }
    }
}

impl Doc {
    pub fn parse(style: Style, text: &str) -> Doc {
        let mut lines = Vec::new();
        for raw in text.split_inclusive('\n') {
            let (body, eol) = if let Some(b) = raw.strip_suffix("\r\n") {
                (b, "\r\n")
            } else if let Some(b) = raw.strip_suffix('\n') {
                (b, "\n")
            } else {
                (raw, "")
            };
            lines.push(Line { text: body.to_string(), eol: eol.to_string() });
        }
        Doc { style, lines }
    }

    pub fn load(style: Style, path: &Path) -> Doc {
        match fs::read(path) {
            Ok(bytes) => Doc::parse(style, &String::from_utf8_lossy(&bytes)),
            Err(_) => Doc::parse(style, ""),
        }
    }

    /// The value the game would see: the last occurrence wins. Keys compare case-sensitively for the cfg file and
    /// case-insensitively for Config.wtf (the game's variable names are not case sensitive).
    pub fn get(&self, key: &str) -> Option<String> {
        let mut found = None;
        for line in &self.lines {
            if let Some((k, v)) = parse_line(self.style, &line.text) {
                if self.same_key(&k, key) {
                    found = Some(v);
                }
            }
        }
        found
    }

    fn same_key(&self, a: &str, b: &str) -> bool {
        match self.style {
            Style::Cfg => a == b,
            Style::Wtf => a.eq_ignore_ascii_case(b),
        }
    }

    fn render(&self, key: &str, value: &str) -> String {
        match self.style {
            Style::Cfg => format!("{key}={value}"),
            Style::Wtf => format!("SET {key} \"{value}\""),
        }
    }

    /// The ending new lines should use: that of the last line that has one, else Unix.
    fn append_eol(&self) -> String {
        self.lines.iter().rev().find(|l| !l.eol.is_empty()).map(|l| l.eol.clone()).unwrap_or_else(|| "\n".to_string())
    }

    /// Sets `key` to `value`: the last existing occurrence is rewritten in place (keeping its line ending, so the
    /// game's last-wins rule keeps working), otherwise the setting is appended. Returns true if the document changed.
    pub fn set(&mut self, key: &str, value: &str) -> bool {
        let mut last = None;
        for (i, line) in self.lines.iter().enumerate() {
            if let Some((k, v)) = parse_line(self.style, &line.text) {
                if self.same_key(&k, key) {
                    last = Some((i, v));
                }
            }
        }
        match last {
            Some((_, current)) if current == value => false,
            Some((i, _)) => {
                self.lines[i].text = self.render(key, value);
                true
            }
            None => {
                let eol = self.append_eol();
                // A file whose last line has no newline needs one before the new line, using the same ending.
                if let Some(prev) = self.lines.last_mut() {
                    if prev.eol.is_empty() {
                        prev.eol = eol.clone();
                    }
                }
                let text = self.render(key, value);
                self.lines.push(Line { text, eol });
                true
            }
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for l in &self.lines {
            out.push_str(&l.text);
            out.push_str(&l.eol);
        }
        out
    }
}

/// Writes `text` to `path` through a temporary file in the same directory and a rename, so a crash or a full disk
/// never leaves a half-written settings file. The original's permissions are kept.
pub fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("settings");
    let tmp: PathBuf = dir.join(format!(".{name}.fta-launcher.tmp"));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
    }
    if let Ok(meta) = fs::metadata(path) {
        let _ = fs::set_permissions(&tmp, meta.permissions());
    }
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_roundtrip_keeps_everything_else() {
        let src = "# header\r\nWXL_A=1\r\n\r\n; note\r\nWXL_B = 0\r\nUNKNOWN=keep me\r\n";
        let mut d = Doc::parse(Style::Cfg, src);
        assert!(!d.set("WXL_A", "1"));
        assert!(d.set("WXL_B", "1"));
        assert!(d.set("WXL_NEW", "0"));
        assert_eq!(d.to_text(), "# header\r\nWXL_A=1\r\n\r\n; note\r\nWXL_B=1\r\nUNKNOWN=keep me\r\nWXL_NEW=0\r\n");
    }

    #[test]
    fn mixed_line_endings_are_each_kept() {
        let src = "A=1\r\nB=1\nC=1\r\n";
        let mut d = Doc::parse(Style::Cfg, src);
        assert_eq!(d.to_text(), src);
        assert!(d.set("B", "0"));
        assert_eq!(d.to_text(), "A=1\r\nB=0\nC=1\r\n");
    }

    #[test]
    fn last_occurrence_wins_and_is_the_one_replaced() {
        let mut d = Doc::parse(Style::Cfg, "K=1\nK=0\n");
        assert_eq!(d.get("K").as_deref(), Some("0"));
        assert!(d.set("K", "1"));
        assert_eq!(d.to_text(), "K=1\nK=1\n");
    }

    #[test]
    fn wtf_set_lines() {
        let mut d = Doc::parse(Style::Wtf, "SET locale \"enUS\"\nSET gxWindow \"1\"\n");
        assert_eq!(d.get("gxwindow").as_deref(), Some("1"));
        assert!(d.set("gxWindow", "0"));
        assert!(d.set("M2UseThreads", "0"));
        assert_eq!(d.to_text(), "SET locale \"enUS\"\nSET gxWindow \"0\"\nSET M2UseThreads \"0\"\n");
    }

    #[test]
    fn no_trailing_newline_is_preserved_until_something_is_appended() {
        let mut d = Doc::parse(Style::Cfg, "A=1");
        assert!(d.set("A", "0"));
        assert_eq!(d.to_text(), "A=0");
        assert!(d.set("B", "1"));
        assert_eq!(d.to_text(), "A=0\nB=1\n");
    }

    #[test]
    fn comment_lines_are_never_matched_as_keys() {
        let d = Doc::parse(Style::Cfg, "# A=1\n; A=1\n");
        assert_eq!(d.get("A"), None);
    }

    /// Set FTA_CLIENT_DIR to a real client folder to check that parsing and rendering is lossless on its files.
    #[test]
    fn real_files_roundtrip_unchanged() {
        let Ok(dir) = std::env::var("FTA_CLIENT_DIR") else { return };
        for (style, rel) in [(Style::Cfg, "WarcraftXL.cfg"), (Style::Wtf, "WTF/Config.wtf")] {
            let path = std::path::Path::new(&dir).join(rel);
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let doc = Doc::parse(style, &text);
            assert_eq!(doc.to_text(), text, "{rel} changed by a no-op roundtrip");
        }
    }
}
