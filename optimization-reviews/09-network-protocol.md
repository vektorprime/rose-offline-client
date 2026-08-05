# Optimization Review 09 — Network & Protocol Subsystem

## 1. Title + Scope Summary

**Topic:** Network and protocol subsystem: network thread, message encoding/decoding, login/world/game connections, ping, event routing, thread communication.

**Scope covered in this review:**
- `src/protocol/` — `mod.rs`, `irose/mod.rs`, `irose/game_client.rs` (~1368 lines), `irose/login_client.rs`, `irose/world_client.rs`
- `src/resources/network_thread.rs`, `game_connection.rs`, `login_connection.rs`, `world_connection.rs`, `client_entity_list.rs`
- `src/systems/network_thread_system.rs`, `game_connection_system.rs` (~2933 lines), `login_connection_system.rs`, `world_connection_system.rs`, `ping_command_system.rs`
- `src/events/` — `mod.rs`, `network_event.rs`, `ping_event.rs`, `game_connection_event.rs`, `world_connection_event.rs`, `login_event.rs`
- `src/lib.rs` (registration + thread startup/teardown), `src/logging/mod.rs` (log level), `src/ui/ui_chatbox_system.rs` (ping/chat senders)
- Network crate sources: `rose-network-common` (`connection.rs`, `packet.rs`), `rose-network-irose` (`packet_codec.rs`, `game_server_packets.rs`, `game_client_packets.rs`), and server-side send paths in `rose-offline-server` (`command_system.rs`, `game_server.rs`) to measure actual traffic patterns

**Architecture docs:** There is **NO networking architecture doc** in `system-architecture/` (25 files present; topics are rendering/lighting/audio/UI/physics/animation/input etc.). The index `system-architecture/README.md` confirms none covers networking. **This is a documentation gap — a `network-architecture.md` doc is recommended.** Relevant prior knowledge: `pitfalls/networking.md` (network thread exit-loop freeze, fixed) and `pitfalls/combat-sync.md` (server-authoritative damage ordering).

**Bottom line:** The subsystem is *structurally sound* — the network thread is event-driven (`tokio::select!`), there are no busy-wait loops (channels are polled once per frame with `try_recv`), entity lookup is O(1) array indexing (not O(n) scans), and outbound movement is event-driven (not per-tick spam). The real issues are: **unbounded channels + full-drain-per-frame processing causing burst frame spikes**, **per-message allocation chains** in the game thread, **fatal error handling that kills the connection on a single malformed packet**, **dead ping feature**, and **stale connections never closed on state transitions**.

---

## 2. Methodology

1. **Pre-work:** Read `pitfalls/index.md`, `pitfalls/networking.md`, `pitfalls/combat-sync.md`. Globbed `system-architecture/` — confirmed no networking doc.
2. **Static read of all client network code** (protocol clients, connection resources, connection systems, network thread, ping systems, event definitions, lib.rs registration).
3. **Verified traffic model from the server side** (`rose-offline-server/src/game/systems/command_system.rs`, `game_server.rs`): confirmed `MoveEntity`/`StopMoveEntity` are sent once per command transition (server-authoritative command echo), *not* per-tick. This bounds the expected steady-state message rate and keeps several theoretical hot-spots (e.g., per-tick move decode) off the critical path.
4. **Verified encode/decode costs in the network crates** (`rose-network-common::connection`, `packet`; `rose-network-irose::packet_codec`) — counted allocations per packet send/receive.
5. **Traced the ping feature end-to-end** — found it is dead code (an event that is never written, a timestamp never read).
6. **No code was modified, no build/run performed** (research-only deliverable).
7. All impact estimates are relative: the client runs against a LAN/local server (`rose-offline-server`) with low message rates (tens–low hundreds of packets/sec), so most costs are latency/memory-safety concerns rather than throughput bottlenecks. Priority is weighted for correctness and burst behavior over micro-optimization.

---

## 3. Findings

### F1. Unbounded channels everywhere — no backpressure, memory grows without bound
**Files:** `src/systems/network_thread_system.rs:30-32` (crossbeam unbounded + tokio unbounded per connection), `src/resources/network_thread.rs:12,22` (tokio unbounded control channel), `src/protocol/irose/*.rs` (all three clients use `UnboundedReceiver`/crossbeam `Sender`).

**Description:** Every hop in the pipeline is an unbounded channel:
1. UI/game systems → `tokio::sync::mpsc::UnboundedSender<ClientMessage>`
2. Network thread → `crossbeam_channel::unbounded::<ServerMessage>()`
3. Game thread → `NetworkThread` control channel (`UnboundedSender<NetworkThreadMessage>`)

**Why it matters:** If the game thread stalls (a long frame — zone load, physics spike) or the server bursts (zone join spawn flood, inventory dump), both directions queue unboundedly. Memory grows without limit; when the stall ends, the game thread then ingests the *entire backlog in one frame* (see F2), compounding the spike. Each message also carries an allocation (crossbeam node / tokio mpsc node) plus the `ServerMessage`/`ClientMessage` payload.

**Impact:** Medium (memory + latency under bursts; negligible at steady state for a LAN server).

**Suggested fix:**
```rust
// Bounded channel with a sane cap; on overflow drop-oldest for position-ish
// messages, block-for-connect-style messages otherwise.
let (server_message_tx, server_message_rx) =
    crossbeam_channel::bounded::<ServerMessage>(1024);
let (client_message_tx, client_message_rx) =
    tokio::sync::mpsc::channel::<ClientMessage>(1024);
```
Caution: `send().ok()` is used everywhere — with bounded channels, sends can fail; use `try_send` + explicit overflow policy per message class (drop-oldest for `MoveEntity`-type sync, keep for state).

---

### F2. Full drain of the server channel every frame → burst frame spikes
**File:** `src/systems/game_connection_system.rs:329-330` (and identically `login_connection_system.rs:30-31`, `world_connection_system.rs:34-35`)

```rust
let result: Result<(), anyhow::Error> = loop {
    match game_connection.server_message_rx.try_recv() {
        ...
        Err(crossbeam_channel::TryRecvError::Empty) => break Ok(()),
    }
};
```

