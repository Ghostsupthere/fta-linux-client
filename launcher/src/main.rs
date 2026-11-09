use std::cell::RefCell;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use slint::winit_030::WinitWindowAccessor;

mod catalog;
mod cfgfile;
mod client;
mod settings;

slint::include_modules!();

/// The only places the launcher will ever open in a browser.
const ALLOWED_LINKS: &[&str] = &["https://wiki.downloadautism.com/"];

fn open_link(url: &str) {
    if !ALLOWED_LINKS.contains(&url) {
        eprintln!("launcher: refusing to open {url}");
        return;
    }
    // xdg-open is the standard way on Linux desktops; no shell is involved.
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

/// Rebuilds the Settings page rows from the model.
fn rows(model: &settings::Settings) -> Vec<SettingRow> {
    let mut last = "";
    catalog::SETTINGS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let header = if s.category != last { s.category } else { "" };
            last = s.category;
            SettingRow {
                key: s.key.into(),
                label: s.label.into(),
                hint: s.hint.into(),
                header: header.into(),
                checked: model.values[i],
            }
        })
        .collect()
}

fn main() -> Result<(), slint::PlatformError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let client_dir = client::find(&args);

    let ui = MainWindow::new()?;

    // ---- Play page ----
    match &client_dir {
        Some(dir) => {
            ui.set_client_found(true);
            ui.set_headline("Ready to play".into());
            let stamp = client::installed_stamp(dir).unwrap_or_else(|| "Installed".to_string());
            ui.set_subline(SharedString::from(stamp));
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
    let rows_model: Rc<VecModel<SettingRow>> = Rc::new(VecModel::default());
    ui.set_settings(ModelRc::from(rows_model.clone()));
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
                rows_model.set_vec(rows(m));
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

    // ---- Play ----
    {
        let dir = client_dir.clone();
        ui.on_play(move || {
            let Some(dir) = &dir else { return };
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
                let _ = slint::quit_event_loop();
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

    // The window has no system title bar, so the header starts the move itself.
    let weak = ui.as_weak();
    ui.on_drag_window(move || {
        if let Some(ui) = weak.upgrade() {
            ui.window().with_winit_window(|w| {
                let _ = w.drag_window();
            });
        }
    });

    ui.on_open_link(|url| open_link(url.as_str()));

    // Development aid: start on a given tab (0 Play, 1 Settings, 2 Diagnostics), e.g. for screenshots.
    if let Some(tab) = std::env::var("FTA_LAUNCHER_TAB").ok().and_then(|v| v.parse::<i32>().ok()) {
        ui.set_tab(tab);
    }

    #[cfg(feature = "dev-snapshot")]
    if let Ok(path) = std::env::var("FTA_LAUNCHER_SNAPSHOT") {
        let weak = ui.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_millis(1200), move || {
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
