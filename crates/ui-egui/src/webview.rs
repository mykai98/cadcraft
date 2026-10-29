//! Layout contract between egui and the desktop's native browser host.
//! No browser or OS dependencies belong in this crate.

#[derive(Default)]
pub struct State {
    pub enabled: bool,
    pub collapsed: bool,
    pub rect: Option<egui::Rect>,
    pub message: String,
    pub reload: bool,
}

impl State {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.rect = None;
        if !self.enabled {
            return;
        }
        let t = crate::theme::Tokens::get();
        if self.collapsed {
            egui::Panel::left("project-webview-collapsed").exact_size(28.0).resizable(false).show(ui, |ui| {
                if ui.button("»").on_hover_text(crate::tl!("Web panel")).clicked() {
                    self.collapsed = false;
                }
            });
            return;
        }
        let max_width = (ui.available_width() - 620.0).max(180.0);
        egui::Panel::left("project-webview")
            .default_size(340.0)
            .size_range(180.0..=max_width)
            .resizable(true)
            .frame(egui::Frame::NONE.fill(t.panel).inner_margin(6.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(crate::tl!("Web panel"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("«").on_hover_text(crate::tl!("Collapse web panel")).clicked() {
                            self.collapsed = true;
                        }
                        if ui.button(crate::tl!("Reload")).clicked() {
                            self.reload = true;
                        }
                    });
                });
                ui.separator();
                let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
                if !self.collapsed && rect.is_positive() {
                    self.rect = Some(rect);
                    ui.label(&self.message);
                }
            });
    }
}
