//! The project extension is kept at the desktop boundary to simplify upstream upgrades.
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "project-config.json";

fn parse_url(value: &str) -> Result<Option<String>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let url = url::Url::parse(value).map_err(|e| format!("Invalid webviewUrl: {e}"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("webviewUrl must be an http:// or https:// URL".into());
    }
    Ok(Some(url.into()))
}

fn config_path(executable: Option<&Path>, working: Option<&Path>) -> Option<PathBuf> {
    executable
        .and_then(Path::parent)
        .map(|dir| dir.join(CONFIG_FILE))
        .filter(|path| path.is_file())
        .or_else(|| working.and_then(|dir| dir.ancestors().map(|p| p.join(CONFIG_FILE)).find(|p| p.is_file())))
}

/// The parsed `project-config.json` next to the executable (or up the working directory), or
/// `None` when the project ships without one. Size-capped: config files are input-derived data.
fn read_config() -> Result<Option<serde_json::Value>, String> {
    let exe = std::env::current_exe().ok();
    let cwd = std::env::current_dir().ok();
    let Some(path) = config_path(exe.as_deref(), cwd.as_deref()) else { return Ok(None) };
    let file = std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    use std::io::Read;
    let mut data = Vec::new();
    file.take(64 * 1024 + 1).read_to_end(&mut data).map_err(|e| format!("Read {}: {e}", path.display()))?;
    if data.len() > 64 * 1024 {
        return Err("project-config.json exceeds 64 KiB".into());
    }
    serde_json::from_slice(&data).map(Some).map_err(|e| format!("{}: {e}", path.display()))
}

fn configured_url() -> Result<Option<String>, String> {
    if let Some(value) = std::env::var_os("CADCRAFT_WEBVIEW_URL") {
        return parse_url(&value.to_string_lossy());
    }
    let Some(value) = read_config()? else { return Ok(None) };
    parse_url(value.get("webviewUrl").and_then(|v| v.as_str()).ok_or("project-config.json requires a string webviewUrl")?)
}

/// `controlPort` from a parsed `project-config.json`: absent or `null` leaves the channel off;
/// anything that is not a TCP port (1–65535) is an error, so a typo doesn't silently ship a
/// build without its agent channel.
fn parse_control_port(value: &serde_json::Value) -> Result<Option<u16>, String> {
    match value.get("controlPort") {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .and_then(|p| u16::try_from(p).ok())
            .filter(|p| *p != 0)
            .map(Some)
            .ok_or_else(|| format!("controlPort must be a port number (1-65535), got {v}")),
    }
}

/// The control port a packaged build opens automatically (`controlPort` in `project-config.json`),
/// so the MCP bridge (`cadcraft-cli mcp --connect`) can reach the app without command-line flags.
/// A broken config is logged and treated as off: start-up never fails over it.
pub fn configured_control_port() -> Option<u16> {
    match read_config().and_then(|v| v.as_ref().map_or(Ok(None), parse_control_port)) {
        Ok(port) => port,
        Err(e) => {
            log::warn!("project-config.json: {e}; the control channel is off");
            None
        }
    }
}

#[cfg(any(target_os = "windows", test))]
fn prepare_data_dir(primary: &Path, fallback: Option<&Path>) -> Result<PathBuf, String> {
    let prepare = |dir: &Path| -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("Create WebView data directory {}: {e}", dir.display()))?;
        // An existing directory can still be read-only. Check before passing it to WebView2.
        let probe = dir.join(format!(".write-check-{}", std::process::id()));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
            .map_err(|e| format!("Write WebView data directory {}: {e}", dir.display()))?;
        drop(file);
        std::fs::remove_file(&probe).map_err(|e| format!("Remove WebView write check {}: {e}", probe.display()))?;
        Ok(())
    };
    match prepare(primary) {
        Ok(()) => Ok(primary.to_owned()),
        Err(first) => {
            let Some(fallback) = fallback.filter(|p| *p != primary) else { return Err(first) };
            prepare(fallback).map_err(|second| format!("{first}\n{second}"))?;
            log::warn!("{first}; using WebView data directory {}", fallback.display());
            Ok(fallback.to_owned())
        }
    }
}

