use bevy::prelude::*;

use crate::components::{FlightState, PlayerCharacter};
use crate::events::FlightToggleEvent;

/// System that handles flight toggle events.
/// 
/// This system listens for [`FlightToggleEvent`] events and toggles the
/// [`FlightState::is_flying`] flag on the target entity. When enabling flight,
/// it initializes the flight state (sets current_speed to 0).
/// 
/// Wing spawning is handled separately in the wing spawn system.
pub fn flight_toggle_system(
    mut commands: Commands,
    mut events: EventReader<FlightToggleEvent>,
    mut query: Query<(Entity, &mut FlightState), With<PlayerCharacter>>,
) {
    for event in events.read() {
        // Try to get the FlightState component for the target entity
        if let Ok((entity, mut flight_state)) = query.get_mut(event.entity) {
            // Toggle flight state
            flight_state.is_flying = !flight_state.is_flying;
            
            if flight_state.is_flying {
                // Initialize flight state when entering flight mode
                flight_state.is_thrusting = false;
                flight_state.current_speed = 0.0;
                
                // Ensure FlightState component exists, if not add it
                // (This handles the case where the entity doesn't have FlightState yet)
                info!(
                    "Flight mode ENABLED for entity {:?}. Spread your wings!",
                    entity
                );
            } else {
                // Reset thrust state when exiting flight mode
                flight_state.is_thrusting = false;
                flight_state.current_speed = 0.0;
                
                // Clean up wing entities if they exist
                if let Some(wing_left) = flight_state.wing_entity_left {
                    commands.entity(wing_left).despawn();
                    flight_state.wing_entity_left = None;
                }
                if let Some(wing_right) = flight_state.wing_entity_right {
                    commands.entity(wing_right).despawn();
                    flight_state.wing_entity_right = None;
                }
                if let Some(wind_emitter) = flight_state.wind_emitter_entity {
                    commands.entity(wind_emitter).despawn();
                    flight_state.wind_emitter_entity = None;
                }
                
                info!(
                    "Flight mode DISABLED for entity {:?}. Wings retracted.",
                    entity
                );
            }
        } else {
            warn!(
                "FlightToggleEvent received for entity {:?} but it has no FlightState component",
                event.entity
            );
        }
    }
}

/// System that ensures the player has a FlightState component.
/// This should run before the flight_toggle_system.
pub fn ensure_flight_state_system(
    mut commands: Commands,
    query: Query<Entity, (With<PlayerCharacter>, Without<FlightState>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(FlightState::default());
        debug!("Added FlightState component to player entity {:?}", entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flight_state_default() {
        let state = FlightState::default();
        assert!(!state.is_flying);
        assert!(!state.is_thrusting);
        assert_eq!(state.current_speed, 0.0);
        assert!(state.wing_entity_left.is_none());
        assert!(state.wing_entity_right.is_none());
        assert!(state.wind_emitter_entity.is_none());
    }
}
