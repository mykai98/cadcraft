//! The project browser leaves room for drawing; the consolidated palette tabs stay usable.
use cadcraft_engine::Session;
use cadcraft_ui_egui::{
    CadApp, Services,
    docking::{self, Panel},
};
use egui_kittest::{Harness, kittest::Queryable};

#[test]
fn browser_bounds_scale_collapse_and_palette_commands() {
    for width in [900.0, 1600.0] {
        for scale in [1.0, 1.5, 2.0] {
            let mut app = CadApp::new(Session::default(), Services::default());
            app.webview.enabled = true;
            let mut h = Harness::builder().with_size(egui::vec2(width, 900.0)).with_pixels_per_point(scale).build_ui_state(
                |ui, app: &mut CadApp| {
                    app.logic(ui.ctx());
                    app.ui(ui);
                },
                app,
            );
            h.run_steps(5);
            if width == 1600.0
                && scale == 1.0
                && let Some(path) = std::env::var_os("CADCRAFT_LAYOUT_SCREENSHOT")
            {
                h.render().unwrap().save(path).unwrap();
            }
            let browser = h.state().webview.rect.unwrap();
            assert!(browser.width() >= 160.0 && browser.height() > 400.0);
            assert!(browser.left() >= 0.0 && browser.right() < width - 300.0);
            let tools = h.get_by_role_and_label(egui::accesskit::Role::Tab, "Tool Sets").rect();
            assert!(tools.left() > browser.right());
            h.get_by_role_and_label(egui::accesskit::Role::Tab, "Layers").click();
            h.run_steps(3);
            h.get_by_role_and_label(egui::accesskit::Role::Tab, "Properties").click();
            h.run_steps(3);
            h.get_by_role_and_label(egui::accesskit::Role::Tab, "Tool Sets").click();
            h.run_steps(3);
            h.get_by_label("Line").click();
            h.run_steps(3);
            assert_eq!(h.state().session.running.as_ref().map(|c| c.id.as_str()), Some("line"));
            h.state_mut().webview.collapsed = true;
            h.run_steps(3);
            assert!(h.state().webview.rect.is_none());
            h.state_mut().webview.collapsed = false;
            h.run_steps(3);
            assert!(h.state().webview.rect.is_some());
        }
    }
}

#[test]
fn default_and_reset_group_palettes_right_and_preserve_canvas_rules() {
    let mut app = CadApp::new(Session::default(), Services::default());
    app.ui.docking = docking::legacy_workspace();
    docking::command(&mut app, true, &serde_json::Value::Null).unwrap();
    docking::validate(&app.ui.docking).unwrap();
    let json = serde_json::to_value(&app.ui.docking.layout).unwrap();
    assert_eq!(json["root"]["Split"]["second"]["Tabs"]["panels"], serde_json::json!(["toolSets", "layers", "properties"]));
    assert_eq!(json["root"]["Split"]["first"]["Tabs"]["panels"], serde_json::json!(["canvas"]));
    assert!(!docking::permissions(&Panel::Canvas).closable);
}
