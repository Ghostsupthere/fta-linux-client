//! The sentinel on the Play page says one of these, picked at random when the launcher starts and again each time
//! it is clicked. Good-natured, short (a bubble holds about three lines) and free of anything a stranger could
//! take the wrong way.
//!
//! Anyone can add their own lines: put them in `quips.txt` in the launcher's config folder
//! (`~/.config/fta-linux-launcher/quips.txt`), one per line. Empty lines and lines starting with `#` are ignored.
//! If the first line is `# replace`, only the lines in the file are used instead of the built-in ones.

pub const QUIPS: &[&str] = &[
    "Somewhere a Windows PC is \"getting things ready\". You're already playing.",
    "Ads in the Start menu? Over here a menu is just a menu.",
    "I use Arch, btw.",
    "Windows needs four restarts to install a font. We just play.",
    "sudo make me a sandwich",
    "Windows: \"Are you sure you want to run this?\" Linux: \"It's your computer.\"",
    "Most of the world's servers run Linux. This one runs a portal.",
    "Paying for an operating system that shows you ads? Couldn't be me.",
    "Every Windows update is a surprise. Every Linux update is a choice.",
    "Be honest: you open the terminal just to look cool.",
    "The Dark Portal is open. So is the source code.",
    "Is that a \"Activate Windows\" watermark? ",
    "Free as in freedom. Also free as in free.",
    "Blue screen? Never heard of her. Green portal, though.",
    "Why guess what your PC is doing when you can read the source?",
    "Telemetry? The only thing reporting in here is the changelog.",
    "Dual-booting? We don't talk about that. Press Play.",
    "A stone sentinel runs on 3 MB of RAM. I checked. Okay, I made that up.",
    "You said \"one more pull\". Windows said \"restarting in 5 minutes\".",
    "The Lich King has nothing on a Windows Update stuck at 99%.",
    "A mage's portal is free. So is this operating system.",
    "Gnomish engineering can't fix Windows either.",
];

/// Longest line the bubble holds, and the most lines read from the file (so a huge file cannot slow the start).
const MAX_LEN: usize = 110;
const MAX_LINES: usize = 500;

/// The lines to use: the built-in ones plus the ones in `<config_dir>/quips.txt`, or only the latter when the
/// file asks for it (and has at least one usable line).
pub fn load(config_dir: Option<&std::path::Path>) -> Vec<String> {
    let builtin = || QUIPS.iter().map(|q| q.to_string()).collect::<Vec<_>>();
    let Some(text) = config_dir.and_then(|d| std::fs::read_to_string(d.join("quips.txt")).ok()) else {
        return builtin();
    };
    let (extra, replace) = parse(&text);
    if replace && !extra.is_empty() {
        extra
    } else {
        let mut all = builtin();
        all.extend(extra);
        all
    }
}

/// The usable lines of a quips file, and whether it asks to replace the built-in ones.
pub fn parse(text: &str) -> (Vec<String>, bool) {
    let replace = text.lines().map(str::trim).find(|l| !l.is_empty()).is_some_and(|l| l.eq_ignore_ascii_case("# replace"));
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter(|l| l.chars().count() <= MAX_LEN)
        .take(MAX_LINES)
        .map(str::to_string)
        .collect();
    (lines, replace)
}

/// An index below `len` different from `prev`, from the clock (this needs no random-number crate).
pub fn pick(prev: Option<usize>, len: usize) -> usize {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize ^ (d.as_secs() as usize).wrapping_mul(2654435761))
        .unwrap_or(0);
    if len < 2 {
        return 0;
    }
    match prev {
        Some(p) => (p + 1 + nanos % (len - 1)) % len,
        None => nanos % len,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_are_in_range_and_never_repeat_the_previous_line() {
        let mut prev = None;
        for _ in 0..500 {
            let i = pick(prev, QUIPS.len());
            assert!(i < QUIPS.len());
            assert_ne!(Some(i), prev);
            prev = Some(i);
        }
        assert_eq!(pick(Some(0), 1), 0);
    }

    #[test]
    fn built_in_lines_fit_a_three_line_bubble() {
        for q in QUIPS {
            assert!(q.chars().count() <= MAX_LEN, "too long for the bubble: {q}");
        }
    }

    #[test]
    fn file_lines_are_trimmed_and_filtered() {
        let long = "x".repeat(MAX_LEN + 1);
        let text = format!("  Hello there  \n\n# a comment\n{long}\nSecond line\n");
        let (lines, replace) = parse(&text);
        assert_eq!(lines, vec!["Hello there".to_string(), "Second line".to_string()]);
        assert!(!replace);
    }

    #[test]
    fn a_replace_marker_on_the_first_line_is_honoured() {
        let (lines, replace) = parse("\n# Replace\nOnly mine\n");
        assert!(replace);
        assert_eq!(lines, vec!["Only mine".to_string()]);
        // the marker lower down is just a comment
        let (_, replace) = parse("Only mine\n# replace\n");
        assert!(!replace);
    }

    #[test]
    fn load_adds_to_or_replaces_the_built_in_lines() {
        let dir = std::env::temp_dir().join(format!("fta-quips-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(load(Some(&dir)).len(), QUIPS.len()); // no file yet
        std::fs::write(dir.join("quips.txt"), "One\nTwo\n").unwrap();
        assert_eq!(load(Some(&dir)).len(), QUIPS.len() + 2);
        std::fs::write(dir.join("quips.txt"), "# replace\nOnly\n").unwrap();
        assert_eq!(load(Some(&dir)), vec!["Only".to_string()]);
        std::fs::write(dir.join("quips.txt"), "# replace\n# nothing usable\n").unwrap();
        assert_eq!(load(Some(&dir)).len(), QUIPS.len()); // an empty replace file falls back
        let _ = std::fs::remove_dir_all(&dir);
    }
}
