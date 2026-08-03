# Combat Sync: Delayed Monster Death After One-Hit Kill

## Problem
A monster that only needs 1 hit to die would stay alive for ~5 seconds after its HP
hit 0 (no damage digits, no death animation), then suddenly die.

## Root Cause
The server is authoritative for death (`DamageEntity { is_killed }`), but the client
only *applied* pending damage when its own attack animation hit frame fired
(`HitEvent`). If no hit frame ever fired, the kill sat in `PendingDamageList` until
the 5-second fallback (`MAX_DAMAGE_AGE`) kicked in.

Two contributors:
1. **Server attack range mismatch**: the server executed attacks within `attack_range * 3`
   (a bot fix), while the client only plays the attack animation within `attack_range`.
   Clicking a monster between 1x-3x range meant the server killed it while the client
   was still running toward it - no attack animation, no hit frame, 5s delayed death.
2. **Death depended on a future cosmetic hit frame**: any interrupted animation
   (out-of-range conversion, stun, attacker dead, despawned projectile) delayed the
   kill by the full fallback timeout.

## Fix
- Server (`rose-offline-server/src/game/systems/command_system.rs`): removed the 3x
  attack range - attacks only execute within actual `attack_range`, moving toward the
  target when beyond (matching the client).
- Client (`src/systems/pending_damage_system.rs`): `is_kill` pending damage applies
  immediately once no hit frame is expected (attacker not mid Attack/CastSkill
  animation, no projectile from the attacker in flight toward the target), plus a
  `KILL_MAX_DAMAGE_AGE = 1.5s` cap. Normal in-range kills still sync to the hit frame.
- Client (`src/systems/command_system.rs`): attack range check is now inclusive
  (`distance <= attack_range`) so client and server attack at the same boundary.

## Lesson Learned
Server-authoritative death should never depend on a client-side cosmetic event (hit
frame) that may never fire. Kill decisions should be applied with at most a short
grace period, while keeping the hit-frame sync only as the preferred path. Also,
server and client must agree on attack range exactly - "generous" server ranges
desync the client's attack animation.
