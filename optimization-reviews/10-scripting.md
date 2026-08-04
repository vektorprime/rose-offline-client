# Optimization Review 10: Scripting Subsystem (Lua 4 VM, Quest Scripting)

## 1. Title + Scope Summary

**Component analyzed:** The custom Lua 4 bytecode VM (`src/scripting/lua4/`), the script/quest function bindings (`src/scripting/`), quest trigger systems (`src/systems/quest_trigger_system.rs`, `collision_system.rs` trigger path, `game_connection_system.rs` quest-message path), and quest UI (interaction only).

**Architecture docs:** `system-architecture/` contains **no** document covering scripting, quests, or the Lua VM (index covers Animation/Assets/Audio/Camera/ECS/Input/Lighting/Physics/Render/Transform/UI/Window). `pitfalls/` has **no** scripting or quest entries. This report is the first documentation of this subsystem.

**Execution model (established by reading call sites):** Scripting is **event-driven, not per-frame**. The Lua VM runs (a) once per NPC/event dialog open (`conversation_dialog_system.rs:99` — main chunk), (b) for menu condition functions when a menu is generated (`conversation_dialog_system.rs:218,405`), and (c) for action functions on click (`conversation_dialog_system.rs:662`). Quest trigger conditions run on demand from Lua (`QF_checkQuestCondition`/`QF_doQuestTrigger`), from player collision with event objects throttled to once per 5 s (`collision_system.rs:616-624`), and from server messages (`game_connection_system.rs:1632-1646`). **No Lua script and no quest condition runs every frame.** Absolute CPU cost is therefore bounded and small; the findings below are about removing avoidable allocations/duplication inside those event-driven executions, plus per-frame UI-layout costs in the dialog systems, and a handful of structural risks.

**Heat summary:** VM per-instruction heap allocations dominate scripting cost (but only run on user-paced events); per-frame costs exist only in dialog rendering and the collision trigger intersection test.

---

## 2. Methodology

1. Enumerated `system-architecture/` and `pitfalls/` — no scripting/quest coverage exists (noted above).
2. Read every file in `src/scripting/` including the full VM: `lua4/value.rs` (182 lines), `lua4/instruction.rs` (132), `lua4/function.rs` (184), `lua4/vm.rs` (565), `mod.rs`, `quest.rs`, `quest_function_context.rs`, `quest_condition_functions.rs`, `quest_reward_functions.rs`, `script_function_context.rs`, `script_function_resources.rs`, `lua_game_constants.rs`, `lua_game_functions.rs`, `lua_quest_functions.rs`.
3. Read all consumers: `conversation_dialog_system.rs` (690 lines, the only Lua runtime host), `quest_trigger_system.rs`, `quest_trigger_event.rs`, `quest_scroll_event.rs`, `ui_quest_list_system.rs`, `ui_quest_scroll_system.rs`, plus the trigger dispatch sites in `collision_system.rs` (lines 250-643) and `game_connection_system.rs` (1600-1660).
4. Verified data structures in the dependency crates: `rose-data/src/quest_database.rs` (QuestDatabase/QuestTriggerHash), `rose-file-readers/src/qsd.rs` (QsdTrigger/QsdCondition/QsdItem), `rose-file-readers/src/con_.rs` (ConFile parse), `rose-data-irose/src/data_decoder.rs` (decode cost), `rose-game-common/src/components/quest_state.rs` (QuestState/ActiveQuest).
5. Traced every `call_lua_function` / `call_global_closure` / `quest_check_conditions` / `QuestTriggerEvent` site to determine execution frequency (see scope summary).
6. Bevy 0.18.1 APIs used here (`SystemParam`, `Query::single(_mut)`, `MessageWriter`, `Local`) are standard and were checked against `bevy-collection/bevy-0.18.1/crates/` where behavior mattered (no issues found; no engine-version risk).

---

## 3. Findings

### F1. Value type `Lua4Value` owns heap strings and deep-clones tables — every register/global/field read allocates

