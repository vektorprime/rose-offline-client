# Networking

Client networking for the irose protocol only (`--data-version/--network-version/--ui-version` accept `irose` alone). Protocol codecs live in `src/protocol/irose/`; threading and Bevy integration live in systems/resources/events.

## Connections

- Resources: `src/resources/{login_connection,world_connection,game_connection,network_thread}.rs` (`LoginConnection`, `WorldConnection`, `GameConnection`, `NetworkThread`).
- Systems (all in `PreUpdate`, before gameplay): `src/systems/{login_connection_system,world_connection_system,game_connection_system}.rs`; `game_connection_system` is gated on `resource_exists::<CurrentZone>`.
- `src/systems/network_thread_system.rs` pumps the background IO thread and exposes `handle_connection_lost` (reused by all three connection systems).
- Events: `src/events/{login_event,world_connection_event,game_connection_event,network_event}.rs`.

## Message flow

- Outgoing: gameplay systems send via `GameConnection::client_message_tx` (e.g. `ClientMessage::Chat`, `ClientMessage::MoveCollision`, `ClientMessage::WarpGateRequest`).
- Incoming: connection systems translate server packets into Bevy messages/entities — e.g. `SpawnEntityMonster` in `src/systems/game_connection_system.rs` inserts `MonsterSeparation` at spawn; `JoinZone` handling spawns the player bundle.
- Movement stays server-authoritative: `collision_system.rs` never mutates `Position` on wall hits; it stops intent (`NextCommand::with_stop()`) and reports `MoveCollision` for validation. Flight is the exception and reports the same message (see [flying-system-architecture.md](flying-system-architecture.md)).

## Debugging

- Start with `pitfalls/networking.md` (thread, respawn, reconnect notes).
- Every session logs to `logs/<timestamp>/{structured.jsonl,session.json}`; connection loss funnels through `handle_connection_lost`.