#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use cadcraft_ui_egui::webview::State;

    #[derive(Default)]
    pub struct Host {
        view: Option<wry::WebView>,
        context: Option<wry::WebContext>,
        attempted: bool,
        visible: bool,
        bounds: Option<(egui::Rect, f32)>,
    }

    impl Host {
        pub fn update(&mut self, state: &mut State, ctx: &egui::Context, frame: &eframe::Frame) {
            if std::mem::take(&mut state.reload) {
                // Re-read project variables on Reload, including after a failed initialization.
                self.view = None;
                self.context = None;
                self.attempted = false;
                self.visible = false;
                self.bounds = None;
            }
            let target = state.rect.filter(|rect| {
                !ctx.input(|i| i.viewport().minimized.unwrap_or(false))
                    && !ctx.memory(|memory| {
                        // Native child windows paint above egui: hide while a dialog/menu covers them.
                        memory
                            .areas()
                            .visible_layer_ids()
                            .iter()
                            .any(|layer| layer.order > egui::Order::Background && memory.area_rect(layer.id).is_some_and(|r| r.intersects(*rect)))
                    })
            });
            let previous_message = state.message.clone();
            let result = self.sync(state, target, ctx.pixels_per_point(), frame);
            if previous_message != state.message {
                ctx.request_repaint();
            }
            if let Err(error) = result {
                log::warn!("Project WebView: {error}");
                self.view = None;
                self.visible = false;
                self.attempted = true;
                state.message = format!("{}\n{error}", cadcraft_ui_egui::i18n::t("Web panel unavailable. See the error below, then click Reload."));
                ctx.request_repaint();
            }
        }

        fn sync(&mut self, state: &mut State, target: Option<egui::Rect>, scale: f32, frame: &eframe::Frame) -> Result<(), String> {
            let Some(rect) = target else {
                if self.visible
                    && let Some(view) = &self.view
                {
                    view.set_visible(false).map_err(|e| e.to_string())?;
                    view.focus_parent().map_err(|e| e.to_string())?;
                    self.visible = false;
                }
                return Ok(());
            };
            let bounds = || wry::Rect {
                position: wry::dpi::PhysicalPosition::new((rect.left() * scale).round() as i32, (rect.top() * scale).round() as i32).into(),
                size: wry::dpi::PhysicalSize::new((rect.width() * scale).round() as u32, (rect.height() * scale).round() as u32).into(),
            };
            if !self.attempted {
                self.attempted = true;
                let Some(url) = configured_url()? else {
                    state.message = cadcraft_ui_egui::i18n::t("Configure webviewUrl in project-config.json, then click Reload.").into();
                    return Ok(());
                };
                let primary = crate::config_dir().unwrap_or_else(std::env::temp_dir).join("project-webview");
                let fallback = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.join("project-webview")));
                let data_dir = prepare_data_dir(&primary, fallback.as_deref())?;
                log::info!("Project WebView data directory: {}", data_dir.display());
                let context = self.context.get_or_insert_with(|| wry::WebContext::new(Some(data_dir.clone())));
                let view = wry::WebViewBuilder::new_with_web_context(context)
                    .with_url(url)
                    .with_bounds(bounds())
                    .with_focused(false)
                    .with_navigation_handler(|url| parse_url(&url).is_ok_and(|u| u.is_some()))
                    .build_as_child(frame)
                    .map_err(|e| format!("Initialize WebView2 (data directory {}): {e}", data_dir.display()))?;
                self.view = Some(view);
                self.bounds = Some((rect, scale));
                self.visible = true;
                state.message.clear();
            }
            if let Some(view) = &self.view {
                if self.bounds != Some((rect, scale)) {
                    view.set_bounds(bounds()).map_err(|e| e.to_string())?;
                    self.bounds = Some((rect, scale));
                }
                if !self.visible {
                    view.set_visible(true).map_err(|e| e.to_string())?;
                    self.visible = true;
                }
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
pub use native::Host;

#[cfg(not(target_os = "windows"))]
#[derive(Default)]
pub struct Host;

#[cfg(not(target_os = "windows"))]
impl Host {
    pub fn update(&mut self, state: &mut cadcraft_ui_egui::webview::State, _ctx: &egui::Context, _frame: &eframe::Frame) {
        state.reload = false;
        state.message = match configured_url() {
            Err(e) => e,
            _ => "Embedded web panel is currently supported on Windows.".into(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_directory_falls_back_and_reports_both_failures() {
        let root = std::env::temp_dir().join(format!("cadcraft-webview-data-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let primary = root.join("primary");
        let fallback = root.join("fallback");
        assert_eq!(prepare_data_dir(&primary, Some(&fallback)).unwrap(), primary);
        assert!(!fallback.exists());
        assert_eq!(std::fs::read_dir(&primary).unwrap().count(), 0);
        std::fs::remove_dir(&primary).unwrap();
        std::fs::write(&primary, "blocked by a file").unwrap();
        assert_eq!(prepare_data_dir(&primary, Some(&fallback)).unwrap(), fallback);
        assert_eq!(std::fs::read_dir(&fallback).unwrap().count(), 0);
        std::fs::remove_dir(&fallback).unwrap();
        std::fs::write(&fallback, "also blocked").unwrap();
        let error = prepare_data_dir(&primary, Some(&fallback)).unwrap_err();
        assert!(error.contains(&primary.display().to_string()));
        assert!(error.contains(&fallback.display().to_string()));
        assert!(prepare_data_dir(&primary, None).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn url_configuration_accepts_local_services_and_rejects_non_web_schemes() {
        assert_eq!(parse_url("  ").unwrap(), None);
        assert_eq!(parse_url(" http://127.0.0.1:3000/app ").unwrap().as_deref(), Some("http://127.0.0.1:3000/app"));
        assert_eq!(parse_url("https://example.com/").unwrap().as_deref(), Some("https://example.com/"));
        for value in ["localhost:3000", "javascript:alert(1)", "file:///C:/test", "data:text/html,test", "https://"] {
            assert!(parse_url(value).is_err(), "{value}");
        }
    }

    #[test]
    fn control_port_accepts_ports_and_rejects_everything_else() {
        let cfg = |s: &str| serde_json::from_str::<serde_json::Value>(s).unwrap();
        assert_eq!(parse_control_port(&cfg("{}")), Ok(None));
        assert_eq!(parse_control_port(&cfg(r#"{"controlPort": null}"#)), Ok(None));
        assert_eq!(parse_control_port(&cfg(r#"{"controlPort": 7979}"#)), Ok(Some(7979)));
        assert_eq!(parse_control_port(&cfg(r#"{"controlPort": 65535}"#)), Ok(Some(65535)));
        for bad in
            [r#"{"controlPort": 0}"#, r#"{"controlPort": 65536}"#, r#"{"controlPort": -1}"#, r#"{"controlPort": "7979"}"#, r#"{"controlPort": 79.5}"#]
        {
            assert!(parse_control_port(&cfg(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn executable_config_precedes_working_directory_and_nested_launch_finds_root() {
        let root = std::env::temp_dir().join(format!("cadcraft-project-config-test-{}", std::process::id()));
        let bin = root.join("dist");
        let nested = root.join("cadcraft/apps");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join(CONFIG_FILE), "{}").unwrap();
        assert_eq!(config_path(Some(&bin.join("cadcraft.exe")), Some(&nested)), Some(root.join(CONFIG_FILE)));
        std::fs::write(bin.join(CONFIG_FILE), "{}").unwrap();
        assert_eq!(config_path(Some(&bin.join("cadcraft.exe")), Some(&nested)), Some(bin.join(CONFIG_FILE)));
        std::fs::remove_dir_all(root).unwrap();
    }
}
