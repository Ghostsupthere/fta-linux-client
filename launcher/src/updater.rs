//! Update checks and installs. The latest GitHub release of this project is looked up, its installer is
//! downloaded, checked against the checksum GitHub publishes for it and run with `--update`, exactly as a person
//! running the `.run` file by hand would (the manual route stays available).

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const REPO: &str = "Ghostsupthere/fta-linux-client";
const INSTALLER_NAME: &str = "FTA-Linux-Installer.run";
const SUMS_NAME: &str = "FTA-Linux-SHA256SUMS.txt";
/// Downloads are only ever taken from this project's own release pages.
const DOWNLOAD_PREFIX: &str = "https://github.com/Ghostsupthere/fta-linux-client/releases/download/";

#[derive(Debug, Clone)]
pub struct Release {
    pub tag: String,
    pub installer_url: String,
    pub size: u64,
    /// Hex sha256 of the installer, from GitHub's digest field or the release's checksum file.
    pub sha256: Option<String>,
    pub sums_url: Option<String>,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    published_at: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    digest: Option<String>,
    browser_download_url: String,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(600)))
        .timeout_connect(Some(Duration::from_secs(10)))
        .user_agent(format!("fta-linux-launcher/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn parse_release(api: ApiRelease) -> Result<Release, String> {
    if api.draft || api.prerelease {
        return Err("The newest release is not a final one.".into());
    }
    let installer = api
        .assets
        .iter()
        .find(|a| a.name == INSTALLER_NAME)
        .ok_or("The newest release has no installer file.")?;
    if !installer.browser_download_url.starts_with(DOWNLOAD_PREFIX) {
        return Err("The installer link does not point at this project's releases.".into());
    }
    let sha256 = installer
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(|h| h.to_ascii_lowercase());
    let sums_url = api
        .assets
        .iter()
        .find(|a| a.name == SUMS_NAME && a.browser_download_url.starts_with(DOWNLOAD_PREFIX))
        .map(|a| a.browser_download_url.clone());
    Ok(Release {
        tag: api.tag_name,
        installer_url: installer.browser_download_url.clone(),
        size: installer.size,
        sha256,
        sums_url,
    })
}

/// One release as the changelog shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub tag: String,
    /// The part of the release name after `vX.Y.Z: `.
    pub title: String,
    /// `2026-11-08`
    pub date: String,
    pub blocks: Vec<Block>,
}

/// A paragraph of the notes: an optional bold lead-in, the text after it and any bullet points.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub lead: String,
    pub text: String,
    pub bullets: Vec<String>,
}

pub struct Feed {
    pub latest: Release,
    pub notes: Vec<Note>,
}

const NOTES_SHOWN: usize = 3;

/// Looks up the newest releases: the update candidate and the changelog come from one request.
pub fn fetch_feed() -> Result<Feed, String> {
    #[cfg(feature = "dev")]
    if let (Ok(tag), Ok(path)) = (std::env::var("FTA_LAUNCHER_FAKE_LATEST"), std::env::var("FTA_LAUNCHER_LOCAL_INSTALLER")) {
        let bytes = std::fs::read(&path).map_err(|e| format!("test installer: {e}"))?;
        let latest = Release {
            tag: tag.clone(),
            installer_url: format!("local:{path}"),
            size: bytes.len() as u64,
            sha256: Some(hex(&Sha256::digest(&bytes))),
            sums_url: None,
        };
        let notes = vec![Note { tag, title: "test release".into(), date: "2026-01-01".into(), blocks: vec![Block { lead: "A test.".into(), text: "Served from a local file.".into(), bullets: vec![] }] }];
        return Ok(Feed { latest, notes });
    }
    let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=8");
    let mut resp = agent()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;
    let all: Vec<ApiRelease> = resp.body_mut().read_json().map_err(|e| format!("Unexpected answer from GitHub: {e}"))?;
    feed_from(all)
}

