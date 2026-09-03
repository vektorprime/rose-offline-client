use bevy::prelude::{ChildOf, Entity, Query};

use rose_game_common::components::{AbilityValues, HealthPoints};

use crate::{components::NameTagHealthbarForeground, render::WorldUiRect};

pub fn name_tag_update_healthbar_system(
    mut query_nametag_healthbar: Query<(&ChildOf, &NameTagHealthbarForeground, &mut WorldUiRect)>,
    query_parent: Query<&ChildOf>,
    query_health: Query<(&HealthPoints, &AbilityValues)>,
) {
    for (parent, name_tag_healthbar_fg, mut rect) in query_nametag_healthbar.iter_mut() {
        let parent_entity: Entity = parent.0;
        if let Ok((health_points, ability_values)) = query_parent
            .get(parent_entity)
            .and_then(|parent| query_health.get(parent.0))
        {
            let max_hp = ability_values.get_max_health();
            if max_hp <= 0 {
                continue;
            }
            let health_percent =
                (health_points.hp as f32 / max_hp as f32).clamp(0.0, 1.0);

            // Epsilon guard: skip WorldUiRect writes (and downstream re-extract)
            // when health hasn't visibly changed since last frame.
            let new_uv_x = name_tag_healthbar_fg.uv_min_x
                + health_percent
                    * (name_tag_healthbar_fg.uv_max_x - name_tag_healthbar_fg.uv_min_x);
            let new_width = name_tag_healthbar_fg.full_width * health_percent;
            if (rect.uv_max.x - new_uv_x).abs() < 0.0005
                && (rect.screen_size.x - new_width).abs() < 0.05
            {
                continue;
            }
            rect.uv_max.x = new_uv_x;
            rect.screen_size.x = new_width;
        }
    }
}
