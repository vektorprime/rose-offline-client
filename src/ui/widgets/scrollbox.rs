use bevy_egui::egui;
use serde::Deserialize;

use crate::{
    resources::{UiResources, UiSprite},
    ui::widgets::DrawWidget,
};

use super::{DataBindings, LoadWidget};

#[derive(Clone, Default, Deserialize)]
#[serde(rename = "SCROLLBOX")]
#[serde(default)]
pub struct Scrollbox {
    #[serde(rename = "ID")]
    pub id: i32,
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "X")]
    pub x: f32,
    #[serde(rename = "Y")]
    pub y: f32,
    #[serde(rename = "OFFSETX")]
    pub offset_x: f32,
    #[serde(rename = "OFFSETY")]
    pub offset_y: f32,
    #[serde(rename = "WIDTH")]
    pub width: f32,
    #[serde(rename = "HEIGHT")]
    pub height: f32,
    #[serde(rename = "MODULEID")]
    pub module_id: i32,
    #[serde(rename = "TICKMOVE")]
    pub tick_move: i32,
    #[serde(rename = "GID")]
    pub sprite_name: String,
    #[serde(rename = "BLINKGID")]
    pub blink_sprite_name: String,
    #[serde(rename = "BLINK")]
    pub is_blink: i32,
    #[serde(rename = "BLINKMID")]
    pub blink_mid: i32,
    #[serde(rename = "BLINKSWAPTIME")]
    pub blink_swap_time: i32,

    #[serde(skip)]
    pub sprite: Option<UiSprite>,
    #[serde(skip)]
    pub blink_sprite: Option<UiSprite>,
}

widget_to_rect! { Scrollbox }

impl LoadWidget for Scrollbox {
    fn load_widget(&mut self, ui_resources: &UiResources) {
        self.sprite = ui_resources.get_sprite(self.module_id, &self.sprite_name);
        self.blink_sprite = ui_resources.get_sprite(self.module_id, &self.blink_sprite_name);
    }
}

impl DrawWidget for Scrollbox {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        if !bindings.get_visible(self.id) {
            return;
        }

        let rect = self.widget_rect(ui.min_rect().min);
        let response = ui.allocate_rect(rect, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            if let Some(sprite) = self.sprite.as_ref() {
                sprite.draw(ui, rect.min);
            }
        }

        bindings.set_response(self.id, response);
    }
}