fn feed_from(all: Vec<ApiRelease>) -> Result<Feed, String> {
    let finals: Vec<ApiRelease> = all.into_iter().filter(|r| !r.draft && !r.prerelease).collect();
    let notes: Vec<Note> = finals.iter().take(NOTES_SHOWN).map(note_of).collect();
    let first = finals.into_iter().next().ok_or("There is no release yet.")?;
    Ok(Feed { latest: parse_release(first)?, notes })
}

fn note_of(r: &ApiRelease) -> Note {
    let title = r.name.strip_prefix(&r.tag_name).map(|t| t.trim_start_matches([':', ' ', '-']).trim().to_string()).unwrap_or_else(|| r.name.clone());
    Note { tag: r.tag_name.clone(), title, date: r.published_at.chars().take(10).collect(), blocks: parse_notes(&r.body) }
}

/// Drops Markdown that a plain text view cannot show: emphasis, code ticks, links (keeping their text) and HTML tags.
fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find('[') {
        // [label](url) keeps the label
        let tail = &rest[i + 1..];
        if let Some(close) = tail.find("](") {
            if let Some(end) = tail[close + 2..].find(')') {
                out.push_str(&rest[..i]);
                out.push_str(&tail[..close]);
                rest = &tail[close + 2 + end + 1..];
                continue;
            }
        }
        out.push_str(&rest[..=i]);
        rest = tail;
    }
    out.push_str(rest);
    let mut clean = String::new();
    let mut in_tag = false;
    for c in out.replace("**", "").replace("__", "").replace('`', "").chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => clean.push(c),
            _ => {}
        }
    }
    clean.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn cap(text: String, max: usize) -> String {
    if text.chars().count() <= max {
        return text;
    }
    let cut: String = text.chars().take(max - 3).collect();
    // end at the last full word
    let cut = cut.rsplit_once(' ').map(|(a, _)| a.to_string()).unwrap_or(cut);
    format!("{}...", cut.trim_end_matches([',', ';', ':', ' ']))
}

/// The user-facing part of the notes, as blocks. It stops at the install line or the `<details>` section, which
/// are for people reading the release page. Notes are written as short paragraphs that start with a bold lead-in.
pub fn parse_notes(body: &str) -> Vec<Block> {
    let body = body.replace("\r\n", "\n");
    let mut kept = String::new();
    for line in body.lines() {
        let t = line.trim_start();
        if t.starts_with("<details") || t.starts_with("**Install") || t.starts_with("Install /") {
            break;
        }
        kept.push_str(line);
        kept.push('\n');
    }
    let mut blocks = Vec::new();
    for para in kept.split("\n\n") {
        let mut text_lines: Vec<&str> = Vec::new();
        let mut bullets: Vec<String> = Vec::new();
        for line in para.lines() {
            let t = line.trim();
            if let Some(b) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
                bullets.push(cap(plain(b), 130));
            } else if !t.is_empty() && !t.starts_with('#') {
                text_lines.push(t);
            }
        }
        let joined = text_lines.join(" ");
        let (lead, rest) = match joined.strip_prefix("**").and_then(|r| r.split_once("**")) {
            Some((lead, rest)) => (plain(lead), rest.to_string()),
            None => (String::new(), joined),
        };
        let lead = lead.trim_end_matches('.').to_string();
        let text = cap(plain(&rest), 230);
        if lead.is_empty() && text.is_empty() && bullets.is_empty() {
            continue;
        }
        let extra = bullets.len().saturating_sub(5);
        bullets.truncate(5);
        if extra > 0 {
            bullets.push(format!("...and {extra} more"));
        }
        blocks.push(Block { lead, text, bullets });
    }
    blocks
}

/// `v0.2.10` -> (0, 2, 10). Anything that is not dotted numbers (after an optional `v`) is not a version.
pub fn parse_version(tag: &str) -> Option<Vec<u64>> {
    let t = tag.trim().trim_start_matches(['v', 'V']);
    if t.is_empty() {
        return None;
    }
    t.split('.').map(|p| p.parse::<u64>().ok()).collect()
}

