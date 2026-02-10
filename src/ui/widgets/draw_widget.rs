use bevy_egui::egui;
use serde::{Deserialize, Serialize};

use crate::resources::UiResources;

use super::{DataBindings, DrawWidget, GetWidget, LoadWidget, Widget};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "DRAW")]
pub struct Draw {
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
}

widget_to_rect! { Draw }

impl LoadWidget for Draw {
    fn load_widget(&mut self, _ui_resources: &UiResources) {
        log::trace!("[DRAW LOAD] Loading Draw widget: id={}, name='{}'",
            self.id, self.name);
    }
}

impl DrawWidget for Draw {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        log::trace!("[DRAW DRAW] Drawing Draw widget: id={}, name='{}'",
            self.id, self.name);

        if !bindings.get_visible(self.id) {
            return;
        }

        let rect = self.widget_rect(ui.min_rect().min);
        ui.allocate_rect(rect, egui::Sense::hover());
    }
}

impl GetWidget for Draw {
    fn get_widget(&self, _id: i32) -> Option<&Widget> {
        None
    }

    fn get_widget_mut(&mut self, _id: i32) -> Option<&mut Widget> {
        None
    }
}
