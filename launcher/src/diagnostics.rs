//! The Diagnostics page: a quick health check of the install and the system, and a text report that can be
//! pasted into a bug report. The report holds no account name, password or other login data, and the home
//! folder is shown as `~`.

use crate::cfgfile::{Doc, Style};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Ok,
    Warn,
    Fail,
}

pub struct Check {
    pub level: Level,
    pub name: String,
    pub detail: String,
}

pub struct Report {
    pub checks: Vec<Check>,
    /// Everything, as plain text.
    pub text: String,
}

fn check(level: Level, name: &str, detail: impl Into<String>) -> Check {
    Check { level, name: name.to_string(), detail: detail.into() }
}

/// Shortens the home folder to `~` so a pasted report does not carry the user name.
fn tidy(text: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if home.len() > 1 => text.replace(&home, "~"),
        _ => text.to_string(),
    }
}

fn read_trim(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn os_name() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                .map(|v| v.trim().trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

fn cpu_name() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("model name").and_then(|r| r.split_once(':')).map(|(_, v)| v.trim().to_string())))
        .unwrap_or_else(|| "unknown".into())
}

fn memory_gb() -> String {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("MemTotal:").map(|v| v.trim().to_string())))
        .and_then(|v| v.split_whitespace().next()?.parse::<f64>().ok())
        .map(|kb| format!("{:.1} GB", kb / 1024.0 / 1024.0))
        .unwrap_or_else(|| "unknown".into())
}

/// Runs a program with a short time limit and returns its standard output.
fn run_limited(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(_) => break,
            None if start.elapsed() > std::time::Duration::from_secs(6) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(std::time::Duration::from_millis(40)),
        }
    }
    let mut out = String::new();
    std::io::Read::read_to_string(&mut child.stdout.take()?, &mut out).ok()?;
    Some(out)
}

/// Graphics devices as Vulkan reports them (needs `vulkaninfo`, which most distributions ship with their Vulkan tools).
fn vulkan_devices() -> Option<Vec<String>> {
    let out = run_limited("vulkaninfo", &["--summary"])?;
    let names: Vec<String> = out
        .lines()
        .filter_map(|l| l.trim().strip_prefix("deviceName").and_then(|r| r.split_once('=')).map(|(_, v)| v.trim().to_string()))
        .collect();
    (!names.is_empty()).then_some(names)
}

fn vulkan_icd_files() -> usize {
    ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d", "/usr/local/share/vulkan/icd.d"]
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flat_map(|rd| rd.flatten())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .count()
}

/// The directories the game's start script puts on the library path after its own.
pub fn system_lib_dirs() -> Vec<&'static str> {
    [
        "/usr/lib", "/usr/lib64", "/usr/lib/x86_64-linux-gnu", "/lib/x86_64-linux-gnu", "/lib64", "/lib", "/usr/local/lib",
        "/usr/lib/pulseaudio", "/usr/lib/x86_64-linux-gnu/pulseaudio",
    ]
    .into_iter()
    .filter(|d| Path::new(d).is_dir())
    .collect()
}

