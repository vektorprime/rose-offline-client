use bevy::{
    prelude::{Camera3d, DirectionalLight, Entity, Mut, With, World},
    window::PrimaryWindow,
};
use bevy_egui::EguiContext;

use crate::{components::PlayerCharacter, resources::DebugInspector, ui::UiStateDebugWindows};

pub fn ui_debug_entity_inspector_system(world: &mut World) {
    let Ok(mut egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single_mut(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    world.resource_scope(
        |world, mut ui_state_debug_windows: Mut<UiStateDebugWindows>| {
            world.resource_scope(|world, mut debug_inspector_state: Mut<DebugInspector>| {
                if !ui_state_debug_windows.object_inspector_open {
                    return;
                }

                egui::Window::new("Entity Inspector")
                    .open(&mut ui_state_debug_windows.object_inspector_open)
                    .resizable(true)
                    .vscroll(true)
                    .show(egui_context.get_mut(), |ui| {
                        ui.style_mut().wrap = Some(false);

                        ui.horizontal(|ui| {
                            if ui.button("Camera").clicked() {
                                if let Ok(entity) = world
                                    .query_filtered::<Entity, With<Camera3d>>()
                                    .get_single(world)
                                {
                                    debug_inspector_state.entity = Some(entity);
                                }
                            }

                            if ui.button("Player").clicked() {
                                if let Ok(entity) = world
                                    .query_filtered::<Entity, With<PlayerCharacter>>()
                                    .get_single(world)
                                {
                                    debug_inspector_state.entity = Some(entity);
                                }
                            }

                            if ui.button("Light").clicked() {
                                if let Ok(entity) = world
                                    .query_filtered::<Entity, With<DirectionalLight>>()
                                    .get_single(world)
                                {
                                    debug_inspector_state.entity = Some(entity);
                                }
                            }
                        });

                        let mut enable_picking = debug_inspector_state.enable_picking;
                        ui.checkbox(&mut enable_picking, "Enable Picking (with P key)");
                        if enable_picking != debug_inspector_state.enable_picking {
                            debug_inspector_state.enable_picking = enable_picking;
                        }
                        ui.separator();

                        if let Some(entity) = debug_inspector_state.entity {
                            // bevy_inspector_egui::bevy_inspector::ui_for_entity(world, entity, ui);
                            ui.label(format!("Entity inspector disabled due to bevy_inspector_egui version conflict"));
                        }
                    });
            });
        },
    );
}