`src/scripting/lua4/value.rs:8-19` defines `Lua4Value` with `String(String)` (owned heap buffer) and `Table { fields: HashMap<String, Lua4Value>, array: Vec<Lua4Value> }`. `#[derive(Clone)]` (value.rs:7) therefore means **cloning a String value allocates, and cloning a Table deep-copies its entire HashMap and Vec**.

Every VM read op clones the value out of the stack/table/globals:
- `OP_GETLOCAL` — `vm.rs:214-220` `.clone()`
- `OP_GETGLOBAL` — `vm.rs:221-228` `.clone()` out of globals HashMap
- `table_get` — `vm.rs:67-81` `fields.get(key).cloned()` / `array.get(idx).cloned()`
- `OP_GETDOTTED`/`OP_GETINDEXED` — `vm.rs:235-240` (also clones the constant string, see F5)
- `OP_PUSHSELF` — `vm.rs:241-249` clones the table twice
- `OP_LFORLOOP` — `vm.rs:516-534` clones the iterator result
- parameter setup — `vm.rs:118` `parameters.get(i).cloned()`

**Impact:** ~1 heap allocation per string read, ~1 deep copy per table read. A 100-iteration loop reading one global string + one table field allocates ~400+ buffers; reading a **table** that lives in a global (common pattern: `tbl.x` in loops) deep-copies the whole table per read. Lua4 strings are immutable, so this is pure waste.
**Fix (preserves semantics — strings immutable, tables copy-on-clone today, see Risks):**
```rust
pub enum Lua4Value {
    Nil,
    UserData(Arc<dyn Any + Send + Sync>),
    Number(f64),
    String(Arc<str>),          // or Rc<str> — refcount instead of memcpy
    Table(Table),              // Table { fields: HashMap<String, Lua4Value>, array: Vec<Lua4Value> } kept as-is
    Closure(Arc<Lua4Function>, Arc<Vec<Lua4Value>>),
    RustClosure(String),
}
```
Keep `String` as `Arc<str>`/`Rc<str>` so reads are refcount increments; keep Table deep-clone semantics (see F4 and Risks) unless you intentionally move to reference semantics. This alone removes the majority of VM allocations.

### F2. `OP_PUSHSTRING` re-allocates every string literal on every execution

`vm.rs:193-197` — `function.constant_strings[kstr as usize].clone()` for each push. String literals are already in the function's constant table (`function.rs:29`) but are stored as plain `String`, so every execution of a literal (including inside loops) allocates a fresh copy.

**Impact:** any loop body containing a string literal allocates per iteration; the compiler/`from_u32` decode work is done once, but the value copies are not.
**Fix:** with F1's `Arc<str>` constants (`constant_strings: Vec<Arc<str>>`), `OP_PUSHSTRING` becomes an `Arc` clone (refcount increment, no allocation). Zero semantic change.

### F3. `OP_CALL`/`OP_TAILCALL` allocate and copy the whole stack tail, then clone every argument again

`vm.rs:133-154`:
- `stack.split_off(...)` allocates a new Vec and memcpies **all** remaining values (args + any live locals above the split point) into it;
- recursion into `call_lua_function` (vm.rs:140) pushes each parameter with `.cloned()` (vm.rs:118) — a second copy of every argument;
- `results.reverse()` + pop loop (vm.rs:150-153) to place `num_results`;
- `OP_TAILCALL` (vm.rs:155-179) still recursively calls `call_lua_function` and then copies results — it is **not** an actual tail call (recursion depth grows), and clears + refills the stack (vm.rs:170-177).
- Each `call_lua_function` allocates a fresh `Vec::with_capacity(max_stack_size)` (vm.rs:115). There is also **no call-depth limit** — a recursive quest script can overflow the native Rust stack (recursion is per-Lua-call, unguarded).

**Impact:** each Lua→Lua call = 1 Vec alloc + 1 memcpy of the tail + 1 clone per argument + 1 Vec alloc for the callee stack. For dialog scripts (which call helper functions per message) this multiplies the F1/F2 costs. The unbounded recursion is a robustness risk (crash) more than a perf issue.
**Fix:** switch to a single reusable stack + explicit frame markers (like real Lua): keep `stack: Vec<Lua4Value>` in a VM-call struct, push args in place, and only clone what is truly needed; add `call_depth` counter with a generous cap (e.g. 200) returning a VM error instead of overflowing the stack. Optionally implement real tail calls by replacing the current frame before re-entry.

