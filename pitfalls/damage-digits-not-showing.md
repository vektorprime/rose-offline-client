# Damage Digits Not Showing - Investigation Notes

## Investigation Date
2026-03-26

## Summary
After investigation, I found that damage digits are NOT being spawned when monsters/entities take damage from server damage packets. The system had two parallel code paths:

1. **hit_event_system** - Handles damage from animation events (client-side attacks), spawns damage digits correctly
2. **pending_damage_system** - Handles damage from server packets (`ServerMessage::DamageEntity`), was NOT spawning damage digits

## Root Cause
The `pending_damage_system` in `src/systems/pending_damage_system.rs` was missing the call to spawn damage digits. It only:
- Applied damage to health points
- Handled kill state

But it never called `damage_digits_spawner.spawn()` like `hit_event_system` does.

Additionally, logging was set to `info` level by default, which filtered out debug messages.

## Solution Applied
Modified `src/systems/pending_damage_system.rs` to:
1. Query `GlobalTransform` and `ModelHeight` components
2. Include `DamageDigitsSpawner` resource
3. Call `damage_digits_spawner.spawn()` when applying damage
4. Changed logging to `info` level for visibility

Modified `src/systems/game_connection_system.rs` to:
1. Add logging when receiving `ServerMessage::DamageEntity` to trace incoming damage

## Files Modified
- `src/systems/pending_damage_system.rs` - Added damage digit spawning to pending damage system
- `src/systems/game_connection_system.rs` - Added logging for incoming damage packets

## Testing
Run the game and fight monsters. Check for:
1. `[GAME_CONNECTION] Received DamageEntity` - shows when server sends damage
2. `[PENDING_DAMAGE] Applying damage` - shows when pending damage system processes it
3. Damage digits should appear above entities

## Notes
- The original C++ client spawns damage digits in `CObjCHAR::CreateImmediateDigitEffect` which is called when receiving damage packets
- This matches the behavior now implemented in `pending_damage_system`
- The `hit_event_system` path remains unchanged as it's used for client-side attack animations