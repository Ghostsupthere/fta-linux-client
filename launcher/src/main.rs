use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use slint::{ModelRc, SharedString, VecModel};

use slint::winit_030::WinitWindowAccessor;

mod catalog;
mod cfgfile;
mod client;
mod diagnostics;
mod quips;
mod game;
mod loginscreen;
mod settings;
mod updater;

slint::include_modules!();

/// The only places the launcher will ever open in a browser.
const ALLOWED_LINKS: &[&str] = &[
    "https://wiki.downloadautism.com/",
    "https://github.com/Ghostsupthere",
    "https://github.com/Ghostsupthere/fta-linux-client/releases",
];

/// The launcher's own config folder (`~/.config/fta-linux-launcher`): the theme choice and an optional quips.txt.
fn config_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))?;
    Some(base.join("fta-linux-launcher"))
}

fn theme_file() -> Option<std::path::PathBuf> {
    Some(config_dir()?.join("theme"))
}

fn save_theme(night: bool) {
    let Some(path) = theme_file() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, if night { "night\n" } else { "day\n" });
}

/// Which artwork to start with: FTA_LAUNCHER_THEME=day|night (for testing), else the saved choice, else night
/// between 19:00 and 07:00 local time. The std library has no local time, so this asks `date` (always there on
/// Linux); if that fails, it is day.
fn initial_night() -> bool {
    match std::env::var("FTA_LAUNCHER_THEME").as_deref() {
        Ok("night") => return true,
        Ok("day") => return false,
        _ => {}
    }
    if let Some(saved) = theme_file().and_then(|p| std::fs::read_to_string(p).ok()) {
        match saved.trim() {
            "night" => return true,
            "day" => return false,
            _ => {}
        }
    }
    let hour = std::process::Command::new("date")
        .arg("+%H")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|t| t.trim().parse::<u32>().ok());
    matches!(hour, Some(h) if !(7..19).contains(&h))
}

/// An explicit motion choice: FTA_LAUNCHER_REDUCE_MOTION=1|0, else the first line of `~/.config/fta-linux-launcher/motion`
/// (`reduce` or `full`). None when the person has not said.
fn motion_choice() -> Option<bool> {
    if let Ok(v) = std::env::var("FTA_LAUNCHER_REDUCE_MOTION") {
        match v.trim() {
            "1" | "true" | "yes" => return Some(true),
            "0" | "false" | "no" => return Some(false),
            _ => {}
        }
    }
    let text = std::fs::read_to_string(config_dir()?.join("motion")).ok()?;
    match text.lines().next()?.trim() {
        "reduce" => Some(true),
        "full" => Some(false),
        _ => None,
    }
}

