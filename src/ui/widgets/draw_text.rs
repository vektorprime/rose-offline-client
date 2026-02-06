use bevy_egui::egui;
use serde::{Deserialize, Serialize};

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, GetWidget, LoadWidget, Widget};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "DRAWTEXT")]
pub struct DrawText {
    #[serde(rename = "ID")]
    pub id: i32,
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "X")]
    pub x: f32,
    #[serde(rename = "Y")]
    pub y: f32,
    #[serde(rename = "WIDTH")]
    pub width: f32,
    #[serde(rename = "HEIGHT")]
    pub height: f32,
    #[serde(rename = "OFFSETX")]
    pub offset_x: f32,
    #[serde(rename = "OFFSETY")]
    pub offset_y: f32,
    #[serde(rename = "TEXT")]
    pub text: String,
}

widget_to_rect! { DrawText }

impl LoadWidget for DrawText {
    fn load_widget(&mut self, _ui_resources: &UiResources) {
        log::trace!("[DRAWTEXT LOAD] Loading DrawText widget: id={}, name='{}', text='{}'",
            self.id, self.name, self.text);
    }
}

impl DrawWidget for DrawText {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        log::trace!("[DRAWTEXT DRAW] Drawing DrawText widget: id={}, name='{}', text='{}'",
            self.id, self.name, self.text);

        if !bindings.get_visible(self.id) {
            return;
        }

        let rect = self.widget_rect(ui.min_rect().min);
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.add(egui::Label::new(&self.text));
        });
    }
}

impl GetWidget for DrawText {
    fn get_widget(&self, _id: i32) -> Option<&Widget> {
        None
    }

    fn get_widget_mut(&mut self, _id: i32) -> Option<&mut Widget> {
        None
    }
}
