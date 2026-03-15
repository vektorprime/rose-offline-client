use bevy::prelude::{Commands, Query, Res, Time};
use rand::Rng;

use crate::{
    components::{BlinkClip, CharacterBlinkTimer, CharacterModel, CharacterModelPart, Dead},
};

pub fn character_model_blink_system(
    mut commands: Commands,
    mut query_characters: Query<(&CharacterModel, &mut CharacterBlinkTimer, Option<&Dead>)>,
    time: Res<Time>,
) {
    for (character_model, mut blink_timer, dead) in query_characters.iter_mut() {
        let mut changed = false;

        if dead.is_none() {
            blink_timer.timer += time.delta().as_secs_f32();

            if blink_timer.is_open {
                if blink_timer.timer >= blink_timer.open_duration {
                    blink_timer.is_open = false;
                    blink_timer.timer -= blink_timer.open_duration;
                    blink_timer.closed_duration =
                        rand::thread_rng().gen_range(CharacterBlinkTimer::BLINK_CLOSED_DURATION);
                    changed = true;
                }
            } else if blink_timer.timer >= blink_timer.closed_duration {
                blink_timer.is_open = true;
                blink_timer.timer -= blink_timer.closed_duration;
                blink_timer.open_duration =
                    rand::thread_rng().gen_range(CharacterBlinkTimer::BLINK_OPEN_DURATION);
                changed = true;
            }
        } else {
            if blink_timer.is_open {
                blink_timer.is_open = false;

                // Set timer so the eyes open as soon as resurrected
                blink_timer.closed_duration = 0.0;
                blink_timer.timer = 0.0;
            }

            changed = true;
        }

        if changed {
            // Apply BlinkClip component to face mesh entities when state changes
            // This tracks blink state per entity for potential shader-based clipping
            let blink_clip = if blink_timer.is_open { 
                BlinkClip::EyesOpen 
            } else { 
                BlinkClip::EyesClosed 
            };

            // Insert the BlinkClip component on all face model part entities
            for &face_entity in character_model.model_parts[CharacterModelPart::CharacterFace].1.iter() {
                commands.entity(face_entity).insert(blink_clip);
            }
        }
    }
}