/// Libraries the client needs that this system does not provide, as the bundled loader sees it.
fn missing_libraries(client: &Path) -> Result<Vec<String>, String> {
    let fta = client.join("FTA-Linux");
    let loader = fta.join("lib/ld-linux-x86-64.so.2");
    let mut path = format!("{}:{}", fta.join("lib").display(), fta.join("dxvk").display());
    for d in system_lib_dirs() {
        path.push(':');
        path.push_str(d);
    }
    let out = Command::new(&loader)
        .arg("--library-path")
        .arg(&path)
        .arg("--list")
        .arg("./Wow64")
        .current_dir(client)
        .env_remove("LD_LIBRARY_PATH")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run the bundled loader: {e}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if !out.status.success() && !text.contains("not found") {
        return Err(text.lines().next().unwrap_or("the bundled loader failed").to_string());
    }
    Ok(text
        .lines()
        .filter(|l| l.contains("not found"))
        .map(|l| l.trim().split_whitespace().next().unwrap_or("").to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// The most recent launch log and its last lines.
fn latest_launch_log(client: &Path) -> Option<(PathBuf, String)> {
    let mut logs: Vec<PathBuf> = std::fs::read_dir(client.join("Logs"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("native-launch-") && n.ends_with(".log")))
        .collect();
    logs.sort(); // names carry the date and time
    let path = logs.pop()?;
    let bytes = std::fs::read(&path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().collect();
    let tail = lines[lines.len().saturating_sub(30)..].join("\n");
    Some((path, tail))
}

/// How many crash reports the game wrote to `Errors/`, the newest one and its first lines.
fn latest_crash_report(client: &Path) -> Option<(usize, PathBuf, String)> {
    let mut reports: Vec<PathBuf> = std::fs::read_dir(client.join("Errors"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("txt")))
        .collect();
    reports.sort(); // names carry the date and time
    let count = reports.len();
    let path = reports.pop()?;
    let bytes = std::fs::read(&path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let head: String = text.lines().take(40).collect::<Vec<_>>().join("\n");
    Some((count, path, head.chars().take(4000).collect()))
}

fn host_only(realm: &str) -> String {
    realm.split(':').next().unwrap_or(realm).to_string()
}

pub fn run(client: Option<&Path>, launcher_version: &str, update_note: &str) -> Report {
    let mut checks = Vec::new();

    // ---- the system ----
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());
    checks.push(check(
        Level::Ok,
        "System",
        format!(
            "{} | kernel {} | {} | {} RAM",
            os_name(),
            read_trim("/proc/sys/kernel/osrelease").unwrap_or_else(|| "?".into()),
            cpu_name(),
            memory_gb()
        ),
    ));
    checks.push(check(
        Level::Ok,
        "Session",
        format!(
            "{} on {}",
            session,
            std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "an unknown desktop".into())
        ),
    ));
    match vulkan_devices() {
        Some(names) => checks.push(check(Level::Ok, "Graphics (Vulkan)", names.join(", "))),
        None => {
            let icds = vulkan_icd_files();
            if icds > 0 {
                checks.push(check(
                    Level::Warn,
                    "Graphics (Vulkan)",
                    format!("{icds} Vulkan driver file(s) found, but the vulkaninfo tool is not installed to list the cards"),
                ));
            } else {
                checks.push(check(
                    Level::Fail,
                    "Graphics (Vulkan)",
                    "no Vulkan driver found. Install your graphics driver's Vulkan package (the installer's library check lists it)",
                ));
            }
        }
    }

    // ---- the install ----
    let Some(client) = client else {
        checks.push(check(Level::Fail, "Client folder", "not found. Start the launcher with --client /path/to/client"));
        return finish(checks, launcher_version, update_note, None, None, None);
    };
    checks.push(check(Level::Ok, "Client folder", tidy(&client.display().to_string())));
    let wow = client.join("Wow64");
    match std::fs::metadata(&wow) {
        Ok(m) if m.len() > 0 => checks.push(check(Level::Ok, "Game program", format!("Wow64, {:.1} MB", m.len() as f64 / 1_048_576.0))),
        _ => checks.push(check(Level::Fail, "Game program", "Wow64 is missing. Run the installer again")),
    }
    let loader = client.join("FTA-Linux/lib/ld-linux-x86-64.so.2");
    if !loader.is_file() {
        checks.push(check(Level::Fail, "Bundled libraries", "FTA-Linux/lib is missing. Run the installer again"));
    } else if wow.is_file() {
        match missing_libraries(client) {
            Ok(m) if m.is_empty() => checks.push(check(Level::Ok, "System libraries", "all the client's libraries are present")),
            Ok(m) => checks.push(check(
                Level::Fail,
                "System libraries",
                format!("missing: {}. Run FTA-Linux/install-deps.sh --install", m.join(", ")),
            )),
            Err(e) => checks.push(check(Level::Fail, "System libraries", e)),
        }
    }
    let packs = ["light_shaders", "light_weather", "wmo_shaders", "soft_particles", "grass_shaders", "grass_instanced"];
    let absent: Vec<&str> = packs
        .iter()
        .copied()
        .filter(|p| !client.join(format!("Utils/WarcraftXL/visuals/{p}.pack")).is_file())
        .collect();
    if absent.is_empty() {
        checks.push(check(Level::Ok, "Shader packs", format!("all {} are installed", packs.len())));
    } else {
        checks.push(check(Level::Warn, "Shader packs", format!("missing: {} (those features stay off)", absent.join(", "))));
    }

    let lamps = client.join("Utils/WarcraftXL/visuals/lamp_models.csv");
    if lamps.is_file() {
        let n = std::fs::read_to_string(&lamps).map(|t| t.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).count().saturating_sub(1)).unwrap_or(0);
        checks.push(check(Level::Ok, "Lamp catalog", format!("{n} models")));
    } else {
        checks.push(check(Level::Warn, "Lamp catalog", "lamp_models.csv is missing (static lamps and torches stay off)"));
    }

    // ---- optional data patches ----
    let data = client.join("Data");
    let has = |f: &str| data.join(f).is_file();
    let (mut present, mut absent_p) = (Vec::new(), Vec::new());
    for (label, ok) in [
        ("model bounds fix", has("patch-YF.mpq")),
        ("Flamestrike fix", has("patch-Z9.mpq") && has("patch-ZY.mpq")),
        ("Murloc race", has("patch-ZD.mpq")),
    ] {
        if ok { present.push(label.to_string()) } else { absent_p.push(label.to_string()) }
    }
    let screens = std::fs::read_dir(data.join("LoginScreens")).map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().starts_with("login-")).count()).unwrap_or(0);
    checks.push(check(
        Level::Ok,
        "Data patches",
        format!(
            "installed: {}; not installed: {}; {} login screen pack(s)",
            if present.is_empty() { "none".to_string() } else { present.join(", ") },
            if absent_p.is_empty() { "none".to_string() } else { absent_p.join(", ") },
            screens
        ),
    ));

    // ---- interface ----
    let env_ini = std::fs::read_to_string(client.join("WTF/WXL/Environment.ini")).ok();
    let env_value = |key: &str| -> Option<String> {
        env_ini.as_ref()?.lines().find_map(|l| {
            let (k, v) = l.trim().split_once('=')?;
            (k.trim() == key).then(|| v.trim().to_string())
        })
    };
    let addons = client.join("Interface/AddOns");
    let drake = addons.join("DrakeishUI");
    let drake_installed = drake.join("DrakeishUI.toc").is_file();
    let drake_complete = drake_installed && drake.join("SelectivePackageManifest.json").is_file() && drake.join("modules/DragonflightUI/Textures").is_dir();
    let drake_selected = env_value("addons.drakeishUI").map(|v| crate::catalog::parse_bool(&v)).unwrap_or(true);
    let elvui = addons.join("ElvUI/ElvUI.toc").is_file();
    let (level, detail) = if drake_installed && !drake_complete {
        (Level::Warn, "DrakeishUI is installed but incomplete (textures or manifest missing); run the installer again, then restart the game".to_string())
    } else if drake_selected && !drake_installed {
        (Level::Warn, "DrakeishUI is the selected interface but is not installed (run the installer, or Extras > DrakeishUI)".to_string())
    } else if drake_installed && drake_selected {
        (Level::Ok, format!("DrakeishUI selected{}", if elvui { "; ElvUI is installed but stays off while DrakeishUI is selected" } else { "" }))
    } else if drake_installed {
        (Level::Ok, format!("DrakeishUI installed but not selected (Interface > Game > Interface style){}", if elvui { "; ElvUI can load" } else { "" }))
    } else {
        (Level::Ok, "stock interface (DrakeishUI is not installed)".to_string())
    };
    checks.push(check(level, "Interface", detail));
    match &env_ini {
        Some(text) => {
            let n = text.lines().filter(|l| l.contains('=')).count();
            let preview = env_value("atmosphere.preview").map(|v| crate::catalog::parse_bool(&v)).unwrap_or(false);
            checks.push(check(
                Level::Ok,
                "In-game visual settings",
                format!("{n} saved; \"Use these settings\" is {}", if preview { "on (the in-game page overrides the launch profile)" } else { "off (the launcher profile applies)" }),
            ));
        }
        None => checks.push(check(Level::Ok, "In-game visual settings", "none saved yet (Video options > Effects > Visuals & Guidance)")),
    }

    // ---- crash reports ----
    let crash = latest_crash_report(client);
    match &crash {
        Some((count, path, _)) => {
            let recent = std::fs::metadata(path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| std::time::SystemTime::now().duration_since(t).ok())
                .is_some_and(|age| age.as_secs() < 7 * 24 * 3600);
            checks.push(check(
                if recent { Level::Warn } else { Level::Ok },
                "Crash reports",
                format!("{count} in Errors/, newest {}{}", path.file_name().and_then(|n| n.to_str()).unwrap_or("?"), if recent { " (within the last week; its first lines are in the saved report)" } else { "" }),
            ));
        }
        None => checks.push(check(Level::Ok, "Crash reports", "none")),
    }

    // ---- settings ----
    let wtf = Doc::load(Style::Wtf, &client.join("WTF/Config.wtf"));
    let realm = wtf.get("realmList").map(|r| host_only(&r));
    if client.join("WarcraftXL.cfg").is_file() {
        checks.push(check(Level::Ok, "WarcraftXL.cfg", "present"));
    } else {
        checks.push(check(Level::Warn, "WarcraftXL.cfg", "not found; native features use their built-in defaults"));
    }
    checks.push(check(
        if realm.is_some() { Level::Ok } else { Level::Warn },
        "Realm",
        realm.clone().unwrap_or_else(|| "not set yet (the Game page sets it)".into()),
    ));
    if let Some(res) = wtf.get("gxResolution") {
        let windowed = wtf.get("gxWindow").map(|v| crate::catalog::parse_bool(&v));
        checks.push(check(
            Level::Ok,
            "Display",
            format!("{res}, {}", match windowed { Some(true) => "windowed", Some(false) => "fullscreen", None => "window mode not set" }),
        ));
    }

    // Debug switches left on slow the game down and fill the logs, so the report says which are set.
    let on: Vec<String> = std::fs::read_to_string(client.join("FTA-Linux/debug.env"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.trim().split_once('='))
        .filter(|(k, v)| !k.starts_with('#') && !matches!(v.trim(), "0" | ""))
        .map(|(k, v)| format!("{}={}", k.trim(), v.trim()))
        .collect();
    if !on.is_empty() {
        checks.push(check(Level::Warn, "Debug switches", format!("{} (Settings page, Debugging)", on.join(", "))));
    }

    let log = latest_launch_log(client);
    let crash_head = crash.map(|(_, path, head)| (path, head));
    finish(checks, launcher_version, update_note, Some(client), log, crash_head)
}

fn finish(checks: Vec<Check>, launcher_version: &str, update_note: &str, client: Option<&Path>, log: Option<(PathBuf, String)>, crash: Option<(PathBuf, String)>) -> Report {
    let mut text = String::new();
    text.push_str("Realmgate launcher: diagnostics report\n");
    // The release tag is what matters once a client is known; the crate version is only a fallback.
    if client.is_none() {
        text.push_str(&format!("launcher version: {launcher_version}\n"));
    }
    if let Some(client) = client {
        text.push_str(&format!("installed release: {}\n", crate::updater::installed_tag(client).unwrap_or_else(|| "not recorded".into())));
        if let Ok(stamp) = std::fs::read_to_string(client.join("FTA-Linux/VERSION.txt")) {
            let line = stamp.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" | ");
            text.push_str(&format!("build: {line}\n"));
        }
    }
    if !update_note.is_empty() {
        text.push_str(&format!("update: {update_note}\n"));
    }
    text.push('\n');
    for c in &checks {
        let mark = match c.level {
            Level::Ok => "OK  ",
            Level::Warn => "WARN",
            Level::Fail => "FAIL",
        };
        text.push_str(&format!("[{mark}] {}: {}\n", c.name, c.detail));
    }
    if let Some((path, tail)) = log {
        text.push_str(&format!("\nLast launch log ({}), final lines:\n{}\n", path.file_name().and_then(|n| n.to_str()).unwrap_or("?"), tail));
    }
    if let Some((path, head)) = crash {
        text.push_str(&format!("\nNewest crash report ({}), first lines:\n{}\n", path.file_name().and_then(|n| n.to_str()).unwrap_or("?"), head));
    }
    Report { checks, text: tidy(&text) }
}

