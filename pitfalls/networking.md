# Networking Pitfalls

This document records networking-related issues encountered during development.

---

## MoveEntity Not Sent After Player Respawn (Fixed 2026-03-15)

### Problem
After player death and respawn, `PlayerCommandEvent::Move` was being sent to the server but no `MoveEntity` response was received back, preventing player movement.

### Root Cause
When respawning in the **same zone**, the VFS zone loader skipped spawning the zone (to prevent memory leaks) but **did not send `ZoneEvent::Loaded`**. This broke the chain:
1. Without `ZoneEvent::Loaded`, `game_zone_change_system` never sent `JoinZoneRequest` to the server
2. Without `JoinZoneRequest`, the server never re-added `ClientEntity`, `ClientEntitySector`, `ClientEntityVisibility` components
3. The server's `server_messages_system` query requires these components to route messages to clients
4. Without these components, `MoveEntity` responses could not be sent to the player

### Solution
In `src/zone_loader.rs`, when skipping zone spawn because zone already exists, still send `ZoneEvent::Loaded`:
```rust
if already_loaded.contains(&event.zone_id.get()) {
    log::warn!("[ZONE LOADED FROM VFS] Zone {} already exists, skipping spawn", ...);
    // FIX: Still send ZoneEvent::Loaded so JoinZoneRequest is sent to server
    zone_events.write(ZoneEvent::Loaded(event.zone_id));
    continue;
}
```

### Files Modified
- `src/zone_loader.rs` - Added `ZoneEvent::Loaded` when zone already exists

### Lesson Learned
When optimizing to skip work (like skipping zone spawn when already loaded), ensure all **side effects** (like events) still occur. The non-VFS path correctly sent `ZoneEvent::Loaded` for cached zones, but the VFS path missed this.

---

## Game Freezes on Window Close Instead of Exiting (Fixed 2026-02-22)

### Problem
When the user closes the game window (clicks the X button), the game freezes instead of exiting cleanly. The process hangs indefinitely and must be force-killed.

### Root Cause
The network thread function `run_network_thread()` in [`src/resources/network_thread.rs`](src/resources/network_thread.rs) had an **extra outer `loop`** that prevented the thread from ever exiting:

```rust
// BROKEN CODE - outer loop causes infinite loop on exit
pub fn run_network_thread(mut control_rx: ...) {
    loop {  // <-- BUG: This outer loop should NOT exist!
        tokio::runtime::Builder::new_current_thread()
            .block_on(async {
                loop {  // <-- Inner loop
                    match control_rx.recv().await {
                        Some(NetworkThreadMessage::Exit) => return,  // Only exits inner block!
                        None => return,  // Only exits inner block!
                        // ...
                    }
                }
            })
        // After return from async block, outer loop continues!
    }
}
```

**What happens on exit:**
1. Window close → Bevy sends `AppExit` event
2. [`lib.rs:1425`](src/lib.rs:1425) sends `NetworkThreadMessage::Exit` to network thread
3. [`lib.rs:1426`](src/lib.rs:1426) calls `network_thread.join()` to wait for thread
4. Network thread receives `Exit` message
5. `return` only exits the inner `block_on` async block, NOT the function
6. Outer `loop` continues, creates new tokio runtime
7. Channel is closed (sender dropped), so `recv()` returns `None` immediately
8. **Infinite busy loop** - thread never exits, `join()` blocks forever = **FREEZE**

### Solution
Remove the outer `loop`. The function should only have the inner async loop:

```rust
// FIXED CODE - no outer loop, thread exits properly
pub fn run_network_thread(mut control_rx: ...) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            loop {
                match control_rx.recv().await {
                    Some(NetworkThreadMessage::RunProtocolClient(client)) => { /* ... */ }
                    Some(NetworkThreadMessage::Exit) => return,  // Now exits the function!
                    None => return,  // Now exits the function!
                }
            }
        })
}
```

### Files Modified
- `src/resources/network_thread.rs` - Removed outer `loop` from `run_network_thread()`

### Lesson Learned
When implementing threaded background tasks that should exit on a signal:
1. **Be careful with nested loops** - A `return` inside an async block only exits that block, not the outer function
2. **Test exit paths** - Always verify background threads actually terminate when sent an exit signal
3. **Avoid unnecessary nesting** - The outer `loop` was unnecessary; the tokio runtime only needs to be created once
4. **Channel closure handling** - When the sender is dropped, `recv()` returns `None`, which should also trigger exit