/// True when `latest` is a higher version than `installed`. An installed version that cannot be read counts as
/// older (the person gets offered the update once, and the installer then records the version).
pub fn is_newer(latest: &str, installed: Option<&str>) -> bool {
    let Some(latest) = parse_version(latest) else { return false };
    match installed.and_then(parse_version) {
        Some(mut have) => {
            let mut want = latest;
            let n = have.len().max(want.len());
            have.resize(n, 0);
            want.resize(n, 0);
            want > have
        }
        None => true,
    }
}

/// The release tag the installer recorded (`FTA-Linux/RELEASE`), if this install has one.
pub fn installed_tag(client: &Path) -> Option<String> {
    let text = std::fs::read_to_string(client.join("FTA-Linux/RELEASE")).ok()?;
    let tag = text.lines().next()?.trim().to_string();
    parse_version(&tag).map(|_| tag)
}

fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("fta-linux-launcher")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The checksum for the installer: GitHub's digest if the release has one, else the line in the checksum file.
fn expected_sha(rel: &Release) -> Result<String, String> {
    if let Some(h) = &rel.sha256 {
        return Ok(h.clone());
    }
    let url = rel.sums_url.as_ref().ok_or("This release has no checksum to verify the download with.")?;
    let mut resp = agent().get(url).call().map_err(|e| format!("Could not fetch the checksum list: {e}"))?;
    let text = resp.body_mut().read_to_string().map_err(|e| format!("Could not read the checksum list: {e}"))?;
    text.lines()
        .filter_map(|l| {
            let (hash, name) = l.split_once(char::is_whitespace)?;
            (name.trim().trim_start_matches('*') == INSTALLER_NAME && hash.len() == 64).then(|| hash.to_ascii_lowercase())
        })
        .next()
        .ok_or_else(|| "The checksum list does not name the installer.".to_string())
}

fn open_source(rel: &Release) -> Result<(Box<dyn Read>, u64), String> {
    #[cfg(feature = "dev")]
    if let Some(path) = rel.installer_url.strip_prefix("local:") {
        let f = std::fs::File::open(path).map_err(|e| format!("test installer: {e}"))?;
        return Ok((Box::new(f), rel.size));
    }
    let resp = agent().get(&rel.installer_url).call().map_err(|e| format!("Download failed: {e}"))?;
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(rel.size);
    Ok((Box::new(resp.into_body().into_reader()), total))
}

