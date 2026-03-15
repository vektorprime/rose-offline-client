use bevy::asset::Asset;
use bevy::prelude::{Assets, Commands, EventReader, EventWriter, Local, Res, ResMut};
use bevy_egui::{egui, EguiContexts};
use egui::text::LayoutJob;

use rose_game_common::components::ItemSlot;

use crate::{
    events::QuestScrollEvent,
    resources::UiResources,
    ui::{
        widgets::{Dialog, DrawWidget, Widget},
        DataBindings, UiSoundEvent,
    },
};

const IID_IMAGE_TOP: i32 = 5;
const IID_IMAGE_MIDDLE: i32 = 6;
const IID_IMAGE_BOTTOM: i32 = 7;
const IID_BUTTON_OK: i32 = 255;
const IID_BUTTON_CANCEL: i32 = 256;

pub struct ActiveQuestScrollDialog {
    id: egui::Id,
    has_set_position: bool,
    dialog_instance: DialogInstance,
    title_layout_job: LayoutJob,
    description_layout_job: LayoutJob,
    item_slot: ItemSlot,
    quest_trigger: String,
}

struct DialogInstance {
    xml_name: &'static str,
}

impl DialogInstance {
    pub fn new(xml_name: &'static str) -> Self {
        Self { xml_name }
    }
}

#[derive(Default)]
pub struct UiStateQuestScroll {
    active: Option<ActiveQuestScrollDialog>,
}