### F4. Closures deep-copy their upvalue vectors on every clone

`Lua4Value::Closure(Arc<Lua4Function>, Vec<Lua4Value>)` (value.rs:17). Cloning a closure value — which happens on any `GETGLOBAL`/`GETLOCAL`/`GETTABLE` of a stored closure — deep-clones every captured upvalue (each upvalue may itself be a String/Table). `OP_CLOSURE` (vm.rs:535-541) captures upvalues by value from the stack.

**Impact:** closures stored in globals and fetched repeatedly (a common pattern for callback/menu functions) re-copy their whole environment each read.
**Fix:** wrap the upvalue vector in `Arc<Vec<Lua4Value>>` (as in F1) so clones are refcounted. Note: this preserves the current *by-value capture* behavior (see Risks — this VM does not share upvalues between closures the way real Lua 4 does; do not "fix" that silently).

### F5. `OP_GETDOTTED`/`OP_GETINDEXED` build a temporary `Lua4Value::String` key per access (extra allocation + double hash)

`vm.rs:235-240`: `let field_name = function.constant_strings[kstr].clone();` then `table_get(&table_value, &Lua4Value::String(field_name))` — the clone is moved into a fresh enum, which is then hashed inside `fields.get(...)` (vm.rs:69-70). So each `tbl.field` in a loop = 1 string alloc + 1 hash of the field name. With F1/F2 the clone disappears, but the wrapper still forces a hash of the constant on every access.

**Impact:** medium within loops (one alloc + one hash per dotted access).
**Fix:** after F1, use a zero-copy key: change `table_get` to accept `&str`/`f64` key or hash via `fields.get(field_name.as_ref())` directly, avoiding the temporary `Lua4Value` entirely.

### F6. Compiled script + parsed ConFile are recreated on every dialog open (no cache)

`conversation_dialog_system.rs:393-399`: each `ConversationDialogEvent` does a full `vfs.read_file::<ConFile, _>` (VFS file read + XOR decode of the whole script binary, `con_.rs:44-47` + full message/menu parse with ~2 string allocs per message, `con_.rs:104-120`), then `create_conversation_dialog` (lines 80-111) rebuilds a fresh `Lua4VM`, re-registers 28 constants + 18 rust-closure globals (each with `name.clone()` string allocs, lines 87-97), and re-decodes the entire Lua chunk via `Lua4Function::from_bytes` (line 99; recursive nested-function decode with a String alloc per constant/local/vararg name, `function.rs:110-183`).

**Impact:** repeatedly talking to the same NPC (or re-entering a zone) redoes file IO + full parse + full bytecode decode every time. Dialog opens are user-paced, so absolute cost is low, but it is 100% redundant work — a 50-KB con script decodes hundreds of Strings per open.
**Fix:** cache parsed artifacts keyed by path, e.g. a `Resource`:
```rust
#[derive(Resource, Default)]
struct ConScriptCache(HashMap<String, Arc<ConFile>>);        // parsed file
// and/or once F1 lands: HashMap<String, Arc<Lua4Function>>  // decoded chunk
```
`Lua4Function` is already `Arc`-friendly (`function.rs:36` returns `Arc<Lua4Function>`), so caching the decoded function is a small change. Invalidate only if scripts are moddable at runtime (they are not in this client). Per-open VM/globals setup (constants+closures) can stay cheap once values are `Arc`-based.

### F7. Globals: `HashMap<String, Lua4Value>` lookup per `GETGLOBAL`, and key clones per `SETGLOBAL`

`vm.rs:91-94, 221-228, 262-267`. GETGLOBAL already borrows the constant as `&str` (no alloc, one hash per access). SETGLOBAL clones the constant String to use as a HashMap key (alloc per write). With F1 (Arc constants) both become refcount ops.

**Impact:** low — the hash is only ~100 ns and dialog scripts are short. Not worth pre-binding globals to slot indices unless the VM becomes per-frame.

### F8. Quest trigger chain lookups: two HashMap hops per trigger, name-based hops per chain step