/// Saves the report to `Logs/fta-launcher-report.txt` in the client folder and returns the path.
pub fn save(client: &Path, text: &str) -> Result<PathBuf, String> {
    let dir = client.join("Logs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let path = dir.join("fta-launcher-report.txt");
    std::fs::write(&path, text).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_never_carries_the_account_name() {
        let dir = std::env::temp_dir().join(format!("fta-launcher-diag-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("WTF")).unwrap();
        std::fs::create_dir_all(dir.join("Data")).unwrap();
        std::fs::create_dir_all(dir.join("Interface")).unwrap();
        std::fs::write(
            dir.join("WTF/Config.wtf"),
            "SET realmList \"play.example.org:3724\"\nSET accountName \"SECRETNAME\"\nSET gxResolution \"1920x1080\"\nSET gxWindow \"1\"\n",
        )
        .unwrap();
        let r = run(Some(&dir), "0.0.0-test", "");
        assert!(!r.text.contains("SECRETNAME"));
        assert!(r.text.contains("play.example.org"));
        assert!(!r.text.contains(":3724"), "only the host is reported");
        assert!(r.text.contains("1920x1080, windowed"));
        assert!(r.checks.iter().any(|c| c.name == "Game program" && c.level == Level::Fail));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn interface_and_crash_checks_read_the_client_folder() {
        let dir = std::env::temp_dir().join(format!("fta-launcher-diag2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("WTF")).unwrap();
        std::fs::create_dir_all(dir.join("Data")).unwrap();
        std::fs::create_dir_all(dir.join("Interface/AddOns/DrakeishUI")).unwrap();
        std::fs::create_dir_all(dir.join("Errors")).unwrap();
        std::fs::write(dir.join("WTF/Config.wtf"), "SET realmList \"example.org\"\n").unwrap();
        std::fs::write(dir.join("Interface/AddOns/DrakeishUI/DrakeishUI.toc"), "## Title: x\n").unwrap();
        std::fs::write(dir.join("Errors/2026-01-01 00.00.00 Crash.txt"), "line one\nline two\n").unwrap();
        let r = run(Some(&dir), "0.0.0", "");
        let find = |name: &str| r.checks.iter().find(|c| c.name == name).unwrap_or_else(|| panic!("no check {name}"));
        assert_eq!(find("Interface").level, Level::Warn);
        assert!(find("Interface").detail.contains("incomplete"));
        assert!(find("Crash reports").detail.starts_with("1 in Errors/"));
        assert!(find("Data patches").detail.contains("not installed"));
        assert!(r.text.contains("Newest crash report") && r.text.contains("line two"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_client_is_one_failing_check() {
        let r = run(None, "0.0.0-test", "");
        assert!(r.checks.iter().any(|c| c.name == "Client folder" && c.level == Level::Fail));
    }

    #[test]
    fn host_only_drops_the_port() {
        assert_eq!(host_only("a.b:3724"), "a.b");
        assert_eq!(host_only("1.2.3.4"), "1.2.3.4");
    }
}