**Description:** All pending server messages are processed in a single system run. When the channel is empty the loop exits immediately (one cheap `try_recv` per frame — this is *not* a busy-wait, and the cost of the empty poll is negligible).

**Why it matters:** The server is not rate-limited — on zone join it bursts `JoinZone` + `SpawnEntityMonster × N` + `SpawnEntityNpc × N` + `SpawnEntityCharacter` + `CharacterInventory` + `QuestData`, possibly hundreds of packets in one frame. Each spawn handler does: `game_data.ability_value_calculator.calculate_npc(...)` (F10-ish work), terrain-height lookup (`get_spawn_height_from_world` / `get_npc_spawn_height_from_world`, which scans **all** static zone NPCs per spawn — `game_connection_system.rs:230-240`), a `commands.queue(move |world| ...)` closure allocation, then a batched deferred spawn. Hundreds of these in one frame = visible hitch on zone entry.

**Impact:** Medium-High (hitch on zone join; worse with big zones / high spawn counts).

**Suggested fix:** Cap per-frame processing and let the remainder land on following frames:
```rust
let mut processed = 0u32;
loop {
    if processed >= MAX_MESSAGES_PER_FRAME { break Ok(()); }
    match rx.try_recv() { Ok(m) => { handle(m); processed += 1; } ... }
}
```
For spawn floods specifically, chunking entity spawns across frames (e.g., 16/frame) smooths the hitch. Alternatively, move spawn work into its own system with `Query`/`Commands` access instead of `world` closures (see F4).

---

### F3. Per-message allocation chain (3–5 heap allocations per server message)
**Files:** `src/systems/game_connection_system.rs` (handlers), `src/protocol/irose/game_client.rs:95-105` (decode + send), `rose-network-common/src/packet.rs:188-193` (PacketWriter 1 KB prealloc)

**Description:** Every server message costs, in order:
1. Network thread: `PacketReader` decode → struct; string fields via `WINDOWS_1252.decode()` allocate per string (`packet.rs:176-179` — chat name/text, clan names, etc.).
2. Network thread: `crossbeam_channel::unbounded.send` — one node allocation.
3. Game thread: `try_recv` → match arm → **`commands.queue(move |world| ...)` — one `Box<dyn Command>` allocation per message** (e.g., every `MoveEntity` at `game_connection_system.rs:787`, `UpdateStatusEffects` at :1420, `DamageEntity` at :921).
4. Application: deferred closure runs `world.entity_mut(...)` + component write.

Additionally, several handlers clone big payloads before the move into the closure (`SpawnEntityCharacter` clones `character_info`, `equipment`, `personal_store_info`, `clan_membership` — `game_connection_system.rs:520-528`).

**Why it matters:** At the measured traffic model (event-driven moves, no per-tick echoes) this is ~3–5 allocs per message at tens of messages/sec — **not a bottleneck today**. It becomes one if the server ever sends tick-rate position updates (typical of many MMOs). The `commands.queue` per trivial write is the most expensive pattern per message and is the easiest to remove.

**Impact:** Low-Medium (steady state); Medium-High (headroom for future tick-rate traffic).

**Suggested fix:** For pure component-write handlers (`NextCommand`, `Position`, `HealthPoints`, `MoveSpeed`, …), skip `commands.queue` closures: either mutate through `EntityCommands` directly where possible, or split the system into per-message systems with `Query<Entity, (&ClientEntityId, ...)>` access. A pragmatic intermediate: a small pool/reusable struct for the most frequent messages (`MoveEntity`, `StopMoveEntity`, `UpdateStatusEffects`).

---

### F4. `commands.queue` closures used for *every* trivial write — deferred work + closure alloc per message
**File:** `src/systems/game_connection_system.rs` (dozens of sites; representative: `MoveEntity` :787-813, `UpdateItemLife` :1205-1245, `UpdateSpeed` :1403-1410, `MoveToggle` :2674-2687)

**Description:** Even a bare `entity.insert(NextCommand::with_stop())` is routed through a closure so the handler can be called from inside the `try_recv` loop (the system has no `World`/`EntityMut` access — only `Commands`).

**Why it matters:** Each closure is a heap allocation; all closures apply in one deferred batch at the end of the system (Bevy `Commands` flush), so per-message cost = alloc + later archetype write. For the ~30 handler arms that only touch one entity with data already in the message, this could be done with `commands.entity(e).insert(...)` (no closure) — several already are (e.g., :879, :894, :1362). The remaining closure sites are the ones doing `world.get_entity_mut` + `get_mut::<T>()` reads, which genuinely need deferred access.

**Impact:** Low-Medium. Closures themselves are cheap; the batch-deferred application is a design feature, not a bug.

**Suggested fix:** Convert closure-free-able arms to `commands.entity(...)`; for arms that need reads, batch multiple queued messages for the *same entity* into a single closure (coalescing — see F5).

---

### F5. No coalescing of same-entity updates arriving in the same frame
**File:** `src/systems/game_connection_system.rs` (`MoveEntity` :765-822, `UpdateStatusEffects` :1413-1481, `UpdateHealthPoints` :1279-1289, `DamageEntity` :906-955)

**Description:** If the server sends 5 messages for the same entity in one frame (e.g., `MoveEntity` + `UpdateHealthPoints` + `UpdateStatusEffects` — all plausible after a fight), each is independently matched, looked up (`client_entity_list.get` — O(1), good), and queued as its own closure. There is no per-frame dirty-map, so the same entity's components are read/written 5×.