`process_quest_chain` (`src/scripting/quest.rs:20-71`) starts from `get_trigger_by_hash` which does **two** lookups — `triggers_by_hash: HashMap<QuestTriggerHash, String>` then `triggers.get(name)` (`rose-data/src/quest_database.rs:104-108`) — and every chain step does `get_trigger_by_name` with a String-key hash (quest.rs:57, 66). Hash→name→trigger requires hashing a 32-bit hash, then hashing the string.

**Impact:** low — chains are 1-3 triggers and evaluation is event-driven; this is a few hundred ns per chain. But it is trivially improvable.
**Fix:** store triggers directly by hash (`HashMap<QuestTriggerHash, Arc<QuestTrigger>>` or store `QuestTrigger` directly; the database is immutable after load at `lib.rs:1850-1853`). Also precompute each trigger's own `next_trigger_hash` at load so chain steps skip name hashing (QsdTrigger has `next_trigger_name: Option<String>`, `qsd.rs:522-527`).

### F9. Quest conditions re-decode raw QSD item/ability IDs on every evaluation

`quest_condition_functions.rs:45-51` (`decode_ability_type` per AbilityValue condition) and `:104-116` (`decode_item_reference` + `decode_equipment_index` per QuestItem condition), also repeated in reward functions (`quest_reward_functions.rs:39-46, 81-88`). The decodes are cheap match statements (`rose-data-irose/src/data_decoder.rs:559-628, 945-997`) — this is *not* a hot cost today.

**Impact:** negligible CPU (tens of ns), but it couples quest evaluation to `ScriptFunctionResources.game_data` when it could be done once at load.
**Fix (optional/low priority):** pre-decode `QsdCondition`/`QsdReward` into a cached parallel struct holding `ItemReference`/`AbilityType` when the quest database is loaded. Low risk, removes a class of failure (`None` fallbacks).

### F10. `QF_getNpcQuestZeroVal` linearly scans every NPC per call

`lua_quest_functions.rs:222-232` iterates `context.query_npc.iter()` until `npc.id == npc_id`. Each call is O(total NPCs in the zone) and it is callable from Lua conditions. Called rarely today (event-driven), but the data needed is already on the NPC component (`npc.quest_index`), so the loop is pure waste.

**Impact:** low (only matters with many NPCs or frequent calls).
**Fix:** maintain `HashMap<NpcId, usize>` (or use `Entity` lookup) once per zone, or query by component with a filtered `Query<(Entity, &Npc)>` and a small map built when NPCs spawn.

### F11. Rust-closure dispatch does two HashMap lookups and allocates a results Vec per call

`conversation_dialog_system.rs:56-78`: `call_rust_closure` probes `quest_functions.closures` then `game_functions.closures` (two hashes of the name per call), and every closure returns `vec![...]` (`lua_quest_functions.rs:11-27` `lua_i32_closure!`; `lua_game_functions.rs:72` etc.). Also `parameters: Vec<Lua4Value>` is moved in — the F3 split_off already allocated it.

**Impact:** low (a small Vec alloc + two hashes per call, event-driven).
**Fix:** merge both maps into one `HashMap<String, LuaClosure>` resource (names are disjoint), and have the VM pass a stack slice; return results into a caller-provided buffer if the Vec alloc shows up in profiling.

### F12. Per-frame dialog text layout while any NPC dialog is open

`conversation_dialog_system.rs:479-501` runs on **every frame** while a dialog is open: clones `generated_dialog.message` (LayoutJob), re-runs `fonts.layout_job` on the message and on every response, and rebuilds every response galley (lines 491-495). The message/response text only changes when the menu changes (event-driven, lines 425/447/677).

**Impact:** small-moderate — a few egui text layouts + several LayoutJob clones per frame per open dialog; scales with dialog length. This is the only per-frame scripting-adjacent cost.
**Fix:** cache the galleys in `GeneratedDialog` and re-layout only when `run_menu` regenerates the dialog (invalidate a dirty flag). Same pattern applies to `ui_quest_scroll_system.rs:159-168` (layout per frame while the scroll dialog is active) — cache once per `Show` event.

