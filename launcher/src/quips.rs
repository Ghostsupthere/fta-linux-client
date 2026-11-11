//! The sentinel on the Play page says one of these, picked at random when the launcher starts and again each time
//! it is clicked. Good-natured, short (a bubble holds about three lines) and free of anything a stranger could
//! take the wrong way.
//!
//! Anyone can add their own lines: put them in `quips.txt` in the launcher's config folder
//! (`~/.config/fta-linux-launcher/quips.txt`), one per line. Empty lines and lines starting with `#` are ignored.
//! If the first line is `# replace`, only the lines in the file are used instead of the built-in ones.

pub const QUIPS: &[&str] = &[
    "Ads in the Start menu? And they pay for that?",
    "I use Arch, btw.",
    "The Stonemasons rebuilt Stormwind for free. Microsoft would've charged for each window.",
    "sudo make me a sandwich",
    "Most of the world's servers run Linux. This one runs a portal.",
    "Paying for an operating system that shows you ads? Couldn't be me.",
    "Every Windows update is a surprise. Every Linux update is a choice.",
    "Gazlowe looked at Windows 11 and said, 'Even I wouldn't charge for that.'",
    "The Dark Portal is open. So is the source code.",
    "Is that an \"Activate Windows\" watermark?",
    "The Dark Iron dwarves have been mining your data. Microsoft wants a cut.",
    "Free as in freedom. Also free as in free.",
    "The Argent Dawn protects Azeroth from the Scourge. Who protects us from Copilot?",
    "Gallywix would've loved subscription-based operating systems.",
    "Blue screen? Never heard of her.",
    "The furbolgs of Felwood are corrupted. Their desktops probably run Windows.",
    "The Booty Bay auction house has fewer hidden fees than Microsoft.",
    "Telemetry? The only thing reporting in here is the changelog.",
    "A stone sentinel runs on 3 MB of RAM. I checked. Okay, I made that up.",
    "You said \"one more pull\". Windows said \"restarting in 5 minutes\".",
    "The goblins of Ratchet will repair anything. Except a Windows registry.",
    "VanCleef wanted fair wages. We just want an OS without ads.",
    "The Burning Legion uses Windows.",
    "Mankrik found his wife before Windows finished updating.",
    "The Lich King has nothing on a Windows Update stuck at 99%.",
    "A mage's portal is free. So is this operating system.",
    "sudo apt install [Thunderfury, Blessed Blade of the Windseeker]",
    "sudo mount /dev/azeroth /mnt/adventure",
    "sudo usermod -aG wheel Hogger",
    "grep -r 'Mankrik' /the-barrens | grep -v 'wife'",
    "The Troggs invaded Gnomeregan. Windows calls that a feature update.",
    "The Darkmoon Faire sells mystery boxes. Windows calls them optional updates.",
    "systemctl status dark-portal.service: active (swirling)",
    "You no take candle. Microsoft no take data.",
    "Work work. Compile compile.",
    "King's Honor, friend. Root access denied.",
    "That'll be $14.99 + $30.00 tip for if you want blue blood elves.",
    "Do you really think Blizzard won't add the WoW Token to Forever? Surely they won't...right?",
    "You can always buy a WoW Token when you're low on gold...oh...wrong version, sorry.",
    "You can always send yourself $14.99 if you're feeling nostalgic.",
    "ERROR 404: Mankrik's wife not found.",
    "Permission denied. Requires reputation: Exalted.",
    "Gnomish engineering can't fix Windows either.",
    "Lok'tar Ogar. sudo pacman -Syu."
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