/// Whether the desktop asks for fewer animations: GNOME and anything that answers the settings portal
/// (`org.gnome.desktop.interface enable-animations`), or KDE (`AnimationDurationFactor=0` in kdeglobals).
fn desktop_prefers_reduced_motion() -> bool {
    if let Some(base) = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
    {
        if let Ok(text) = std::fs::read_to_string(base.join("kdeglobals")) {
            let mut in_kde = false;
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with('[') {
                    in_kde = line == "[KDE]";
                } else if in_kde {
                    if let Some(v) = line.strip_prefix("AnimationDurationFactor=") {
                        if v.trim().parse::<f64>().map(|f| f <= 0.0).unwrap_or(false) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    let out = std::process::Command::new("gdbus")
        .args([
            "call", "--session", "--timeout", "2",
            "--dest", "org.freedesktop.portal.Desktop",
            "--object-path", "/org/freedesktop/portal/desktop",
            "--method", "org.freedesktop.portal.Settings.Read",
            "org.gnome.desktop.interface", "enable-animations",
        ])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).contains("false"),
        _ => false,
    }
}

fn open_link(url: &str) {
    if !ALLOWED_LINKS.contains(&url) {
        eprintln!("launcher: refusing to open {url}");
        return;
    }
    // xdg-open is the standard way on Linux desktops; no shell is involved.
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

/// The Settings page groups: one block per category, each with its toggles (and their position in the full list).
fn groups(model: &settings::Settings) -> Vec<SettingGroup> {
    let mut out: Vec<(String, Vec<SettingRow>)> = Vec::new();
    for (i, s) in catalog::SETTINGS.iter().enumerate() {
        if out.last().map(|(title, _)| title.as_str()) != Some(s.category) {
            out.push((s.category.to_string(), Vec::new()));
        }
        out.last_mut().unwrap().1.push(SettingRow {
            key: s.key.into(),
            label: s.label.into(),
            hint: s.hint.into(),
            checked: model.values[i],
            index: i as i32,
        });
    }
    out.into_iter()
        .map(|(title, rows)| {
            // "DEBUGGING (takes effect at ...)": the part in brackets becomes the small note beside the title
            let (head, note) = match title.split_once('(') {
                Some((h, n)) => (h.trim().to_string(), n.trim_end().trim_end_matches(')').to_string()),
                None => (title.clone(), String::new()),
            };
            SettingGroup { title: head.into(), note: note.into(), rows: ModelRc::new(VecModel::from(rows)) }
        })
        .collect()
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Shows the Game page values that are not text boxes (the boxes keep what the person typed).
fn show_game(ui: &MainWindow, g: &game::Game) {
    ui.set_game_windowed(g.values.windowed);
    ui.set_game_vsync(g.values.vsync);
    ui.set_game_msaa(g.values.msaa as i32);
    ui.set_game_dirty(g.is_dirty());
    let realm_bad = !g.values.realm.is_empty() && game::realm_problem(&g.values.realm).is_some();
    let res_bad = !g.values.resolution.is_empty() && game::resolution_problem(&g.values.resolution).is_some();
    ui.set_game_realm_bad(realm_bad);
    ui.set_game_resolution_bad(res_bad);
    // An empty realm is only a problem once something else was changed (a fresh client has none yet).
    ui.set_game_problem(g.problem().filter(|_| g.is_dirty()).unwrap_or("").into());
}

/// Fills the Login tab from what is on disk.
fn show_login(ui: &MainWindow, client: &std::path::Path) {
    let st = loginscreen::state(client);
    let rows: Vec<LoginRow> = st
        .entries
        .iter()
        .map(|e| LoginRow { id: e.id.into(), title: e.title.into(), available: e.available && !st.foreign_file, active: e.active })
        .collect();
    ui.set_login_rows(ModelRc::new(VecModel::from(rows)));
    ui.set_login_foreign(st.foreign_file);
    ui.set_login_folder(st.packs_dir.display().to_string().into());
}

fn show_game_text(ui: &MainWindow, g: &game::Game) {
    ui.set_game_realm(g.values.realm.clone().into());
    ui.set_game_resolution(g.values.resolution.clone().into());
}

/// The changelog as flat rows for the list: a heading per release (with its position, so the page can fold the
/// others away), then its paragraphs and bullets.
fn change_rows(notes: &[updater::Note], installed: Option<&str>) -> Vec<ChangeRow> {
    let mut rows = Vec::new();
    let body = |release: i32, kind: i32, text: &str| ChangeRow {
        kind,
        text: text.into(),
        title: "".into(),
        date: "".into(),
        release,
        latest: false,
        installed: false,
    };
    for (i, n) in notes.iter().enumerate() {
        let i = i as i32;
        rows.push(ChangeRow {
            kind: 0,
            text: n.tag.as_str().into(),
            title: n.title.as_str().into(),
            date: n.date.as_str().into(),
            release: i,
            latest: i == 0,
            installed: installed == Some(n.tag.as_str()),
        });
        for b in &n.blocks {
            if !b.lead.is_empty() {
                rows.push(body(i, 1, &b.lead));
            }
            if !b.text.is_empty() {
                rows.push(body(i, 2, &b.text));
            }
            for bullet in &b.bullets {
                rows.push(body(i, 3, bullet));
            }
        }
    }
    rows
}

fn show_report(ui: &MainWindow, report: &diagnostics::Report) {
    let rows: Vec<CheckRow> = report
        .checks
        .iter()
        .map(|c| CheckRow {
            level: match c.level {
                diagnostics::Level::Ok => 0,
                diagnostics::Level::Warn => 1,
                diagnostics::Level::Fail => 2,
            },
            name: c.name.clone().into(),
            detail: c.detail.clone().into(),
        })
        .collect();
    ui.set_checks(ModelRc::new(VecModel::from(rows)));
    ui.set_report_text(report.text.clone().into());
}

/// Runs the checks on a worker thread and shows the result when they finish.
fn run_diagnostics(weak: slint::Weak<MainWindow>, client: Option<std::path::PathBuf>, update_note: String) {
    if let Some(ui) = weak.upgrade() {
        ui.set_diag_busy(true);
        ui.set_diag_status("".into());
    }
    std::thread::spawn(move || {
        let report = diagnostics::run(client.as_deref(), VERSION, &update_note);
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                show_report(&ui, &report);
                ui.set_diag_busy(false);
            }
        });
    });
}

/// Looks up the newest release in the background and updates the Play page.
fn check_for_update(weak: slint::Weak<MainWindow>, client: Option<std::path::PathBuf>, slot: Arc<Mutex<Option<updater::Release>>>) {
    let Some(client) = client else { return };
    if let Some(ui) = weak.upgrade() {
        ui.set_update_retry(false);
        ui.set_update_offline(false);
        ui.set_update_state(1);
        ui.set_update_text("Checking for updates...".into());
        ui.set_changelog_status("Loading...".into());
    }
    std::thread::spawn(move || {
        let result = updater::fetch_feed();
        let installed = updater::installed_tag(&client);
        let _ = slint::invoke_from_event_loop(move || {
            let Some(ui) = weak.upgrade() else { return };
            match result {
                Ok(feed) => {
                    let rel = feed.latest;
                    ui.set_changelog(ModelRc::new(VecModel::from(change_rows(&feed.notes, installed.as_deref()))));
                    ui.set_open_release(0);
                    ui.set_changelog_status("".into());
                    if updater::is_newer(&rel.tag, installed.as_deref()) {
                        let have = installed.as_deref().map(|t| format!("You have {t}.")).unwrap_or_else(|| "Your install does not record its version.".into());
                        ui.set_update_text(format!("{} is available. {}", rel.tag, have).into());
                        ui.set_update_state(2);
                        *slot.lock().unwrap() = Some(rel);
                    } else {
                        ui.set_update_text(format!("Up to date ({}).", installed.as_deref().unwrap_or(&rel.tag)).into());
                        ui.set_update_state(0);
                    }
                }
                Err(err) => {
                    eprintln!("launcher: update check failed: {err}");
                    ui.set_changelog_status("The changelog could not be loaded (no connection?).".into());
                    ui.set_update_text("Could not check for updates (no connection?). You can still play.".into());
                    ui.set_update_offline(true);
                    ui.set_update_state(0);
                }
            }
        });
    });
}

/// Downloads, verifies and installs the release, reporting progress, then starts the (new) launcher.
fn install_update(
    weak: slint::Weak<MainWindow>,
    client: std::path::PathBuf,
    rel: updater::Release,
    cancel: Arc<AtomicBool>,
) {
    if let Some(ui) = weak.upgrade() {
        ui.set_update_retry(false);
        ui.set_update_state(4);
        ui.set_update_progress(0.0);
        ui.set_update_text("Downloading...".into());
    }
    std::thread::spawn(move || {
        let w = weak.clone();
        let progress = move |done: u64, total: u64| {
            let w = w.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = w.upgrade() {
                    let frac = if total > 0 { done as f32 / total as f32 } else { 0.0 };
                    ui.set_update_progress(frac * 0.7);
                    ui.set_update_text(format!("Downloading... {:.1} of {:.1} MB", done as f64 / 1e6, total as f64 / 1e6).into());
                }
            });
        };
        let outcome = updater::download(&rel, &cancel, progress).and_then(|installer| {
            let w = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = w.upgrade() {
                    ui.set_update_progress(0.75);
                    ui.set_update_text("Installing...".into());
                }
            });
            let w = weak.clone();
            updater::run_installer(&installer, &client, move |line| {
                let line: String = line.chars().take(110).collect();
                let w = w.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = w.upgrade() {
                        ui.set_update_text(line.into());
                        ui.set_update_progress((ui.get_update_progress() + 0.004).min(0.97));
                    }
                });
            })
        });
        let _ = slint::invoke_from_event_loop(move || {
            let Some(ui) = weak.upgrade() else { return };
            match outcome {
                Ok(()) => {
                    ui.set_update_progress(1.0);
                    ui.set_update_text(format!("Updated to {}. Restarting the launcher...", rel.tag).into());
                    if !restart_launcher(&client) {
                        ui.set_update_state(0);
                        ui.set_update_text(format!("Updated to {}. Close and reopen the launcher.", rel.tag).into());
                    }
                }
                Err(err) => {
                    ui.set_update_state(5);
                    ui.set_update_retry(true);
                    ui.set_update_text(err.into());
                }
            }
        });
    });
}

