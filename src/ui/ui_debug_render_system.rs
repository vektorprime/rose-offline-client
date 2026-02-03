use bevy::{
    gizmos::config::GizmoConfigStore,
    hierarchy::Children,
    prelude::{Assets, Handle, Local, Query, ResMut, With},
};
use bevy_egui::{egui, EguiContexts};

use crate::{
    components::{EventObject, WarpObject},
    resources::DebugRenderConfig,
    ui::UiStateDebugWindows,
};

#[derive(Default)]
pub struct UiStateDebugRender {
    pub render_event_objects: bool,
    pub render_warp_objects: bool,
}

pub fn ui_debug_render_system(
    mut egui_context: EguiContexts,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_debug_render: Local<UiStateDebugRender>,
    mut debug_render_config: ResMut<DebugRenderConfig>,
    query_event_objects: Query<&Children, With<EventObject>>,
    query_warp_objects: Query<&Children, With<WarpObject>>,
    rapier_debug: Option<ResMut<bevy_rapier3d::prelude::DebugRenderContext>>,
    mut gizmo_config_store: ResMut<GizmoConfigStore>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Debug Render")
        .open(&mut ui_state_debug_windows.debug_render_open)
        .show(egui_context.ctx_mut(), |ui| {
            ui.checkbox(&mut debug_render_config.colliders, "Show Colliders");
            if let Some(mut rapier_debug) = rapier_debug {
                ui.checkbox(&mut rapier_debug.enabled, "Show Rapier Debug");
            }
            ui.checkbox(&mut debug_render_config.skeleton, "Show Skeletons");
            ui.checkbox(&mut debug_render_config.bone_up, "Show Bone Up");
            ui.checkbox(
                &mut debug_render_config.directional_light_frustum,
                "Show Directional Light Frustum",
            );
            ui.checkbox(
                &mut debug_render_config.directional_light_frustum_freeze,
                "Freeze Render Directional Light Frustum",
            );

            if ui
                .checkbox(
                    &mut ui_state_debug_render.render_event_objects,
                    "Show Event Objects",
                )
                .clicked()
            {
                // TODO: Event object alpha transparency removed with old material system
                // This functionality needs to be reimplemented with new ExtendedMaterial pattern
            }

            if ui
                .checkbox(
                    &mut ui_state_debug_render.render_warp_objects,
                    "Show Warp Objects",
                )
                .clicked()
            {
                // TODO: Warp object alpha transparency removed with old material system
                // This functionality needs to be reimplemented with new ExtendedMaterial pattern
            }

            ui.separator();
            // TODO: GizmoConfig fields changed in Bevy 0.13 - line_width and depth_bias no longer exist
            // ui.label("Gizmo line width:");
            // ui.add(egui::Slider::new(&mut gizmo_config_store.config_mut::<DefaultGizmoConfigGroup>().0.line_width, 1.0..=10.0).show_value(true));
            // ui.label("Gizmo depth bias:");
            // ui.add(egui::Slider::new(&mut gizmo_config_store.config_mut::<DefaultGizmoConfigGroup>().0.depth_bias, -1.0..=1.0).show_value(true));
        });
}