### F13. Per-frame collider allocation + shape intersection for event-object trigger detection

`collision_system.rs:596-641`: every frame, per collision player, `Collider::ball(1.0)` is allocated (line 597) and `rapier_context.intersect_shape` runs over the whole event-object/warp-object group, even if the player has not moved. The quest trigger **write** is throttled to 5 s (lines 616-624) but the expensive intersection test is not.

**Impact:** small-moderate — one heap alloc + a broad-phase scan per frame; negligible with a handful of event objects, meaningful in zones with many event/warp objects.
**Fix:** hoist the `Collider::ball(1.0)` to a `Local` (or reuse a cached collider), and skip the test when the player position delta since the last test is below a threshold (e.g. 10 cm) or when `time` since last check < 100 ms. Keep the 5-s cooldown for event writes.

### F14. `QuestTriggerEvent::ApplyRewards` is dispatched but ignored (dead code path)

`game_connection_system.rs:1632-1639` writes `ApplyRewards(trigger_hash)` on server `QuestTriggerResult{success:true}`, but `quest_trigger_system.rs:16` handles it with `ApplyRewards(_) => {}` — a no-op. `quest_apply_rewards`/`quest_triggers_apply_rewards` (`quest.rs:86-97`, `quest_reward_functions.rs:302-401`) is therefore never invoked from any call site (grep confirms only `mod.rs` exports and `quest.rs` definitions).

**Impact:** none on performance; this is dead code / an unimplemented feature. Rewards are currently applied server-side only. Worth either wiring it up (client-side quest-item rewards after server confirmation) or removing the arm to avoid confusion.

### F15. Per-condition ECS query re-acquisition

Every condition/reward function calls `script_context.query_quest.single()`/`single_mut()` and `query_player_stats.single()` independently (`quest_condition_functions.rs:38-43, 84-87, 118-123, 191-193`; `quest_reward_functions.rs:48-50, 118-120, 155-157, 180-183, 207-212, 228-230`). A trigger with 5 conditions does ~10-15 `single()` calls plus the same in rewards.

**Impact:** negligible (each is a cheap archetype lookup); not worth restructuring.

### F16. Logging per instruction / per condition

`vm.rs:125` `log::trace!` per instruction, `quest_condition_functions.rs:313-317` `log::debug!` per condition. With `trace`/`debug` disabled (default release) the level check short-circuits before formatting, so cost is ~zero. Only note: if the client is ever run with `RUST_LOG=lua=trace` for debugging, expect 10-100x VM slowdown — mention in docs.

---

## 4. Priority-Ranked Summary Table

| # | Finding | Impact | Effort | Priority |
|---|---------|--------|--------|----------|
| F1 | Value type: owned strings + deep-cloned tables on every read | High (VM hot cost) | Medium | **High** |
| F2 | OP_PUSHSTRING re-allocates literals | Medium (allocation per literal exec) | Low (comes with F1) | **High** |
| F3 | OP_CALL stack split_off + arg re-clone + unguarded recursion | Medium-High (call-heavy scripts; crash risk) | Medium | **High** |
| F6 | ConFile parse + Lua chunk decode per dialog open | Medium (redundant IO/parse per open) | Low | Medium-High |
| F12 | Per-frame dialog text layout while open | Medium (per-frame while dialog open) | Low | Medium |
| F13 | Per-frame collider alloc + intersection scan | Medium (per-frame, scales with zone objects) | Low | Medium |
| F4 | Closure upvalue vector deep-copied per clone | Low-Medium | Low (comes with F1) | Medium |
| F5 | GETDOTTED temp String key per access | Medium in loops | Low (comes with F1) | Medium |
| F8 | Trigger lookup double-hop + name hops per chain step | Low | Low | Medium |
| F10 | QF_getNpcQuestZeroVal O(N) scan | Low | Low | Medium |
| F14 | ApplyRewards no-op dead path | None (correctness/feature) | Low | Low |
| F11 | Double HashMap dispatch + Vec per rust call | Low | Low | Low |
| F7 | SETGLOBAL key clones | Low | Low (comes with F1) | Low |
| F9 | Re-decode QSD ids per evaluation | Negligible | Medium | Low |
| F15 | Repeated single() per condition | Negligible | — | Low |
| F16 | trace/debug logging | Zero when disabled | — | Info only |