pub fn ui_quest_scroll_system(
    mut commands: Commands,
    mut ui_state: Local<UiStateQuestScroll>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    mut egui_context: EguiContexts,
    mut quest_scroll_events: EventReader<QuestScrollEvent>,
    mut quest_scroll_events_writer: EventWriter<QuestScrollEvent>,
    mut dialog_assets: ResMut<Assets<Dialog>>,
    ui_resources: Res<UiResources>,
) {
    let mut dialog = if let Some(dialog) = dialog_assets.get_mut(&ui_resources.dialog_message_box) {
        dialog
    } else {
        return;
    };

    let image_top_height = if let Some(Widget::Image(image)) = dialog.get_widget(IID_IMAGE_TOP) {
        image.height
    } else {
        26.0
    };

    let image_middle_height =
        if let Some(Widget::Image(image)) = dialog.get_widget(IID_IMAGE_MIDDLE) {
            image.height
        } else {
            22.0
        };

    let image_bottom_height =
        if let Some(Widget::Image(image)) = dialog.get_widget(IID_IMAGE_BOTTOM) {
            image.height
        } else {
            59.0
        };

    // Handle incoming quest scroll events
    for event in quest_scroll_events.read() {
        if let QuestScrollEvent::Show { item_slot, quest_trigger } = event {
            // Cancel any currently open dialog
            if let Some(active) = ui_state.active.take() {
                // Send cancel event for the previous dialog
                quest_scroll_events_writer.send(QuestScrollEvent::Cancel);
            }

            let mut title_job = egui::text::LayoutJob::default();
            let title_format = egui::text::TextFormat {
                color: egui::Color32::WHITE,
                font_id: egui::FontId::proportional(16.0),
                ..Default::default()
            };
            title_job.append(&format!("Quest: {}", quest_trigger), 0.0, title_format);

            let mut desc_job = egui::text::LayoutJob::default();
            let desc_format = egui::text::TextFormat {
                color: egui::Color32::WHITE,
                font_id: egui::FontId::proportional(14.0),
                ..Default::default()
            };
            desc_job.wrap.max_width = dialog.width - 16.0;
            desc_job.append("Quest Scroll - Accept this quest?", 0.0, desc_format.clone());

            let id = egui::Id::new("quest_scroll_dialog");

            ui_state.active = Some(ActiveQuestScrollDialog {
                id,
                dialog_instance: DialogInstance::new("MSGBOX.XML"),
                has_set_position: false,
                title_layout_job: title_job,
                description_layout_job: desc_job,
                item_slot: *item_slot,
                quest_trigger: quest_trigger.clone(),
            });
        }
    }

    // Show modal overlay if dialog is active
    if ui_state.active.is_some() {
        egui::Area::new(egui::Id::new("modal_quest_scroll"))
            .interactable(true)
            .fixed_pos(egui::Pos2::ZERO)
            .show(egui_context.ctx_mut(), |ui| {
                let interceptor_rect = ui.ctx().input(|input| input.screen_rect());

                ui.allocate_response(interceptor_rect.size(), egui::Sense::click_and_drag());
                ui.allocate_ui_at_rect(interceptor_rect, |ui| {
                    ui.painter().add(egui::epaint::Shape::rect_filled(
                        interceptor_rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 144),
                    ));
                });
            });
    }

    // Render and handle the active dialog
    if let Some(active_dialog) = ui_state.active.as_mut() {
         let dialog = if let Some(dialog) = get_dialog(&mut dialog_assets, &ui_resources) {
            dialog
        } else {
            return;
        };

        let (title_galley, description_galley, num_image_middle) =
            egui_context.ctx_mut().fonts(|fonts| {
                let title_galley = fonts.layout_job(active_dialog.title_layout_job.clone());
                let description_galley = fonts.layout_job(active_dialog.description_layout_job.clone());
                let description_size = description_galley.size();
                let num_image_middle = 1 + (description_size.y / image_middle_height) as usize;
                (title_galley, description_galley, num_image_middle)
            });

        let dialog_width = dialog.width;
        let dialog_height =
            image_top_height + image_middle_height * num_image_middle as f32 + image_bottom_height;

        let screen_size = egui_context
            .ctx_mut()
            .input(|input| input.screen_rect().size());
        let default_x = screen_size.x / 2.0 - dialog_width / 2.0;
        let default_y = screen_size.y / 2.0 - dialog_height / 2.0;

        if let Some(Widget::Image(image)) = dialog.get_widget_mut(IID_IMAGE_MIDDLE) {
            image.y = image_top_height;
        }

        if let Some(Widget::Image(image)) = dialog.get_widget_mut(IID_IMAGE_BOTTOM) {
            image.y = image_top_height + image_middle_height * num_image_middle as f32;
        }

        // Position buttons - accept on left, cancel on right
        if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BUTTON_OK) {
            button.x = dialog_width / 4.0 - button.width / 2.0;
            button.y = image_top_height
                + image_middle_height * num_image_middle as f32
                + image_bottom_height / 2.0
                - button.height / 2.0;
        }

        if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BUTTON_CANCEL) {
            button.x = dialog_width * 3.0 / 4.0 - button.width / 2.0;
            button.y = image_top_height
                + image_middle_height * num_image_middle as f32
                + image_bottom_height / 2.0
                - button.height / 2.0;
        }

        let mut response_button_ok = None;
        let mut response_button_cancel = None;

        let mut area = egui::Area::new(active_dialog.id)
            .movable(true)
            .interactable(true)
            .default_pos([default_x, default_y])
            .order(egui::Order::Foreground);

        if !active_dialog.has_set_position {
            area = area.current_pos([default_x, default_y]);
            active_dialog.has_set_position = true;
        }

        area.show(egui_context.ctx_mut(), |ui| {
            let response = ui.allocate_response(
                egui::vec2(dialog_width, dialog_height),
                egui::Sense::hover(),
            );

            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    visible: &mut [(IID_BUTTON_OK, false), (IID_BUTTON_CANCEL, false)],
                    response: &mut [
                        (IID_BUTTON_OK, &mut response_button_ok),
                        (IID_BUTTON_CANCEL, &mut response_button_cancel),
                    ],
                    ..Default::default()
                },
                |ui, bindings| {
                    // Draw middle images if needed
                    if let Some(Widget::Image(image)) = dialog.get_widget(IID_IMAGE_MIDDLE) {
                        if let Some(sprite) = image.sprite.as_ref() {
                            let mut pos = ui.min_rect().min;
                            pos.y += image_top_height;

                            for _ in 1..num_image_middle {
                                pos.y += image_middle_height;
                                sprite.draw(ui, pos);
                            }
                        }
                    }

                    // Draw title
                    let title_rect = egui::Rect::from_min_size(
                        ui.min_rect().min + egui::vec2(8.0, image_top_height + 4.0),
                        egui::vec2(dialog_width - 16.0, title_galley.size().y),
                    );
                    ui.allocate_ui_at_rect(title_rect, |ui| {
                        ui.add(egui::Label::new(title_galley.clone()));
                    });

                    // Draw description
                    let description_rect = egui::Rect::from_min_size(
                        ui.min_rect().min + egui::vec2(8.0, image_top_height + 16.0 + title_galley.size().y),
                        egui::vec2(dialog_width - 16.0, image_middle_height * num_image_middle as f32 - title_galley.size().y - 16.0),
                    );
                    ui.allocate_ui_at_rect(description_rect, |ui| {
                        ui.add(egui::Label::new(description_galley.clone()));
                    });

                    bindings.visible = &mut [];
                    if let Some(Widget::Button(button)) = dialog.get_widget(IID_BUTTON_OK) {
                        button.draw_widget(ui, bindings);
                    }
                    if let Some(Widget::Button(button)) = dialog.get_widget(IID_BUTTON_CANCEL) {
                        button.draw_widget(ui, bindings);
                    }
                },
            );

            response
        });

        // Handle button clicks
        if response_button_ok.map_or(false, |x| x.clicked()) {
            let dialog = ui_state.active.take().unwrap();
            quest_scroll_events_writer.send(QuestScrollEvent::Confirm {
                quest_trigger: dialog.quest_trigger,
                item_slot: dialog.item_slot,
            });
            return;
        }

        if response_button_cancel.map_or(false, |x| x.clicked()) {
            let _ = ui_state.active.take();
            quest_scroll_events_writer.send(QuestScrollEvent::Cancel);
            return;
        }
    }
}

fn get_dialog<'a>(
    dialog_assets: &'a mut ResMut<Assets<Dialog>>,
    ui_resources: &Res<UiResources>,
) -> Option<&'a mut Dialog> {
    dialog_assets.get_mut(&ui_resources.dialog_message_box)
}
