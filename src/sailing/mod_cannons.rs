//! Boat Cannons and Projectiles
//!
//! Cannon system for player boats and pirate ships.

use bevy::prelude::*;

use crate::components::{BoatState, FacingDirection, Position, Projectile};

/// Cannon component for boats
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct BoatCannon {
    /// Cannon side (left/right)
    pub side: CannonSide,
    /// Fire cooldown timer
    pub fire_cooldown: Timer,
    /// Cannon range (meters)
    pub range: f32,
    /// Cannon damage
    pub damage: f32,
    /// Whether cannon is currently firing
    pub is_firing: bool,
}

impl Default for BoatCannon {
    fn default() -> Self {
        Self {
            side: CannonSide::Both,
            fire_cooldown: Timer::from_seconds(2.0, TimerMode::Once),
            range: 150.0,
            damage: 20.0,
            is_firing: false,
        }
    }
}

/// Cannon side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum CannonSide {
    Left,
    Right,
    Both,
}

/// Cannon projectile component
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct CannonProjectile {
    /// Damage on impact
    pub damage: f32,
    /// Velocity
    pub velocity: Vec3,
    /// Lifetime timer
    pub lifetime: Timer,
    /// Source boat entity
    pub source_entity: Entity,
    /// Whether this is a player cannonball
    pub is_player: bool,
}

/// Fire cannons when player presses fire key
pub fn cannon_fire_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cannon_query: Query<(
        &mut BoatCannon,
        &mut BoatState,
        &Position,
        &FacingDirection,
        Entity,
    )>,
    commands: &mut Commands,
) {
    // Fire on spacebar or F key
    let fire_pressed = keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::KeyF);
    
    if !fire_pressed {
        return;
    }

    for (mut cannon, boat, position, facing, entity) in cannon_query.iter_mut() {
        if !boat.active {
            continue;
        }

        cannon.fire_cooldown.tick(time.delta());
        if !cannon.fire_cooldown.just_finished() {
            continue;
        }

        // Fire cannonball
        let heading = facing.desired;
        let velocity = Vec3::new(
            heading.sin() * 50.0 * 100.0,
            heading.cos() * 50.0 * 100.0,
            0.0,
        );

        for side in [CannonSide::Left, CannonSide::Right] {
            if cannon.side == CannonSide::Both || cannon.side == side {
                let offset = match side {
                    CannonSide::Left => Vec3::new(-500.0, 0.0, 0.0),
                    CannonSide::Right => Vec3::new(500.0, 0.0, 0.0),
                    CannonSide::Both => Vec3::ZERO,
                };

                commands.spawn((
                    Name::new("Cannonball"),
                    CannonProjectile {
                        damage: cannon.damage,
                        velocity,
                        lifetime: Timer::from_seconds(3.0, TimerMode::Once),
                        source_entity: entity,
                        is_player: true,
                    },
                    Projectile::default(),
                    Position::new(position.position + offset),
                    Transform::default(),
                    GlobalTransform::default(),
                ));
            }
        }

        cannon.is_firing = true;
        cannon.fire_cooldown.reset();
        log::debug!("[cannon_fire_system] Fired cannon from boat");
    }
}

/// Update cannon projectile movement
pub fn cannon_projectile_system(
    time: Res<Time>,
    mut projectile_query: Query<(&mut CannonProjectile, &mut Position)>,
) {
    for (mut projectile, mut position) in projectile_query.iter_mut() {
        projectile.lifetime.tick(time.delta());

        if projectile.lifetime.finished() {
            // Projectile expired
            continue;
        }

        // Move projectile
        position.position += projectile.velocity * time.delta_secs();

        // Apply gravity (parabolic arc)
        projectile.velocity.z -= 9.8 * 100.0 * time.delta_secs();
    }
}

/// Handle cannon projectile impacts
pub fn cannon_impact_system(
    commands: &mut Commands,
    mut projectile_query: Query<(Entity, &mut CannonProjectile, &Position)>,
    boat_query: Query<(Entity, &Position), With<BoatState>>,
) {
    for (proj_entity, mut projectile, proj_position) in projectile_query.iter_mut() {
        // Check for impacts with boats
        for (boat_entity, boat_position) in boat_query.iter() {
            // Skip self
            if boat_entity == projectile.source_entity {
                continue;
            }

            let dx = proj_position.position.x - boat_position.position.x;
            let dy = proj_position.position.y - boat_position.position.y;
            let dz = proj_position.position.z - boat_position.position.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            if dist < 1000.0 { // 10m hit radius
                // Hit!
                log::debug!("[cannon_impact_system] Cannonball hit boat {} for {} damage", 
                    boat_entity, projectile.damage);
                
                // Would apply damage to boat here
                
                // Remove projectile
                commands.entity(proj_entity).despawn();
                break;
            }
        }

        // Remove if hit water surface
        if proj_position.position.z < -100.0 {
            commands.entity(proj_entity).despawn();
        }
    }
}