---

## 5. Quick Wins

1. **F1+F2 (one combined change):** switch `Lua4Value::String` to `Arc<str>`/`Rc<str>` and `Lua4Function::constant_strings` to `Vec<Arc<str>>` (requires touching only `value.rs`, `function.rs`, `vm.rs` and the `TryFrom` conversions in `value.rs:140-181`). Removes ~80% of VM allocations with zero semantic change. String equality/ordering (`value.rs:45-103`) still works on the contents.
2. **F6:** cache `Arc<Lua4Function>` per con-file path in a new `Resource`; dialog opens stop re-decoding bytecode.
3. **F12:** cache dialog galleys; re-layout only when `run_menu` runs.
4. **F13:** `Local<Collider>` reuse + movement threshold before `intersect_shape`.
5. **F8:** store `HashMap<QuestTriggerHash, QuestTrigger>` directly (drop the name-hop) and precompute `next_trigger_hash`.
6. **F11:** merge quest+game closure maps into one lookup.

---

## 6. Risks / Considerations

**VM correctness is paramount — Lua 4 semantics must be preserved. Any refactor needs a regression harness (a few representative .con/.lua scripts replayed and diffed) before it can be considered safe.**

- **Table copy semantics are non-standard here and must be preserved.** This VM's `Table` clone is a **deep copy** (`value.rs:63-73` deep-compares, `#[derive(Clone)]` deep-copies). Real Lua 4 tables are reference types (`t2 = t1; t2.x = 1` mutates shared state). This VM's `OP_SETTABLE` (vm.rs:268-282) mutates the table *in the stack slot*, so the copy-vs-reference question only matters when a table is cloned through `GETGLOBAL`/`GETLOCAL`/parameter passing. If F1/F4 use `Rc` to share tables, **behavior could change** (aliasing). Decide deliberately: either keep deep-copy clones (safe, keep `HashMap`/`Vec` inside a `Table` struct) or move to reference semantics with `Rc<RefCell>`-style mutation and verify against original scripts. Recommend keeping current deep-copy semantics.
- **Upvalue capture is by value and possibly non-functional** — `OP_CLOSURE` captures stack values (vm.rs:535-541), `OP_PUSHUPVALUE` reads them from the callee's local stack area (vm.rs:204-213), and each closure holds its own `Vec` copy. Mutations via one closure never propagate to another. This is likely a latent correctness bug, not just perf; do not "optimize" it without first testing whether quest scripts depend on upvalue sharing (most ROSE con scripts don't use closures heavily, but verify).
- **OP_TAILCALL is not a tail call** — changing it to a real frame-replace tail call changes observable behavior only if scripts depend on call-stack depth (e.g. recursion counting); normal scripts are unaffected, but treat as a semantics-sensitive change.
- **Adding a call-depth guard** changes failure mode from "native stack overflow (abort)" to "Lua VM error" — strictly better, but a deep-recursive script that currently works up to native limits might start failing at a smaller bound; pick a generous cap (≥ 500).
- **`Lua4Value` size** is dominated by `Table` (~72+ bytes payload); if tables become `Rc`-shared the enum shrinks, which speeds up all stack pushes/pops — a free bonus of F1, but re-measure if `Lua4Value` becomes `Copy`-adjacent: don't introduce unsafe or `Box` hacks; plain `Rc`/`Arc` is sufficient.
- **Number→String formatting** (`value.rs:177`, `vm.rs:373`) uses `format!("{}", n)`; f64 formatting is locale-independent here, so keeping `Arc<str>` values must not switch to a different float formatter that changes quest/condition outputs (e.g., scientific vs decimal notation affects string compares in Lua `==`).
- **Caching parsed con files (F6)** is safe only because game data is immutable at runtime in this client (loaded once at `lib.rs:1850-1853`); if a future "reload data" feature arrives, the cache must be invalidated.
- **Event-driven nature is the biggest win already in place** — do not "optimize" by moving quest condition evaluation to per-frame polling; that would add the only real hot path this subsystem could have.
