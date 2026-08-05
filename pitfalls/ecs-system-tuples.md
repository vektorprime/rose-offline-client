# ECS / System Registration Pitfalls

---

## Adding a System to a Full Tuple Breaks the Build With a Cryptic Error (Fixed 2026-08-04, user confirmed)

### Problem
After adding `world_ui_occlusion_system` to the large `app.add_systems(Update, (...))` tuple in `src/lib.rs`, the build failed with:

```
error[E0277]: `(..., ..., ..., ...)` does not describe a valid system configuration
    --> src\lib.rs:1116:9
```

The error pointed at the whole tuple and the note chain did **not** identify which system was at fault. An earlier build had a real error in the new system, which masked the issue (the tuple error looked like a cascade); only after the real error was fixed did it become clear the tuple itself was the problem.

### Root Cause
Bevy implements `IntoScheduleConfigs` for tuples of size **1 to 20** only (`all_tuples!(impl_node_type_collection, 1, 20, ...)` in `bevy_ecs/src/schedule/config.rs`). The tuple already contained exactly 20 systems; adding a 21st made the whole tuple fail the trait bound.

### Solution
Register the extra system in its own `add_systems` call — ordering constraints like `.after(...)` work across separate calls within the same schedule:

```rust
// Separate add_systems call: the tuple above is already at Bevy's 20-system tuple limit.
app.add_systems(
    Update,
    world_ui_occlusion_system.after(name_tag_visibility_system),
);
```

### Files Modified
- `src/lib.rs`

### Lesson Learned
- Bevy system tuples are capped at **20 elements**. When an `add_systems` tuple fails with "does not describe a valid system configuration" and the note chain names no specific system, first **count the tuple elements**.
- Big `add_systems` tuples in `lib.rs` are a shared hotspot — when multiple changes land at once, check whether someone else already filled the tuple to the limit.