/// Starts the freshly installed launcher and ends this one. False if there is no start script to run.
fn restart_launcher(client: &std::path::Path) -> bool {
    let script = client.join("FTA-Linux/launcher.sh");
    if !script.is_file() {
        return false;
    }
    let started = std::process::Command::new("setsid")
        .arg("-f")
        .arg(&script)
        .current_dir(client)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok();
    if started {
        let _ = slint::quit_event_loop();
    }
    started
}

fn main() -> Result<(), slint::PlatformError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let client_dir = client::find(&args);

    // Wayland shells pick the taskbar/dock icon from the .desktop file whose name matches the window's app id.
    // The installer writes fta-linux-client.desktop (with Icon= and StartupWMClass=), so the two must agree.
    // (set_xdg_app_id needs a platform to exist, so the default backend is selected first.)
    slint::BackendSelector::new().select()?;
    slint::set_xdg_app_id("fta-linux-client")?;
    let ui = MainWindow::new()?;
    ui.global::<Pal>().set_night(initial_night());
    {
        let weak = ui.as_weak();
        ui.on_toggle_theme(move || {
            if let Some(ui) = weak.upgrade() {
                let pal = ui.global::<Pal>();
                let night = !pal.get_night();
                pal.set_night(night);
                save_theme(night);
            }
        });
    }

    // ---- the sentinel's line ----
    let lines = Rc::new(quips::load(config_dir().as_deref()));
    let quip_index = Rc::new(std::cell::Cell::new(quips::pick(None, lines.len())));
    ui.set_quip(lines[quip_index.get()].as_str().into());
    {
        let weak = ui.as_weak();
        let index = quip_index.clone();
        let lines = lines.clone();
        ui.on_new_quip(move || {
            if let Some(ui) = weak.upgrade() {
                let next = quips::pick(Some(index.get()), lines.len());
                index.set(next);
                ui.set_quip(lines[next].as_str().into());
            }
        });
    }

    // ---- Play page ----
    match &client_dir {
        Some(dir) => {
            ui.set_client_found(true);
            ui.set_headline("Ready to play".into());
            ui.set_installed_tag(updater::installed_tag(dir).unwrap_or_default().into());
            let stamp = client::installed_stamp(dir).unwrap_or_else(|| "Installed".to_string());
            let line = match updater::installed_tag(dir) {
                Some(tag) => format!("{tag}  ·  {stamp}"),
                None => stamp,
            };
            ui.set_subline(SharedString::from(line));
        }
        None => {
            ui.set_client_found(false);
            ui.set_headline("Client folder not found".into());
            ui.set_subline(
                "Run the installer from your From The Ashes folder first, or start the launcher with --client /path/to/client.".into(),
            );
        }
    }

    // ---- Settings page ----
    let model = Rc::new(RefCell::new(client_dir.as_deref().map(settings::Settings::load)));
    let rows_model: Rc<VecModel<SettingGroup>> = Rc::new(VecModel::default());
    ui.set_setting_groups(ModelRc::from(rows_model.clone()));
    ui.set_preset_names(ModelRc::new(VecModel::from(
        catalog::PRESETS.iter().map(|p| SharedString::from(p.name)).collect::<Vec<_>>(),
    )));

    let refresh = {
        let model = model.clone();
        let rows_model = rows_model.clone();
        let weak = ui.as_weak();
        Rc::new(move || {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(m) = model.borrow().as_ref() {
                rows_model.set_vec(groups(m));
                ui.set_dirty(m.is_dirty());
            }
            ui.set_game_running(client::game_running());
        })
    };
    refresh();

    {
        let model = model.clone();
        let refresh = refresh.clone();
        let weak = ui.as_weak();
        ui.on_toggle_setting(move |i| {
            if let Some(m) = model.borrow_mut().as_mut() {
                m.toggle(i as usize);
            }
            if let Some(ui) = weak.upgrade() {
                ui.set_status("".into());
            }
            refresh();
        });
    }
    {
        let model = model.clone();
        let refresh = refresh.clone();
        let weak = ui.as_weak();
        ui.on_load_preset(move |i| {
            let name = model.borrow_mut().as_mut().and_then(|m| m.load_preset(i as usize));
            if let (Some(ui), Some(name)) = (weak.upgrade(), name) {
                ui.set_status(format!("Preset \"{name}\" loaded. Press Apply to save it.").into());
            }
            refresh();
        });
    }
    {
        let model = model.clone();
        let refresh = refresh.clone();
        let weak = ui.as_weak();
        let dir = client_dir.clone();
        ui.on_reload_settings(move || {
            if let Some(dir) = &dir {
                *model.borrow_mut() = Some(settings::Settings::load(dir));
            }
            if let Some(ui) = weak.upgrade() {
                ui.set_status("Changes discarded.".into());
            }
            refresh();
        });
    }
    {
        let model = model.clone();
        let refresh = refresh.clone();
        let weak = ui.as_weak();
        ui.on_apply_settings(move || {
            let message = if client::game_running() {
                "The game is running. Close it first.".to_string()
            } else {
                match model.borrow_mut().as_mut().map(|m| m.apply()) {
                    Some(Ok(msg)) => msg,
                    Some(Err(err)) => err,
                    None => "No client folder.".to_string(),
                }
            };
            if let Some(ui) = weak.upgrade() {
                ui.set_status(message.into());
            }
            refresh();
        });
    }

    // ---- Game page ----
    let game_model = Rc::new(RefCell::new(client_dir.as_deref().map(game::Game::load)));
    ui.set_resolution_presets(ModelRc::new(VecModel::from(
        game::RESOLUTION_PRESETS.iter().map(|r| SharedString::from(*r)).collect::<Vec<_>>(),
    )));
    ui.set_msaa_choices(ModelRc::new(VecModel::from(game::MSAA_CHOICES.iter().map(|n| *n as i32).collect::<Vec<_>>())));
    if let Some(g) = game_model.borrow().as_ref() {
        show_game_text(&ui, g);
        show_game(&ui, g);
    }
    {
        let m = game_model.clone();
        let weak = ui.as_weak();
        ui.on_game_realm_edited(move |t| {
            if let (Some(g), Some(ui)) = (m.borrow_mut().as_mut(), weak.upgrade()) {
                g.values.realm = t.trim().to_string();
                ui.set_game_status("".into());
                show_game(&ui, g);
            }
        });
    }
    {
        let m = game_model.clone();
        let weak = ui.as_weak();
        ui.on_game_resolution_edited(move |t| {
            if let (Some(g), Some(ui)) = (m.borrow_mut().as_mut(), weak.upgrade()) {
                g.values.resolution = t.trim().to_string();
                ui.set_game_status("".into());
                show_game(&ui, g);
            }
        });
    }
    {
        let m = game_model.clone();
        let weak = ui.as_weak();
        ui.on_game_pick_resolution(move |r| {
            if let (Some(g), Some(ui)) = (m.borrow_mut().as_mut(), weak.upgrade()) {
                g.values.resolution = r.to_string();
                ui.set_game_status("".into());
                show_game_text(&ui, g);
                show_game(&ui, g);
            }
        });
    }
    {
        let m = game_model.clone();
        let weak = ui.as_weak();
        ui.on_game_toggle_windowed(move || {
            if let (Some(g), Some(ui)) = (m.borrow_mut().as_mut(), weak.upgrade()) {
                g.values.windowed = !g.values.windowed;
                ui.set_game_status("".into());
                show_game(&ui, g);
            }
        });
    }
    {
        let m = game_model.clone();
        let weak = ui.as_weak();
        ui.on_game_toggle_vsync(move || {
            if let (Some(g), Some(ui)) = (m.borrow_mut().as_mut(), weak.upgrade()) {
                g.values.vsync = !g.values.vsync;
                ui.set_game_status("".into());
                show_game(&ui, g);
            }
        });
    }
    {
        let m = game_model.clone();
        let weak = ui.as_weak();
        ui.on_game_set_msaa(move |n| {
            if let (Some(g), Some(ui)) = (m.borrow_mut().as_mut(), weak.upgrade()) {
                g.values.msaa = n as u8;
                ui.set_game_status("".into());
                show_game(&ui, g);
            }
        });
    }
    {
        let m = game_model.clone();
        let weak = ui.as_weak();
        let dir = client_dir.clone();
        ui.on_reset_game(move || {
            if let (Some(dir), Some(ui)) = (&dir, weak.upgrade()) {
                let g = game::Game::load(dir);
                show_game_text(&ui, &g);
                show_game(&ui, &g);
                ui.set_game_status("Changes discarded.".into());
                *m.borrow_mut() = Some(g);
            }
        });
    }
    {
        let m = game_model.clone();
        let weak = ui.as_weak();
        ui.on_apply_game(move || {
            let Some(ui) = weak.upgrade() else { return };
            let message = if client::game_running() {
                "The game is running. Close it first.".to_string()
            } else {
                match m.borrow_mut().as_mut().map(|g| g.apply()) {
                    Some(Ok(msg)) => msg,
                    Some(Err(err)) => err,
                    None => "No client folder.".to_string(),
                }
            };
            ui.set_game_status(message.into());
            if let Some(g) = m.borrow().as_ref() {
                show_game(&ui, g);
            }
        });
    }

    // ---- Login tab ----
    if let Some(dir) = client_dir.as_deref() {
        show_login(&ui, dir);
    }
    {
        let weak = ui.as_weak();
        let dir = client_dir.clone();
        ui.on_pick_login(move |id| {
            let (Some(dir), Some(ui)) = (&dir, weak.upgrade()) else { return };
            let message = if client::game_running() {
                "The game is running. Close it first.".to_string()
            } else {
                match loginscreen::select(dir, id.as_str()) {
                    Ok(msg) | Err(msg) => msg,
                }
            };
            ui.set_login_status(message.into());
            show_login(&ui, dir);
        });
    }

    // ---- Updates ----
    let release_slot: Arc<Mutex<Option<updater::Release>>> = Arc::new(Mutex::new(None));
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let weak = ui.as_weak();
        let dir = client_dir.clone();
        let slot = release_slot.clone();
        ui.on_check_update(move || check_for_update(weak.clone(), dir.clone(), slot.clone()));
    }
    {
        let weak = ui.as_weak();
        let slot = release_slot.clone();
        let dir = client_dir.clone();
        ui.on_ask_update(move || {
            let (Some(ui), Some(rel)) = (weak.upgrade(), slot.lock().unwrap().clone()) else { return };
            let have = dir.as_deref().and_then(updater::installed_tag);
            ui.set_update_title(format!("Update to {}?", rel.tag).into());
            let notes = format!(
                "Download about {:.0} MB{}. What changed is listed on the right. Your settings and the Windows client are not touched.",
                rel.size as f64 / 1e6,
                have.map(|h| format!(" (you have {h})")).unwrap_or_default()
            );
            ui.set_update_notes(notes.into());
            ui.set_update_state(3);
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_cancel_ask(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_update_state(2);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let slot = release_slot.clone();
        let dir = client_dir.clone();
        let cancel = cancel.clone();
        ui.on_start_update(move || {
            let (Some(ui), Some(dir), Some(rel)) = (weak.upgrade(), dir.clone(), slot.lock().unwrap().clone()) else { return };
            if client::game_running() {
                ui.set_update_state(5);
                ui.set_update_retry(true);
                ui.set_update_text("The game is running. Close it, then try again.".into());
                return;
            }
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            install_update(weak.clone(), dir, rel, cancel.clone());
        });
    }

    // ---- Diagnostics ----
    {
        let weak = ui.as_weak();
        let dir = client_dir.clone();
        ui.on_run_diagnostics(move || run_diagnostics(weak.clone(), dir.clone(), String::new()));
    }
    {
        let weak = ui.as_weak();
        let dir = client_dir.clone();
        ui.on_save_report(move || {
            let Some(ui) = weak.upgrade() else { return };
            let msg = match &dir {
                Some(dir) => match diagnostics::save(dir, ui.get_report_text().as_str()) {
                    Ok(path) => format!("Saved to {}", path.display()),
                    Err(e) => e,
                },
                None => "No client folder.".to_string(),
            };
            ui.set_diag_status(msg.into());
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_report_copied(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_diag_status("Report copied. Paste it into your message.".into());
            }
        });
    }
    run_diagnostics(ui.as_weak(), client_dir.clone(), String::new());
    #[cfg(feature = "dev")]
    let skip_check = std::env::var("FTA_LAUNCHER_UPDATE_STATE").is_ok();
    #[cfg(not(feature = "dev"))]
    let skip_check = false;
    if !skip_check {
        check_for_update(ui.as_weak(), client_dir.clone(), release_slot.clone());
    }

    // The Play button comes back when the game closes.
    let tick = slint::Timer::default();
    {
        let refresh = refresh.clone();
        tick.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(2), move || refresh());
    }

    // ---- Play ----
    {
        let dir = client_dir.clone();
        let weak = ui.as_weak();
        ui.on_play(move || {
            let Some(dir) = &dir else { return };
            let Some(ui) = weak.upgrade() else { return };
            let script = dir.join("FTA-Linux/play.sh");
            // Start the game detached from the launcher, then close the launcher.
            let started = std::process::Command::new("setsid")
                .arg("-f")
                .arg(&script)
                .current_dir(dir)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok();
            if started {
                // Show the button's loading state for a moment so the click visibly did something.
                ui.set_play_error("".into());
                ui.set_launching(true);
                slint::Timer::single_shot(std::time::Duration::from_millis(900), || {
                    let _ = slint::quit_event_loop();
                });
            } else {
                ui.set_play_error("The game could not be started. Run Diagnostics to see what is missing.".into());
            }
        });
    }

    // ---- Window ----
    ui.on_close_requested(|| {
        let _ = slint::quit_event_loop();
    });

    let weak = ui.as_weak();
    ui.on_minimize_requested(move || {
        if let Some(ui) = weak.upgrade() {
            ui.window().set_minimized(true);
        }
    });

    // The window has no system title bar, so the header starts the move itself. Once the system takes over the
    // move (Wayland), the button release goes to it and never reaches the window, which would leave the header
    // holding the pointer: every later click would start another move instead of pressing a button. So the
    // release is delivered here, once the press has been handled.
    let weak = ui.as_weak();
    ui.on_drag_window(move |x, y| {
        let Some(ui) = weak.upgrade() else { return };
        let started = ui.window().with_winit_window(|w| w.drag_window().is_ok()).unwrap_or(false);
        if !started {
            return;
        }
        let weak = weak.clone();
        slint::Timer::single_shot(std::time::Duration::from_millis(30), move || {
            if let Some(ui) = weak.upgrade() {
                ui.window().dispatch_event(slint::platform::WindowEvent::PointerReleased {
                    position: slint::LogicalPosition::new(x, y),
                    button: slint::platform::PointerEventButton::Left,
                });
            }
        });
    });

    ui.on_open_link(|url| open_link(url.as_str()));

    // Frameless window: the edges and corners start a resize themselves (0 N, 1 S, 2 W, 3 E, 4 NW, 5 NE, 6 SW, 7 SE).
    let weak = ui.as_weak();
    ui.on_resize_window(move |edge, x, y| {
        use slint::winit_030::winit::window::ResizeDirection as D;
        let Some(ui) = weak.upgrade() else { return };
        let dir = match edge {
            0 => D::North,
            1 => D::South,
            2 => D::West,
            3 => D::East,
            4 => D::NorthWest,
            5 => D::NorthEast,
            6 => D::SouthWest,
            _ => D::SouthEast,
        };
        let started = ui.window().with_winit_window(|w| w.drag_resize_window(dir).is_ok()).unwrap_or(false);
        if !started {
            return;
        }
        // Same as the move: the compositor takes over and swallows the release.
        let weak = weak.clone();
        slint::Timer::single_shot(std::time::Duration::from_millis(30), move || {
            if let Some(ui) = weak.upgrade() {
                ui.window().dispatch_event(slint::platform::WindowEvent::PointerReleased {
                    position: slint::LogicalPosition::new(x, y),
                    button: slint::platform::PointerEventButton::Left,
                });
            }
        });
    });

    // Reduced motion: an explicit choice wins; otherwise the desktop is asked (in the background, it can be slow).
    match motion_choice() {
        Some(reduce) => ui.set_reduce_motion(reduce),
        None => {
            let weak = ui.as_weak();
            std::thread::spawn(move || {
                let reduce = desktop_prefers_reduced_motion();
                if reduce {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            ui.set_reduce_motion(true);
                        }
                    });
                }
            });
        }
    }

    // Development aid: a window size for screenshots, e.g. 1000x640.
    #[cfg(feature = "dev")]
    if let Some((w, h)) = std::env::var("FTA_LAUNCHER_SIZE").ok().and_then(|v| v.split_once('x').and_then(|(a, b)| Some((a.parse::<f32>().ok()?, b.parse::<f32>().ok()?)))) {
        ui.window().set_size(slint::LogicalSize::new(w, h));
    }

    // Development aid: start on a given tab (0 Play, 1 Game, 2 Settings, 3 Diagnostics), e.g. for screenshots.
    if let Some(tab) = std::env::var("FTA_LAUNCHER_TAB").ok().and_then(|v| v.parse::<i32>().ok()) {
        ui.set_tab(tab);
    }

    // Development aid: show an update state with sample text, for screenshots (the real check is skipped).
    #[cfg(feature = "dev")]
    if let Some(state) = std::env::var("FTA_LAUNCHER_UPDATE_STATE").ok().and_then(|v| v.parse::<i32>().ok()) {
        ui.set_update_state(state);
        ui.set_update_title("Update to v0.2.10?".into());
        ui.set_update_notes("Smoother in busy scenes. Particle effects are now updated on the model worker threads.\n\nDownload about 31 MB (you have v0.2.9). Your settings and the Windows client are not touched.".into());
        ui.set_update_text(match state { 4 => "Downloading... 14.2 of 31.0 MB", 5 => "The download is damaged (its checksum does not match). Try again, or download the .run file from the releases page.", _ => "v0.2.10 is available. You have v0.2.9." }.into());
        ui.set_update_progress(0.45);
    }

    // Development aid: copy the diagnostics report to the clipboard a moment after start.
    #[cfg(feature = "dev")]
    if std::env::var("FTA_LAUNCHER_DEV_COPY").is_ok() {
        let weak = ui.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_millis(2500), move || {
            if let Some(ui) = weak.upgrade() {
                ui.invoke_copy_report();
            }
        });
        slint::Timer::single_shot(std::time::Duration::from_millis(9000), || {
            let _ = slint::quit_event_loop();
        });
    }

    #[cfg(feature = "dev")]
    if let Ok(path) = std::env::var("FTA_LAUNCHER_SNAPSHOT") {
        let weak = ui.as_weak();
        let delay = std::env::var("FTA_LAUNCHER_SNAPSHOT_DELAY_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(1200);
        slint::Timer::single_shot(std::time::Duration::from_millis(delay), move || {
            if let Some(ui) = weak.upgrade() {
                if let Ok(buf) = ui.window().take_snapshot() {
                    if let Ok(file) = std::fs::File::create(&path) {
                        let mut enc = png::Encoder::new(file, buf.width(), buf.height());
                        enc.set_color(png::ColorType::Rgba);
                        enc.set_depth(png::BitDepth::Eight);
                        if let Ok(mut w) = enc.write_header() {
                            let _ = w.write_image_data(buf.as_bytes());
                        }
                    }
                }
            }
            let _ = slint::quit_event_loop();
        });
    }

    ui.run()
}