/// Downloads the installer into the cache folder and verifies it. `progress(done, total)` is called as data
/// arrives; setting `cancel` stops the download and removes the partial file.
pub fn download(rel: &Release, cancel: &AtomicBool, mut progress: impl FnMut(u64, u64)) -> Result<PathBuf, String> {
    let expected = expected_sha(rel)?;
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let part = dir.join(format!("{INSTALLER_NAME}.part"));
    let done_path = dir.join(INSTALLER_NAME);
    let _ = std::fs::remove_file(&part);
    let _ = std::fs::remove_file(&done_path);

    let (mut reader, total) = open_source(rel)?;
    let result = (|| -> Result<(), String> {
        let mut file = std::fs::File::create(&part).map_err(|e| format!("Could not write {}: {e}", part.display()))?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 64 * 1024];
        let mut done = 0u64;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Cancelled.".into());
            }
            let n = reader.read(&mut buf).map_err(|e| format!("Download interrupted: {e}"))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            file.write_all(&buf[..n]).map_err(|e| format!("Could not write the download: {e}"))?;
            done += n as u64;
            progress(done, total);
        }
        file.sync_all().map_err(|e| format!("Could not write the download: {e}"))?;
        let actual = hex(&hasher.finalize());
        if actual != expected {
            return Err("The download is damaged (its checksum does not match). Try again, or download the .run file from the releases page.".into());
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            std::fs::rename(&part, &done_path).map_err(|e| format!("Could not finish the download: {e}"))?;
            Ok(done_path)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

/// Runs the downloaded installer in update mode, passing each output line to `on_line`. Ok when it exits 0.
pub fn run_installer(installer: &Path, client: &Path, mut on_line: impl FnMut(&str)) -> Result<(), String> {
    let mut child = std::process::Command::new("bash")
        .arg(installer)
        .arg("--client")
        .arg(client)
        .arg("--update")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start the installer: {e}"))?;
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            let t = line.trim();
            if !t.is_empty() {
                on_line(t);
            }
        }
    }
    let status = child.wait().map_err(|e| format!("The installer did not finish: {e}"))?;
    let _ = std::fs::remove_file(installer);
    if status.success() {
        Ok(())
    } else {
        Err("The installer reported a problem. Logs/fta-linux-install.log in the client folder has the details.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("v0.2.10", Some("v0.2.9")));
        assert!(!is_newer("v0.2.9", Some("v0.2.9")));
        assert!(!is_newer("v0.2.8", Some("v0.2.9")));
        assert!(is_newer("v0.3", Some("v0.2.9")));
        assert!(!is_newer("v0.3.0", Some("v0.3")));
        assert!(is_newer("v1.0.0", Some("v0.99.99")));
    }

    #[test]
    fn unknown_installed_version_is_offered_the_update() {
        assert!(is_newer("v0.2.9", None));
        assert!(is_newer("v0.2.9", Some("garbage")));
        assert!(!is_newer("garbage", Some("v0.2.9")));
        assert!(!is_newer("garbage", None));
    }

    fn api(json: &str) -> ApiRelease {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn release_needs_an_installer_from_this_projects_releases() {
        let ok = format!(
            r#"{{"tag_name":"v0.3.0","body":"hi","assets":[{{"name":"{INSTALLER_NAME}","size":5,"digest":"sha256:{}","browser_download_url":"{DOWNLOAD_PREFIX}v0.3.0/{INSTALLER_NAME}"}}]}}"#,
            "A".repeat(64)
        );
        let r = parse_release(api(&ok)).unwrap();
        assert_eq!(r.tag, "v0.3.0");
        assert_eq!(r.sha256.as_deref(), Some(&*"a".repeat(64)));

        let foreign = ok.replace("github.com/Ghostsupthere", "example.org/someone");
        assert!(parse_release(api(&foreign)).is_err());
        assert!(parse_release(api(r#"{"tag_name":"v1","assets":[]}"#)).is_err());
        let draft = ok.replace(r#""body":"hi""#, r#""body":"hi","draft":true"#);
        assert!(parse_release(api(&draft)).is_err());
        let bad_digest = ok.replace(&"A".repeat(64), "zz");
        assert_eq!(parse_release(api(&bad_digest)).unwrap().sha256, None);
    }

    #[test]
    fn installed_tag_reads_the_first_line_only_when_it_is_a_version() {
        let dir = std::env::temp_dir().join(format!("fta-launcher-tag-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("FTA-Linux")).unwrap();
        std::fs::write(dir.join("FTA-Linux/RELEASE"), "v0.2.10\nsomething else\n").unwrap();
        assert_eq!(installed_tag(&dir).as_deref(), Some("v0.2.10"));
        std::fs::write(dir.join("FTA-Linux/RELEASE"), "not a version\n").unwrap();
        assert_eq!(installed_tag(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// End to end against a local installer: `FTA_TEST_INSTALLER=/path/to/new.run FTA_TEST_CLIENT=/scratch/client
    /// cargo test --features dev update_flow -- --nocapture`. The client folder is changed, so never a real one.
    #[cfg(feature = "dev")]
    #[test]
    fn update_flow_against_a_local_installer() {
        let (Ok(installer), Ok(client)) = (std::env::var("FTA_TEST_INSTALLER"), std::env::var("FTA_TEST_CLIENT")) else { return };
        std::env::set_var("FTA_LAUNCHER_FAKE_LATEST", "v9.9.9");
        std::env::set_var("FTA_LAUNCHER_LOCAL_INSTALLER", &installer);
        let rel = fetch_feed().unwrap().latest;
        assert!(is_newer(&rel.tag, installed_tag(Path::new(&client)).as_deref()));
        let cancel = AtomicBool::new(false);
        let mut last = 0;
        let path = download(&rel, &cancel, |done, _| last = done).unwrap();
        assert_eq!(last, rel.size);
        let mut lines = Vec::new();
        run_installer(&path, Path::new(&client), |l| lines.push(l.to_string())).unwrap();
        for l in &lines {
            println!("installer: {l}");
        }
        assert!(!path.exists(), "the downloaded installer is removed after use");
        assert!(lines.iter().any(|l| l.starts_with("Done.")));
    }

    #[cfg(feature = "dev")]
    #[test]
    fn a_damaged_download_is_refused() {
        let dir = std::env::temp_dir().join(format!("fta-launcher-dl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("fake.run");
        std::fs::write(&file, b"hello").unwrap();
        let rel = Release {
            tag: "v1.0.0".into(),
            installer_url: format!("local:{}", file.display()),
            size: 5,
            sha256: Some("0".repeat(64)),
            sums_url: None,
        };
        let err = download(&rel, &AtomicBool::new(false), |_, _| {}).unwrap_err();
        assert!(err.contains("damaged"), "{err}");
        assert!(!cache_dir().join(format!("{INSTALLER_NAME}.part")).exists());
        assert!(!cache_dir().join(INSTALLER_NAME).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn notes_stop_before_the_install_line_and_read_as_plain_text() {
        let body = "**Smoother in busy scenes.** Particle `effects` are [faster](https://example.org/x) now.\n\n**Five options.** Off by default.\n- `a`: one\n- **b**: two\n\n**Install / update:** download the file.\n\n<details>\n<summary>More</summary>\n\n**Hidden.** no\n</details>\n";
        let blocks = parse_notes(body);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].lead, "Smoother in busy scenes");
        assert_eq!(blocks[0].text, "Particle effects are faster now.");
        assert_eq!(blocks[1].lead, "Five options");
        assert_eq!(blocks[1].bullets, vec!["a: one".to_string(), "b: two".to_string()]);
    }

    #[test]
    fn long_text_and_many_bullets_are_trimmed() {
        let body = format!("**Lead.** {}\n\n**List.**\n{}", "word ".repeat(100), (1..=9).map(|i| format!("- item {i}\n")).collect::<String>());
        let blocks = parse_notes(&body);
        assert!(blocks[0].text.ends_with("...") && blocks[0].text.chars().count() <= 230);
        assert_eq!(blocks[1].bullets.len(), 6);
        assert_eq!(blocks[1].bullets[5], "...and 4 more");
    }

    #[test]
    fn a_paragraph_without_a_lead_is_kept_as_text() {
        let blocks = parse_notes("Just a sentence.\n");
        assert_eq!(blocks, vec![Block { lead: String::new(), text: "Just a sentence.".into(), bullets: vec![] }]);
        assert!(parse_notes("").is_empty());
        assert!(parse_notes("<details>\nx").is_empty());
    }

    #[test]
    fn feed_skips_drafts_and_titles_come_from_the_name() {
        let json = r#"[
          {"tag_name":"v0.3.0","name":"v0.3.0: shiny","published_at":"2026-11-20T10:00:00Z","body":"**Shiny.** Yes.","draft":true},
          {"tag_name":"v0.2.9","name":"v0.2.9: faster particles","published_at":"2026-11-08T21:00:00Z","body":"**Fast.** Very.",
           "assets":[{"name":"FTA-Linux-Installer.run","size":5,"browser_download_url":"https://github.com/Ghostsupthere/fta-linux-client/releases/download/v0.2.9/FTA-Linux-Installer.run"}]},
          {"tag_name":"v0.2.8","name":"v0.2.8","published_at":"2026-11-01T00:00:00Z","body":""}]"#;
        let feed = feed_from(serde_json::from_str(json).unwrap()).unwrap();
        assert_eq!(feed.latest.tag, "v0.2.9");
        assert_eq!(feed.notes.len(), 2);
        assert_eq!(feed.notes[0].title, "faster particles");
        assert_eq!(feed.notes[0].date, "2026-11-08");
        assert_eq!(feed.notes[1].title, "");
    }
}