**Why it matters:** Correctness-wise this is fine (last-write-wins is the server's ordering). Performance-wise it multiplies closure allocs and component writes. Only matters at high message rates.

**Impact:** Low (current rates). Worth an explicit design note if tick-rate sync is ever introduced.

**Suggested fix:** A frame-scoped `HashMap<Entity, SmallVec<ServerMessage>>` (or per-message-class coalescing for `MoveEntity`-like types: keep only the latest position per entity per frame). Do *not* do this blindly — it must preserve ordering between *different* message classes.

---

### F6. Any single malformed packet kills the whole connection (fatal decode errors)
**Files:** `src/protocol/mod.rs:37-39` (`read_packet` error → `run_connection` returns Err), `rose-network-common/src/connection.rs:66-85` (`DecryptHeaderFailed`/`DecryptBodyFailed` are fatal), `src/systems/network_thread_system.rs:44-58` (`handle_connection_lost` → removes resource + modal dialog)

**Description:** `decrypt_packet_body` validates a CRC-style checksum (`packet_codec.rs:491-517`); any mismatch returns `false` → `read_packet` returns `Err(DecryptBodyFailed)` → the `implement_protocol_client!` loop (`protocol/mod.rs:24-50`) returns the error → the task dies, the channel disconnects, the connection system tears down the resource and shows "Connection to game server lost" — all because of one corrupted/oversized/malformed packet (e.g., a single flipped byte on a lossy link, or a server quirk sending a packet larger than `read_length`).

Also note `protocol/mod.rs:32-33`: when a `handle_packet` decode fails, the client logs `Error {} handling packet [{:03X}] {:02x?}` and **terminates the connection** rather than skipping the packet.

**Why it matters:** Robustness/availability: one bad packet = forced disconnect + re-login. For the protocol's own framing there is enough redundancy (size + checksum) to skip-and-resync safely; killing the connection is a policy choice, not a requirement.

**Impact:** Low (perf) / Medium-High (robustness — this is the most likely real-world "random disconnect" source with the offline server or flaky links).

**Suggested fix:**
```rust
// In the select! loop: distinguish IO errors from protocol errors.
packet = connection.read_packet() => match packet {
    Ok(packet) => { /* handle */ },
    Err(e) if e.downcast_ref::<ConnectionError>() == Some(&ConnectionError::DecryptBodyFailed)
        || e.downcast_ref::<ConnectionError>() == Some(&ConnectionError::DecryptHeaderFailed) => {
        log::warn!("Skipping malformed packet: {e}");
        // buffer already advanced past the bad frame in read_packet; continue loop
    }
    Err(e) => return Err(e),   // IO-level errors remain fatal
}
```
Mirror for `handle_packet` decode errors: log + `continue` instead of `return Err(error)`.

---

### F7. Ping feature is dead code — `/ping` never measures anything
**Files:** `src/ui/ui_chatbox_system.rs:425-439` (writes `ping_state.pending_ping_timestamp`, sends `ClientMessage::Chat{"/ping"}`), `src/systems/ping_command_system.rs:18-47` (reads `PingRequestEvent`/`PingResponseEvent`), `src/events/ping_event.rs` (definitions)

**Description:** Traced end-to-end:
- `PingRequestEvent` is **never written** anywhere (only the `MessageWriter<PingRequestEvent>` param exists at `ui_chatbox_system.rs:101`; the `/ping` branch doesn't use it).
- `PingResponseEvent` is **never written** anywhere (single reference is the reader in `ping_command_system.rs:36`).
- `pending_ping_timestamp` is written at `ui_chatbox_system.rs:428` and `ping_command_system.rs:25` but **never read**.
- `last_ping_ms` is only written by `ping_response_system` (never triggered) — the admin menu's "Ping: -- ms" display (`ui_admin_menu_system.rs:114`) never shows a value.
- Net effect of `/ping`: a chat message `"/ping"` is sent to the server, which broadcasts it back as `LocalChat` from your own entity — i.e., "/ping" just appears in your chatbox. No RTT is computed.

**Why it matters:** The feature is broken (silently), and every invocation wastes a round-trip chat message (plus the server chat-broadcast fan-out). Not a CPU problem; a correctness/completeness problem. The `ping_command_system` + `PingRequestEvent`/`PingResponseEvent` machinery is fully dead weight (two registered systems + two message types run every frame doing nothing).

**Impact:** Low. Worth fixing (measure RTT from the echo time of your own `LocalChat`, which already flows through `game_connection_system.rs:1054-1073`) or deleting the dead systems.

**Suggested fix (minimal):** in the `LocalChat` handler, if `entity_id == player_entity_id` and `text == "/ping"`, compute `ping_state.last_ping_ms = now - pending_ping_timestamp` and write the response — reusing the existing chat path. Then delete `PingRequestEvent`/`PingResponseEvent`/`ping_command_system`.

---

### F8. Stale login/world TCP connections never closed on state transition
**Files:** `src/systems/login_connection_system.rs:91-108` (on `JoinServerSuccess` writes `ConnectWorld` but **does not remove `LoginConnection`**), `src/systems/world_connection_system.rs:56-69` (same for `ConnectGame`/`WorldConnection`), teardown only in `handle_connection_lost` (`network_thread_system.rs:57`)

**Description:** The only thing that terminates a protocol task is channel closure (sender dropped) or a socket error. When the flow advances login → world → game, the old `LoginConnection`/`WorldConnection` resources (and their `client_message_tx`) stay alive for the entire session:
- the login socket stays open to the login server for the whole game session;
- the world socket stays open to the world server after entering the game;
- each holds a task on the network thread's single-threaded runtime (cooperative scheduling; idle tasks are cheap but accumulate).

**Why it matters:** Long-lived sockets that are never used; a rogue server could push packets to them (e.g., the login client would forward `NetworkStatus` replies into the login UI pipeline at any time — `login_client.rs:50-61`). On repeat logins, old tasks also linger until the server closes them.

**Impact:** Low (CPU/memory), Medium (hygiene + protocol robustness).

**Suggested fix:** Drop the previous-stage resource when transitioning:
```rust
// in login_connection_system on JoinServerSuccess:
commands.remove_resource::<LoginConnection>();   // drops tx → task ends
```
Same for `WorldConnection` on `SelectCharacterSuccess`. Verify no in-flight messages are needed after the transition (the `NetworkEvent::ConnectWorld` write happens first; ordering via `MessageWriter` is preserved).

---

### F9. `ClientEntityList.clear()` on reconnect without despawning ECS entities
**File:** `src/systems/game_connection_system.rs:331-333`

```rust
Ok(ServerMessage::ConnectionRequestSuccess { .. }) => {
    client_entity_list.clear();
}
```

**Description:** On a *new* game connection (re-login / server restart), the entity list is wiped but every previously spawned ECS entity (old player, all monsters, NPCs, item drops) **remains in the world** with stale components. The subsequent `CharacterData` handler spawns a *second* player entity (`game_connection_system.rs:356-374`). Duplicate players, ghost monsters, and stale `ClientEntity`/`ClientEntityId` components accumulate.

**Why it matters:** Not a throughput issue, but it's the single biggest correctness hazard in the network layer: after any reconnect there are two player entities, and many systems query `With<PlayerCharacter>`/`ClientEntityId` without deduplication (double movement/health/UI state, `selected_target` pointing at a ghost, etc.).

**Impact:** Medium-High (correctness on the only reconnection path that exists).

**Suggested fix:** Despawn everything before clearing:
```rust
for entity in client_entity_list.client_entities.iter().flatten() {
    commands.entity(*entity).despawn();
}
if let Some(player) = client_entity_list.player_entity { commands.entity(player).despawn(); }
client_entity_list.clear();
```
(A `iter().flatten()` over `Vec<Option<Entity>>` is O(1) per entry, no allocation.)

---

### F10. `ClientEntityList` — 512 KB dense vec, `fill(None)` clears
**File:** `src/resources/client_entity_list.rs:14-23,34-36`

**Description:** `vec![None; u16::MAX as usize]` = 65 536 × 8 B ≈ 512 KB, allocated once, retained for the session. `clear()` = `fill(None)` over the whole vec (~512 KB memset), called on `ConnectionRequestSuccess`, `JoinZone`, `Teleport` (`game_connection_system.rs:333,489,1039`).

**Why it matters:** Trivial. Direct indexing (`get`, :38-40) is O(1) — **no O(n) scans anywhere** (this was a specific concern in the task brief; verified clean). The memset cost is sub-0.1 ms. The only meaningful remark: with F9's despawn fix the clear becomes rare anyway. Optionally shrink via a `Vec` with upper-bound check, but not worth the effort — keep as-is.

**Impact:** Negligible (documented as verified-clean).

---

### F11. Per-outbound-packet allocation chain: `PacketWriter::new(1024)` + `Packet::from` + second `BytesMut` + memcpy + per-packet `flush()`
**Files:** `src/protocol/irose/game_client.rs:1058-1061` (and login/world clients), `rose-network-irose/src/game_client_packets.rs:172-181` (`PacketWriter::new(ClientPackets::Move)`), `rose-network-common/src/packet.rs:188-193` (`with_capacity(1024)` — a full 1 KB allocation even for a 10-byte connect packet), `rose-network-common/src/connection.rs:89-110` (`write_packet` allocates a *new* `BytesMut::with_capacity(size)`, copies the data in, encrypts O(n) with table XOR, then `flush()` — a syscall per packet)

**Description:** Each send costs: 1 KB `BytesMut` (PacketWriter) → `Packet` (freeze) → new `BytesMut` sized to the packet + memcpy → encrypt → `write_all` → `flush`. For small packets (most of them: `ConnectRequest`, `Move`, `Attack`, `MoveToggle`, `Chat` ≈ 6–40 bytes) that's ~2 allocs + 1 memcpy + 1 syscall per packet, with 1 KB of capacity allocated per packet.

**Why it matters:** Outbound rates are low (command-driven, verified in F-intro). Not a bottleneck today. But it's the cheapest win in the crate layer and matters if the client ever sends periodic updates (flight `MoveCollision` at `flight_movement_system.rs:125-175` can fire every frame while flying — worth re-checking its rate).

**Impact:** Low-Medium (per-send cost ~1 µs; syscall latency more than CPU).

**Suggested fix (in `rose-network-common`, client + server compatible):**
- Reuse a scratch `BytesMut` in `Connection` for the framing+encrypt step (avoid the second alloc + copy: encode straight into the final buffer).
- `PacketWriter::new` with capacity `min(1024, 64)` won't help much; instead add `Connection::write_packet_ref(&Packet)` that borrows `packet.data` without re-freezing.
- Drop the per-packet `flush()` for bulk sends, or at least keep it (chat latency matters) — measure first.
- The iRose wire format must not change (see Risks) — this is purely an allocation/latency refactor inside the client-side `Connection`; byte-for-byte identical output.

---

### F12. Decode cost is already lean — no action needed (documented)
**Files:** `rose-network-common/src/packet.rs:48-180` (`PacketReader` — zero-copy `&[u8]` cursor, LE reads), `connection.rs:42-87` (`read_packet` reuses one `BytesMut` buffer, `split_to(...).into()` shares the buffer — no per-packet buffer alloc), `packet_codec.rs:455-522` (in-place header/body decryption, single pass, table-driven)

**Description:** Receive path is genuinely efficient: one shared read buffer (4 KB initial), in-place decrypt, zero-copy `Bytes` slice-out, bounds-checked reads. Per-packet cost ≈ O(n) table XOR + struct decode. The only per-packet heap allocation on the wire is the `Bytes` handle itself. String fields allocate once per string via `WINDOWS_1252.decode()` (`packet.rs:176-179`) — unavoidable at the message layer since `ServerMessage` owns `String`s.

**Why it matters:** Verified clean; the decode side needs no optimization. Any future work should target the *game thread* (F2–F5), not the codec.

**Impact:** None (verified-clean).

---

### F13. Hot-path diagnostic logging (`[ATTACK_DIAG]`) is enabled by default at `info` level
**Files:** `src/systems/game_connection_system.rs:776-783` (every player `MoveEntity`), :828-834 (every player `AdjustPosition`), :876-878 (every player `StopMoveEntity`), and similar `[RESPAWN_MOVE_DIAG]`/`[NPC SPAWN]`/`[TELEPORT]`/`[PLAYER SPAWN]` infos; default level `"info"` in `src/logging/mod.rs:104-112`

**Description:** Default logging level is `info` with **two** layers: console + `structured.jsonl` (JSON-serialized). The `[ATTACK_DIAG]` family fires on *every* player movement echo from the server; `log::info!` arguments are only formatted when the level is enabled — and `info` is enabled. Each such line = formatted string + JSON serialization + queue write + console write, per player-move message, for the whole session (the JSONL file grows continuously; these are diagnostic messages that served their purpose).

**Why it matters:** Low-Medium. Steady-state CPU cost is small (a few moves/sec), but the log files bloat and any burst multiplies it. It's dead weight that can be removed entirely or demoted to `trace!` behind a debug flag.

**Impact:** Low-Medium.

**Suggested fix:** Delete the `[ATTACK_DIAG]` blocks or gate them with `#[cfg(feature = "network-debug")]` / a runtime flag. Keep `log::warn!` for the genuinely actionable cases (entity-not-found at :815-820 is already the right severity).

---

### F14. Network thread: single-threaded runtime hosting all protocol tasks — cooperative but fine
**File:** `src/resources/network_thread.rs:21-41`

**Description:** One OS thread runs a current-thread tokio runtime; each connection is a `tokio::spawn`ed task. The control loop is event-driven (`control_rx.recv().await`), and each client's `run_connection` is a `tokio::select!` between `read_packet()` and `client_message_rx.recv()` — **no polling, no busy-wait** (verified). All three protocol clients share the thread; decode work is O(packet size) so starvation is not realistic.

**Why it matters:** Verified clean. One caveat: `client.run_connection().await.ok()` (`network_thread.rs:33`) swallows the error — the game thread only learns about failure via channel disconnect (`TryRecvError::Disconnected`), so the actual error detail never reaches the UI ("Connection lost" with no cause). Not perf; minor observability note.

**Impact:** None (perf). Low (observability).

---

### F15. Event routing: connect flow has a one-frame latency hop; otherwise events are change-driven
**Files:** `src/lib.rs:1651-1658` (connection systems in `PreUpdate`), `src/lib.rs:1682` (`network_thread_system` in `PostUpdate`), `src/events/mod.rs`

**Description:** `NetworkEvent`s written by login/world systems (PreUpdate) are consumed by `network_thread_system` in **PostUpdate** — the new connection resource appears one frame late. All other events in the flow (`GameConnectionEvent::Connected`, `LoadZoneEvent`, `ChatboxEvent`, etc.) are written only on change, and readers are per-system (each system has its own message buffer — no contention, no cross-system locks).

**Why it matters:** One-frame latency in connection setup is imperceptible. The event layer is clean; `ChatboxEvent` is written both directly (`chatbox_events.write`) and via `world.resource_mut::<Messages<ChatboxEvent>>().write(...)` inside closures (e.g., `game_connection_system.rs:946-949,1060-1070,2245-2248`) — both are valid Bevy 0.18 message writes, no double-delivery (single global message queue per type). No issue found.

**Impact:** None (verified-clean).

---

### F16. Per-frame cost when *no* connection exists is ~zero (verified)
**Files:** `src/systems/game_connection_system.rs:325-327` (`Option<Res<GameConnection>>` early-out), `login_connection_system.rs:25-27`, `world_connection_system.rs:22-32`, registration `src/lib.rs:1651-1658` (gated by `resource_exists::<CurrentZone>`)

**Description:** All three connection systems early-return when their resource is absent; `game_connection_system` additionally gates on `CurrentZone` existing. Empty-channel polling is one `try_recv` per frame (~tens of ns). No busy-wait, no per-frame allocations on the idle path.

**Why it matters:** Verified clean — the "polling vs event-driven" concern from the brief is resolved in favor of cheap polling; converting to event-driven would add complexity for ~0 gain at this frequency.

**Impact:** None.

---

### F17. Outbound message construction: chat path double-copies text
**Files:** `src/ui/ui_chatbox_system.rs:466-474` (`text.clone()` of the input box), `src/protocol/irose/game_client.rs:1096-1098` → `PacketClientChat` borrows → `Packet::from` copies into `BytesMut` (`game_client_packets.rs:215-221`)

**Description:** Sending a chat line copies the string twice (UI clone + packet copy), plus the F11 send-chain allocations. Same pattern for `LoginRequest` (username/password strings at `login_connection_system.rs:36-41` → `to_md5()` allocation).

**Why it matters:** Human-typed chat is rare; `to_md5()` runs once per login. Negligible. Grouped with F11 as part of the outbound allocation story.

**Impact:** Negligible.

---

### F18. `ServerMessage` channel payloads are large but boxed-appropriately (verified)
**Files:** `rose-game-common/src/messages/server.rs:306+` (enum with ~80 variants), `src/protocol/irose/game_client.rs:127-145` (`CharacterData` wrapped in `Box`)

**Description:** Large payloads (`CharacterData`, `CharacterDataItems`, `SpawnEntityCharacter`) are already boxed; small variants (≈100-200 B each) move by value. Crossbeam node allocation dominates message size. No oversized stack movement of big payloads was found; the enum's own size is fine for channel transfer.

**Why it matters:** Verified-clean; no change needed.

**Impact:** None.

---

### F19. Zone-join spawn path does redundant per-spawn work (terrain/NPC scans + ability calc)
**File:** `src/systems/game_connection_system.rs:86-120` (`spawn_client_entity` → `get_npc_spawn_height_from_world`), :220-266 (per-spawn O(zone NPCs) scan for IFO height), :597-600, :651-654 (`calculate_npc` per spawn)

**Description:** Each spawn does: (a) a scan of **all** static zone NPCs to find a matching IFO position (`get_npc_spawn_height_from_world` — only needed for `SpawnEntityNpc`; `spawn_client_entity` calls it for NPC spawns and the generic path for everything else), and (b) a full `calculate_npc` ability build. During the F2 burst, this multiplies the per-spawn cost: N spawns × (O(zone_npcs) scan + ability calc + closure alloc).

**Why it matters:** Medium when combined with F2's full-drain burst (zone entry hitch). The NPC scan is the most suspicious: it's O(zone NPCs) per NPC spawn and runs even when the zone has no matching static NPC (which is the common case for dynamically spawned monsters — but note `spawn_client_entity` calls `get_npc_spawn_height_from_world` **only when `npc_id` is Some**, so monsters use the cheap terrain path).

**Impact:** Medium (burst-time only).

**Suggested fix:** Precompute a per-zone `HashMap<NpcId, (f32,f32,f32)>` of IFO spawn points at zone load (in `zone_loader`), then `get_npc_spawn_height_from_world` is a single lookup instead of a scan. Cache `calculate_npc` results per `(npc_id, status_effects)` in a frame-scoped map if repeated spawns of identical mobs occur in one burst.

---

## 4. Priority-Ranked Summary

| # | Finding | Impact | Effort | Priority |
|---|---------|--------|--------|----------|
| F6 | Fatal on single malformed packet → forced disconnect | High (robustness) | Low | **High** |
| F9 | Reconnect: `clear()` without despawn → duplicate player/ghost entities | High (correctness) | Low | **High** |
| F2 | Full-drain per frame → zone-join burst hitch | Medium-High (perf) | Low | **High** |
| F1 | Unbounded channels, no backpressure | Medium (memory/latency) | Low-Med | Medium |
| F7 | Ping feature is dead code; `/ping` sends junk chat | Low-Med (correctness) | Low | Medium |
| F8 | Stale login/world sockets never closed on transition | Low-Med (hygiene) | Low | Medium |
| F3/F4 | Per-message closure/alloc chain in handlers | Low-Med | Med | Medium |
| F19 | Per-spawn O(zone NPCs) scan + ability calc on zone join | Medium (burst) | Med | Medium |
| F13 | `[ATTACK_DIAG]` info logs on hot paths, default on | Low-Med | Low | Medium |
| F11 | Outbound: 1 KB alloc + memcpy + syscall per packet | Low-Med | Med | Low |
| F5 | No same-frame coalescing of same-entity updates | Low | High | Low |
| F10/F12/F15/F16/F18 | Verified-clean areas (no change) | — | — | — |

---

## 5. Quick Wins

1. **Skip-and-continue on malformed packets (F6)** — ~15-line change in `protocol/mod.rs` + `rose-network-common/connection.rs`; eliminates the "random disconnect on flaky link" class entirely.
2. **Despawn entities before `client_entity_list.clear()` on reconnect (F9)** — ~6 lines in `game_connection_system.rs:331-333`; fixes duplicate-player/ghost-monster bugs on the only reconnect path.
3. **Cap messages processed per frame (F2)** — ~4 lines (`if processed >= N { break }`) in the three connection systems; removes zone-join hitch headroom.
4. **Remove/demote `[ATTACK_DIAG]` logging (F13)** — delete 3 blocks; shrinks `structured.jsonl` growth and removes per-message formatting.
5. **Drop stale `LoginConnection`/`WorldConnection` resources on transition (F8)** — one `commands.remove_resource` per transition; closes unused sockets.
6. **Wire `/ping` to the existing `LocalChat` echo path (F7)** — ~10 lines; makes the feature real and removes dead systems/events.

---

## 6. Risks & Considerations

1. **Protocol compatibility (iRose wire format) is the hard constraint.** The offline server (`rose-offline-server`) and client share `rose-network-irose`. Any change to `rose-network-common` (F11) must be **byte-for-byte output identical** (header bitfields `HeadCryptedServer/Client`, seed tables, checksum CRC over the body, XOR table walk — `packet_codec.rs:402-522`). Refactors are safe only if they touch allocation/latency, never framing. Server-side code uses the same `Connection` (`rose-offline-server/src/irose/protocol/game_server.rs:570-651`), so `Connection` changes must stay symmetric.
2. **Do NOT convert decode to "skip" blindly (F6):** `read_packet` must still advance the buffer past the bad frame — the current code does (`split_to`/`advance` on decrypt success; on checksum failure the framing is still known from `add_buffer_len`), but verify resync behavior with a fuzz harness before shipping. The client must never desync the shared `BytesMut` buffer.
3. **Bounded channels (F1) change failure semantics:** every `send().ok()` site swallows send failures today. With bounded channels, define per-message-class overflow policy (drop-oldest for position/sync messages; block or retain for authoritative state like `UpdateInventory`). Silent drops of state-changing messages would desync inventory/quest state.
4. **Message ordering (F5 coalescing) must preserve server order** between different message classes (e.g., `DamageEntity` before `UpdateStatusEffects`). Coalesce only within a class.
5. **F8 teardown timing:** removing `LoginConnection` drops the sender *inside* the same system that wrote `ConnectWorld`; ensure the `NetworkEvent` (PostUpdate consumer) is delivered before the sender drop terminates the task — event write happens first in the same system run, and the event buffer survives the frame, so this is safe; keep `ConnectWorld` → `WorldConnection::new` → synchronous `ConnectionRequest` ordering (`world_connection.rs:13-32`) intact.
6. **F9 despawn timing:** `commands.entity(...).despawn()` inside the `ConnectionRequestSuccess` arm is applied deferred — the old player entity still exists until flush, which happens before the next frame's systems, so `With<PlayerCharacter>` queries see at most one player per frame afterward. Order the despawn before the `CharacterData`-driven spawn in the same frame's command buffer.
7. **`client.run_connection().await.ok()` swallows the real error** (`network_thread.rs:33`); if F6's skip logic is added, consider propagating the error text through a side channel so `handle_connection_lost` shows the cause instead of the generic message.
8. **Server is the traffic source; no client-side pacing exists or is needed today** (movement is command-echo, verified in `rose-offline-server` `command_system.rs:482-531`). If the server ever moves to tick-rate position broadcasts, F2/F3/F5 become the critical items — the report flags them accordingly.
9. **Logging level is `info` by default** (`logging/mod.rs:104-112`) — any future hot-path `log::info!` should be `trace!`-gated; the `[ATTACK_DIAG]` blocks are the precedent of why.

---

## 7. Verification Update (2026-08-04)

Independent sub-agent scrutiny of every finding (F1–F19) against the actual source, Bevy 0.18.1, the network crates (`rose-network-common`/`rose-network-irose` in the rose-offline workspace), the server send paths, and the previous implementation attempt on `wip/local-changes-2026-08-04` (validated in `12-validation.md`). Verdicts per finding:

| Finding | Verdict | Scrutiny result / action |
|---|---|---|
| F1 | CONFIRM (claim) / fix NOT safe as written — DEFER | All three hops verified unbounded. **The fix sketch is unsafe**: `crossbeam_channel::bounded` + plain `send` **blocks** — all senders live on the network thread's single-threaded runtime, so a full channel = head-of-line block of the entire network subsystem (the opposite of backpressure); the tokio hop needs async `send` (the ~60 game-thread sites use sync sends); drop-oldest is safe ONLY for an ephemeral allow-list (MoveEntity/StopMoveEntity/AdjustPosition/UpdateStatusEffects/UpdateHealthPoints/UpdateItemLife/MoveSpeed/NextCommand) — anything else dropped = permanent desync. Overflow is unlikely in practice (TCP backpressure caps the backlog; F2's per-frame cap already fixes the compounding half). If ever done: bound hop 2 only (crossbeam `bounded(4096)` matching F2's drain cap) with `try_send` + drop-oldest allow-list + warn-log; leave hops 1 and 3 unbounded. |
| F2 | PARTIAL (confirms GC-03) | Uncapped drain + server burst confirmed from the server source (`client_entity_visibility_system` sends one message per visible entity per run). The wip cap mechanism is correct (counter-before-recv, strict FIFO, no drops) — **but 4096 is toothless for the documented "hundreds" burst** (the wip comment itself admits the burst drains in one frame). Lower the game cap to **256–512** (16 would spread spawn-in across ~20 frames); keep 64/64 login/world (harmless no-ops). The structural alternative (per-message systems with Query access, F4) remains the real solution. |
| F3 | PARTIAL (core largely refuted; fold into GC-02) | `WINDOWS_1252.decode()` is **dead code** (zero call sites — all strings use `read_null_terminated_utf8`, a zero-alloc borrowed `&str`); `commands.queue` is NOT a `Box<dyn Command>` alloc in Bevy 0.18 (dense `Vec<MaybeUninit<u8>>` byte buffer — no per-command heap alloc, amortized). Real per-message cost: MoveEntity ≈ 1 alloc (crossbeam node); chat ≈ 3; SpawnEntityCharacter ≈ 6–8 but rare. **Genuine F3-only residue**: the `:520-528`/`:539` SpawnEntityCharacter clones (character_info/equipment/personal_store_info/clan_membership/name) — eliminate via borrow-then-move. Drop the "pool" suggestion (premise refuted). Also correct F12's note (repeats the win1252 error). |
| F4 | CONFIRM (fold into GC-02; one citation error) | 67 closure sites vs 23 direct sites verified. **`MoveToggle :2674-2687` is NOT a closure site** (already fully direct — the doc's census needs an arm-by-arm audit before any conversion). Converting unconditional-insert arms to `commands.entity(e).insert(...)` is behaviorally identical (same queue, same flush, FIFO preserved; skip-vs-skip on missing entities). Blocked on the **16-param limit** (verified: exactly 16 params, zero queries — a Query param won't compile; bundle the 9 MessageWriters into a SystemParam first). Read-requiring arms (MoveEntity, UpdateItemLife, UpdateSpeed) stay closures for despawn safety. Fold into GC-02; not implemented anywhere. |
| F5 | CONFIRM (design-note-only — defer) | No coalescing exists; **traffic reality kills the payoff**: UpdateStatusEffects is sent at most once per entity per tick (duplicates impossible); same-entity ≥2-messages-in-one-frame ≈ 0.1–0.3% of frames ≈ a few per minute, saving ≪1 µs each. The ordering hazard is real (the HP trio: DamageEntity additive, UpdateHealthPoints absolute, UpdateStatusEffects delta — FIFO preserves server intent; arbitrary batch order breaks it). Only UpdateHealthPoints and MoveEntity are collapsible (MoveEntity has a hidden trap: `target_entity` is resolved before queueing). Keep as a design note for future tick-rate sync; do not implement. |
| F6 | CONFIRM (fix on wip, safe — merge + commit the dependency) | Death chain verified end-to-end. Two stale details: (1) `rose-network-common/connection.rs` **already advances the buffer past the bad frame** on checksum failure — but this is an **UNCOMMITTED change in the rose-offline workspace that must be committed** (the client fix is coupled to it; if reverted, behavior degrades gracefully); (2) "flaky link" is overstated (TCP is reliable — real value is server-encode bugs and codec-seed desync on reconnect). The wip implementation (MAX_MALFORMED_PACKETS=8, only `DecryptBodyFailed` skipped, header failures stay fatal) is **strictly better than the doc's snippet**: skipping `DecryptHeaderFailed` would live-lock (read_packet does NOT advance on header failure). Decode-error skip is safe (no side effects occur before successful decode — verified across all arms). Resync correctness proven by byte accounting (length-prefixed framing; checksum covers the whole frame). Optional: a read_packet resync unit test. |
| F7 | PARTIAL (implement-as-echo, corrected) | Dead-code claim CONFIRMED (never written/read; one correction — the two systems are **not even registered** in lib.rs, so the cost is zero, not per-frame). **The echo mechanism is REFUTED as written**: the server routes any `/`-prefixed chat to `ChatCommandEvent` (never echoed; `/ping` currently yields a "Failed: /ping" whisper — not "appears in your chatbox"). Plain non-slash chat IS echoed back to the sender. Correct fix: `/ping` sends a unique non-slash token (`PING<n>`); the LocalChat arm matches own-entity + token → compute RTT, suppress the Say/bubble writes, add a timeout + single-flight guard; delete PingRequestEvent/PingResponseEvent/both systems (keep PingState + `is_ping_command` + tests). No iRose ping packet exists — protocol-based is overkill. |
| F8 | PARTIAL (REJECT as written) | World half confirmed mechanically, but **removing WorldConnection on SelectCharacterSuccess is a functional regression**: the world socket's persistence is intentional and load-bearing — it is the only in-session recovery path after a game disconnect (debug "Game Character Select" and `character_select_system` require it, else the player is stranded at login) and it carries the server's return-to-character-select channel. Login half REFUTED: `login_state_exit_system` already removes LoginConnection on `OnExit(GameLogin)` ~1 RTT after JoinServerSuccess (plus two more teardown sites); no task accumulation (insert_resource replaces). Tie any future world-socket cleanup to explicit session-end, not SelectCharacterSuccess. |
| F9 | CONFIRM (fix on wip, correct and complete — merge) | Claim confirmed (bare `clear()` — player_entity/player_entity_id not even reset; 76 systems query With<PlayerCharacter>; the "selected_target ghost" is self-healing — minor overstatement). The wip fix is complete: despawns all list entities + the player (skipping the in-loop duplicate), recursive despawn handles all children (skeleton joints, model parts, weapon trails, name tags, effects — verified), zone-content entities are correctly untouched (spawned via ZoneEvent::Loaded, not the list), ordering is sound (server sends ConnectionRequestSuccess→CharacterData back-to-back; FIFO keeps ≤1 player per frame). **§3.12's panic claim REFUTED**: despawn uses the warn error handler + entity generations. Implement as-is; optional: reset player_entity/player_entity_id + prune ProjectileIndex on reconnect. |
| F10 | CONFIRM (no action) + one doc correction | Verified (512 KB vec; clear = fill(None) ≈ 10–30 µs; Teleport is the frequent clear site). **"No O(n) scans anywhere" is FALSE**: the Teleport handler iterates all 65,536 entries every teleport (`:1027-1038`) ≈ 50–150 µs (negligible vs ms-scale zone load); F9-on-wip adds a second full-vec scan on the rare reconnect. Confirm no-change; correct the doc line. |
| F11 | PARTIAL (implement piece 1 opportunistically; defer the rest) | Claim confirmed (2 mallocs + 1 memcpy + 1 syscall per send; flight rate is 60 Hz only while thrusting/momentum, hover 10 Hz on wip — negligible). Fix 1 (scratch BytesMut) is SAFE — **must be a NEW field** (reusing `self.buffer` would corrupt in-flight received packets: read_packet returns zero-copy Bytes sharing it); byte-identical output; ~20 lines in the shared crate; the server benefits more (it sends per tick). Fix 2 (`write_packet_ref`) is near-worthless without a caller-side builder redesign. Fix 3 (drop per-packet flush) REJECTED — unbounded latency at command-driven rates; the BufWriter holds packets until the buffer fills. |
| F12 | CONFIRM (no action — verified-clean) | All claims verified; two precision notes: the `Bytes` from `split_to().into()` is a stack struct (refcount bump), not a heap alloc; `WINDOWS_1252.decode` is dead code — the real path is zero-alloc UTF-8 borrows, with `.to_string()` only where messages own Strings. Confirm no-change; fix the doc's win1252 note. |
| F13 | CONFIRM (aligns with GC-04; one divergence to fix at merge) | Sites + default level verified; keep the `:815-820` entity-not-found at **warn** — the wip demoted it to debug (must be restored during the port). The `cfg(feature)` suggestion is invalid (Cargo.toml has no `[features]` section — use demote-to-debug + RUST_LOG). One-shot [NPC SPAWN]/[PLAYER SPAWN]/[TELEPORT] infos stay (low frequency, wip-validated). |
| F14 | CONFIRM (verified-clean) | All claims accurate (one OS thread; true select! loops; no busy-wait; `.ok()` swallows the error → generic "Connection lost"). No perf change; optional observability: 1-line `log::warn!` on error now (free), full error-propagation via oneshot channel defer (sequence after the wip F6 merge — same file region). |
| F15 | CONFIRM (no action — verified-clean) | One-frame hop verified (Messages persist across frame boundaries; consumers just start polling one frame later); ChatboxEvent double-write verified safe (one global queue per type in Bevy 0.18, cursor-based readers — no double delivery); per-system readers confirmed. Cosmetic: the ConnectLogin writer is in the egui pass, not PreUpdate. Confirm no-change. |
| F16 | CONFIRM (no action — verified-clean) | Early-outs verified (world_connection is even stricter: needs both WorldConnection AND Account); try_recv is a lock-free tens-of-ns pop; zero idle-path allocations. Line-ref drift: gating is at lib.rs:1659-1666/1664. Confirm no-change. |
| F17 | CONFIRM (implement the trivial fix) | UI clone is avoidable (nothing reads the text between send and clear); the packet copy is inherent serialization; md5 once per login. Zero-risk fix: `std::mem::take(&mut textbox_text)` instead of clone+clear at ui_chatbox_system.rs:470-473. Do not touch to_md5 or the packet path. |
| F18 | CONFIRM (no action) + doc correction | Boxing verified (4 boxed variants — CharacterDataQuest was omitted); **the ~100-200 B claim is wrong for 3 variants**: `UpdateStatusEffects` carries 432 + 1440 B EnumMaps → the enum is ~1.9 KB and **every send copies ~1.9 KB** into the channel node (still node-alloc-dominated). Boxing the EnumMaps would cut transfers ~34× but isn't worth the churn (rare message, 3 server sites + all match sites). Confirm no-change; correct the doc premise. |
| F19 | CONFIRM (implement-with-changes; impact downgraded to Low) | Mechanics accurate (scan only for SpawnEntityNpc — monsters/characters/drops use the cheap path; O(M) per spawn with no early-out; calculate_npc deterministic). Realistic cost: tens–low-hundreds of µs on a zone-join burst — invisible next to model loading. Fix sound with one design correction: use `HashMap<NpcId, Vec<Vec3>>` (multiple spawn points per id are common) **stored inside `ZoneLoaderAsset`** (shares the F1 eviction lifecycle — a separate resource would go stale), preserving the exact acceptance criteria (nearest, `distance_sq < 200²`, `z != 0`, terrain window, else fallback). calculate_npc cache: optional/low-value; key must exclude `expire_times` (Instants aren't Hash). |

**Cross-cutting note for the whole doc set:** the vendored source folder `bevy-collection\bevy-0.18.1` is actually **0.19.0-dev** (its `Cargo.toml` declares `version = "0.19.0-dev"`); the client builds against crates.io Bevy 0.18.1. API citations verified only against that folder must be re-checked against 0.18.1 before implementing. Additional note: the F6 fix depends on an **uncommitted change in `rose-network-common` (rose-offline workspace)** — that must be committed alongside the client-side work.
