# Simplification Report 08 — SCRIPTING & CORE

**This is a PLAN ONLY. No code changes were made.**

- Project: `rose-offline-client` (Bevy 0.18, Rust)
- Scope: `src/scripting/` (2,995 LOC), `src/lib.rs` (2,782), `src/main.rs` (329), `src/types.rs` (7), `src/loader.rs` (10), `src/bundles/` (705), `src/resources/` (~3,900), `src/components/` (~2,500), `src/events/` (63 + files)
- Total analyzed: **~13,900 LOC**
- Note: `src/lib.rs:3` and `src/main.rs:1` both contain `#![allow(warnings)]` — **all dead code below is invisible to the compiler today**.

---

## 1. DEAD FILES (not even in the crate)

| File | LOC | Evidence |
|---|---|---|
| `src/types.rs` | 7 | `VisibilityType` defined, never referenced; `types` not declared in `lib.rs` |
| `src/loader.rs` | 10 | Orphaned method body (`read_asset_bytes` has no `impl`); `loader` not declared in `lib.rs`; would not compile if included |

**Fix:** delete both files. **Save: 17 LOC.**

---

## 2. lib.rs — THE APP BOOTSTRAP (2,782 lines, 813–2106 is one `run_client` function)

### 2a. Dead diagnostic systems registered in the app (~130 LOC)
- `lib.rs:855-874` — VFS diagnostic Startup system whose entire loop body is commented out (lines 868-873).
- `lib.rs:877-913` — Asset-server diagnostic system: 37 lines of pure log spam, no logic.
- `lib.rs:1459-1499` — Four "SCHEDULE CHECK" systems (`TransformPropagate`, `VisibilityPropagate`, `CheckVisibility`, `CalculateBounds`) whose only statements are commented out — 4 empty systems.
- `lib.rs:2076-2100` — Two EGUI diagnostic systems with all log lines commented out.
- `lib.rs:2425-2501` — `diagnose_camera_extraction_state`: 77-line function, **never registered** anywhere.
- **Fix:** delete all of the above. **Save: ~230 LOC.**

### 2b. Leftover test code
- `lib.rs:1988` registers `spawn_test_cube` (`lib.rs:2408-2421`) — a "Bevy 0.14.2 rendering isolation test" red cube spawned into the game world at (5100, 75, -5100). Not guarded by any flag.
- **Fix:** delete. **Save: ~15 LOC.**

### 2c. Dead system-set enums (~55 LOC)
Defined but referenced **only at their own definition** (verified across whole `src/`):
- `GameSystemOrdering` (768-780), `GameStateSystemSets` (782-788), `ModelViewerSystemSets` (790-793), `LoginSystemOrdering` (795-800), `CharacterSelectSystemOrdering` (802-811).
- **Fix:** delete 5 enums (keep `GameStages`, `UiSystemSets`, `ModelSystemSets`, `EffectSystemSets`, `UiSystemOrdering`, `GameSystemSets` which are actually used). **Save: ~55 LOC.**

### 2d. Duplicated VFS index-loading code (347-509)
`create_virtual_filesystem` has 4 near-identical ~30-line arms (`AruaVfs` 362-392, `TitanVfs` 393-423, `Vfs` 424-454, `IrosePh` 455-485), each repeating the same "compute parent dir of index path" logic and the same "push HostFilesystemDevice + set base_path" epilogue.
- **Fix:** extract `fn index_root_path(path: &str) -> PathBuf` and a helper `fn add_device(vfs_devices, base_path, index_root, log_name)`; each arm becomes ~8 lines. **Save: ~90 LOC.**

### 2e. Fragmented single-system registrations (~200 LOC)
138 `add_systems` calls, 159 ordering constraints (`.after/.before/.in_set/.run_if/.chain`), 13 `configure_sets`, 15 `add_plugins`, 35 `add_message`, 40 `init/insert_resource`, 96 `log::info!` diagnostics.
- The `AppState::Game` block (1634-1811) contains ~30 separate `app.add_systems(Update, single_system.run_if(in_state(AppState::Game)))` calls where the ordering is already expressed via `.after()`. Grouping (as already done at 1301-1339) would cut ~150 lines. Boat section (1729-1811) alone is 12 single-system calls.
- **Fix:** fold into batched `add_systems((...))` calls with `.after()` constraints; verify each `configure_sets` after grouping. **Save: ~150-200 LOC.**

### 2f. Diagnostic log noise
96 `log::info!` call sites in `run_client` (e.g. lines 815-999 region, 1060-1150, 236-2394 comments, "STARRY SKY" block 2643-2781 which logs ~25 lines per spawn). The `spawn_starry_sky_and_moon` function (2634-2782) spends ~140 of its 148 lines on `log::info!`.
- **Fix:** trim to a handful of `info!` at meaningful milestones. **Save: ~150 LOC** (mostly removed log lines).

### 2g. `match config.game.X_version` dispatch (1997-2020)
Three identical match blocks ("irose" / "custom" / panic) — acceptable, but could be one helper. Minor. **~10 LOC.**

---

## 3. scripting/lua4/ — hand-rolled Lua 4.0 VM (1,257 LOC)

### 3a. Over-engineering assessment
This is a from-scratch partial Lua 4.0 implementation (VM + bytecode parser + value type). It exists solely to run `.con` dialog scripts (`systems/conversation_dialog_system.rs`) and quest triggers. Only **4 game functions** (GF_getVariable, GF_openBank, GF_openStore, GF_organizeClan) and **14 quest functions** (QF_*) are registered — the closure-table infrastructure is ~1,800 lines around a 18-function runtime.
Options, in increasing effort:
1. **Compact the existing VM** (items 3b-3d): ~250 LOC saved, no behavior change.
2. Delete the two function-registry resources and the VM's `Lua4VMRustClosures` indirection; the only implementor is `LuaVMContext` in conversation_dialog_system.rs:56-78.
3. (Long-term) Replace the interpreter loop with a pre-decoded IR, or vendored Lua 4 interpreter — out of scope for quick wins.

### 3b. Near-identical match arms in `vm.rs` `call_lua_function` (lines 56-632)
- 5 binary arithmetic ops `OP_ADD/SUB/MULT/DIV/POW` (350-414) — identical "pop 2, match Numbers, push result, else Nil" skeleton; only the operator differs.
- 6 compare-jump ops `OP_JMPNE/JMPEQ/JMPLT/JMPLE/JMPGT/JMPGE` (446-493) — identical except the comparison.
- 4 table-lookup ops `OP_GETTABLE/GETDOTTED/GETINDEXED/PUSHSELF` (207-264) — the same "pop table, else NotTable, unwrap_or Nil" body 4x.
- Unary ops `OP_ADDI/MINUS/NOT` (363-370, 430-445) — same pop/match/push.
- **Fix:** extract helpers `fn pop_number(stack) -> Option<f64>` (replaces ~20 `stack.pop().ok_or(MissingStackValue)?` call sites) and generic `binary_arith(stack, f: impl Fn(f64,f64)->Option<f64>)`; comparison jumps via a `jump_if(stack, cmp, pc)` helper. **Save: ~150 LOC.**

### 3c. `OP_CALL` debug formatting (89-121)
Manual `write!`-based "Call rust closure: name(...) = [...]" builder with 4 near-identical loops (`take(1)` then `skip(1)`).
- **Fix:** `format!("{:?}", parameters)` in a single `log::debug!`. **Save: ~30 LOC.**

### 3d. Dead API in `lua4/` (~80 LOC)
- `Lua4Value::to_f32`, `to_f64`, `to_i64` (`value.rs:32-46`) — **never called** (verified whole repo); the `TryFrom<&Lua4Value> for f32/f64` impls (158-185) exist only to serve them → also deletable.
- `Lua4VMError::TableKeyNotFound` (`vm.rs:20-21`) — never constructed.
- 6 `TryFrom<&Lua4Value>` impls (158-246) all duplicate the same "Number, else parse String" pattern — collapse with a small macro. **Save: ~80 LOC.**

### 3e. `function.rs` header validation (72-142)
8 sequential "read u8, `bail!` on mismatch" blocks (83-134). A `fn expect_byte(reader, expected, label)` helper collapses each to 1 line. **Save: ~25 LOC.**

---

## 4. scripting/ — closure registration & quest logic (1,738 LOC)

### 4a. Repeated HashMap-registry boilerplate (3 resources)
`lua_game_functions.rs` (18-101), `lua_quest_functions.rs` (11-71), `lua_game_constants.rs` (25-104): each declares the closure/constant type inline 5+ times, e.g. `fn(&ScriptFunctionResources, &mut ScriptFunctionContext, Vec<Lua4Value>) -> Vec<Lua4Value>` written out 6x in `lua_game_functions.rs` alone.
- **Fix:** `pub type LuaClosure = fn(&ScriptFunctionResources, &mut ScriptFunctionContext, Vec<Lua4Value>) -> Vec<Lua4Value>;` in `scripting/mod.rs`, plus a `lua_closures!` macro for the insert lists. **Save: ~40 LOC.**

### 4b. 10 near-identical QF_* getters (`lua_quest_functions.rs`)
`QF_findQuest` (121-137), `QF_getEpisodeVAR` (157-171), `QF_getJobVAR` (174-188), `QF_getPlanetVAR` (191-205), `QF_getQuestCount` (208-226), `QF_getQuestID` (229-244), `QF_getQuestSwitch` (276-292), `QF_getQuestVar` (295-311), `QF_getUserSwitch` (314-328), `QF_getNpcQuestZeroVal` (331-349) all share the exact skeleton:
`let result = || -> Option<i32> { params → query → map }().unwrap_or(-1); vec![result.into()]`
- **Fix:** macro `lua_i32_closure!(QF_name, |params, ctx| -> Option<i32> { ... })`. **Save: ~100 LOC.**

### 4c. Commented-out "planned function" lists
- `lua_game_functions.rs:42-97` — 55-line comment block of unimplemented GF_* functions.
- `lua_quest_functions.rs:45-67` — 22-line comment block of QF_* functions.
- **Fix:** delete or move to an issue tracker. **Save: ~77 LOC.**

### 4d. `quest.rs` — `quest_check_conditions` vs `quest_apply_rewards`
`quest_check_conditions` (13-63) and `quest_apply_rewards` (65-115) are byte-for-byte the same ~50-line while-loop; only the per-trigger callback differs (`quest_triggers_skip_rewards` vs `quest_triggers_apply_rewards`).
- **Fix:** one generic `fn process_quest_chain(..., mode: QuestChainMode)` (or an `fn` parameter). **Save: ~45 LOC.**

### 4e. No-op stubs & stale TODOs
- `quest_reward_functions.rs:292-301` — `quest_reward_call_lua_function` returns `true` and does nothing (TODO).
- `quest_reward_functions.rs:63-64, 143-144, 172-173, 201-202, 298-299` — 5 repeated "TODO: Event writers removed" comments.
- `script_function_context.rs:69-73` (`queue_chatbox_event`) and `90-94` (`queue_system_func_event`) — **never called** (verified); the 5 `queue_*` methods are 5 identical 4-line closures → 1 generic `queue_event`.
- **Fix:** remove dead queues + stubs. **Save: ~30 LOC.**

### 4f. `script_function_context.rs` giant tuple queries (20-58)
`query_player_stats` is a 6-tuple query, `query_player_mutable` a 9-tuple query; consumers re-document the tuple layout via comments (`quest_reward_functions.rs:225-226`, `quest_condition_functions.rs:53-54, 125`). Fragile and verbose.
- **Fix:** introduce named param structs (e.g. `PlayerStatsQuery`) with one tuple query each, used by both the scripting layer and `systems/game_connection_system.rs`. **Save: ~30 LOC + removes 3 duplicated comment blocks.**

### 4g. `lua_game_constants.rs` constant registration (30-102)
28 `constants.insert(...)` calls; the SV_* set duplicates each constant 3x (Rust const + map key string + value).
- **Fix:** build from a single `[("SV_SEX", SV_SEX.into()), ...]` array; ITEM_TYPE_* can be a small `item_type_constants!` macro over the 13 entries. **Save: ~35 LOC.**

---

## 5. bundles/ability_values.rs (699 LOC) — worst duplication in the codebase

### 5a. 4 near-identical 30-arm match statements
- `ability_values_add_value` (148-252) vs `ability_values_add_value_exclusive` (254-408): same arms, wrapped differently — ~160 duplicated lines.
- `ability_values_set_value` (411-519) vs `ability_values_set_value_exclusive` (521-698): same — ~200 duplicated lines.
- `UnionPoint1..UnionPoint10` arm repeated **10x in each of the 4 functions = 40 copies** of the same 4-line arm.
- **Fix:** implement each pair via a shared core (e.g. non-exclusive delegates to exclusive via `EntityWorldMut`, or both delegate to small `set_*`/`add_*` helpers on the component structs); emit the UnionPoint arms with a macro or array-based index access (`points[i]`). **Save: ~300 LOC.**

### 5b. Dead functions masked by `#[allow(dead_code)]`
- `ability_values_set_value` (411-519): **never called** anywhere (verified).
- `ability_values_add_value` (148-252): called exactly once (`systems/use_item_event_system.rs:91`); could be switched to the exclusive variant → both become dead.
- **Fix:** delete `set_value`; migrate the one caller; delete `add_value`. **Save: ~160 LOC.**

---

## 6. resources/

### 6a. `ui_resources.rs` (709 LOC)
- `ui_requested_cursor_apply_system` (700-709): **completely empty body** (all behavior commented out), yet registered in lib.rs:1344-1347. Delete → 10 LOC.
- `get_sprite` / `get_sprite_by_index` / `get_sprite_image` / `get_sprite_image_by_index` (153-275): 4 hand-unrolled lookup chains with repeated `match/return None` blocks; `get_sprite_image_by_index` is a strict subset of `get_sprite_by_index`. Merge into 2 methods using `?`. → ~60 LOC.
- `UiResources` holds 22 individual `dialog_*: Handle<Dialog>` fields (113-136) *plus* a `dialog_files: HashMap` (113) — the struct fields could read from the HashMap (or vice versa). → ~25 LOC.
- 10x repeated `.map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok()` (607-615) → one `load_or_log()` helper. → ~15 LOC.
- **Total: ~110 LOC.**

### 6b. `zone_debug_diagnostics.rs` (433 LOC)
Debug scaffolding: `ZoneDebugDiagnosticsPlugin` (425-433) is **never added in lib.rs**; `zone_child_visibility_diagnostic_system` (271-422) is gated behind an env var; `zone_memory_profiler_system.rs`/`zone_render_validation_system.rs` consume `ZoneDebugDiagnostics` (systems/zone_memory_profiler_system.rs:349, zone_render_validation_system.rs:13) but are themselves not registered in lib.rs. Whole cluster appears to be orphaned debug tooling.
- **Fix:** verify nothing registers `zone_memory_profiler_system`/`zone_render_validation_system`; if orphaned, delete the trio or gate behind a cargo feature. **Save: up to ~430 LOC + 2 system files (~1,100 LOC in systems/).**

### 6c. `season_settings.rs` (206 LOC)
`SummerSettings` (110-172) carries 6 `#[deprecated]` fields for "CPU-based grass" — the CPU grass path is disabled (`bevy_procedural_grass` incompatible with Bevy 0.18, lib.rs:52). Dead configuration surface. → ~30 LOC.

### 6d. `damage_digits_spawner.rs` (154 LOC)
~80 lines of the file are `log::info!` diagnostics in `load()` (32-70) and `spawn()` (91-145). Trim to 2-3 messages. → ~50 LOC.

---

## 7. components/

### 7a. `command.rs` (306 LOC) — duplicated constructors
`NextCommand` (219-292) is a newtype over `Option<Command>` that re-implements **13 of Command's 13 `with_*` constructors** (`with_attack`, `with_cast_skill`, `with_emote`, `with_move`, ...) with copy-pasted body + `Some(...)` wrapper (compare 74-141 vs 222-291). ~90 duplicated lines.
- **Fix:** drop the duplicate constructors; callers use `NextCommand(Some(Command::with_attack(t)))` (or add just the 2-3 actually used). Check usage before deleting each. **Save: ~90 LOC.**

### 7b. Minor
- `blood_overlay.rs:37-68` — `BloodStain::new` / `new_for_material` share 7 of 8 fields; add an `Option<Entity>` parameter. ~10 LOC.
- `command.rs:75-77, 79-81` — `with_die()`/`with_stop()` one-liners wrapping unit variants; inline. ~4 LOC.
- Remaining component files (`cooldowns.rs`, `particle_sequence.rs`, `wind_effect.rs`, `blood_effect.rs`, `collision.rs`) are reasonably lean — no action.

---

## 8. main.rs (329 LOC)
- Uses deprecated clap 3.x API (`takes_value`, `value_of`, `is_present`) — migrating to `#[derive(Parser)]` + `TryFrom<ArgMatches>` or a small config-assembly helper would cut the ~90 lines of manual `if let Some(x) = matches.value_of(...)` blocks by half.
- Leftover `println!` debug lines (14-15, 163-167, 281, 304-327) — trim.
- **Save: ~60-100 LOC.**

---

## 9. events/ (skim)
- 34 small event files + `mod.rs` re-exports (63 lines). No duplication or dead code of note; individual events are 5-100 lines. Leave as-is.

---

## PRIORITIZED SUMMARY — Top 5 quick wins first

| # | Item | File:lines | Save |
|---|---|---|---|
| 1 | Remove `#![allow(warnings)]` (lib.rs:3, main.rs:1) and delete the dead code it hides: `diagnose_camera_extraction_state` (2425-2501), 4 empty SCHEDULE-CHECK systems (1459-1499), VFS/asset/EGUI diagnostics (855-913, 2076-2100), `spawn_test_cube` (2408-2421), 5 unused system-set enums (768-811), orphaned `types.rs`/`loader.rs`, no-op `ui_requested_cursor_apply_system` | lib.rs, types.rs, loader.rs, ui_resources.rs | ~450 |
| 2 | Collapse `ability_values.rs` 4-way duplication; delete dead `ability_values_set_value`; migrate the single `ability_values_add_value` caller to exclusive | bundles/ability_values.rs | ~460 |
| 3 | Extract helpers in lua4 VM: binary arith/cmp/jump helpers + pop_number; delete dead `to_f32/to_f64/to_i64` + `TableKeyNotFound`; one-line the OP_CALL debug formatting | scripting/lua4/vm.rs, value.rs, function.rs | ~280 |
| 4 | Deduplicate quest chain: merge `quest_check_conditions`/`quest_apply_rewards`; macro the 10 identical QF_* getters; delete 77 LOC of commented-out function lists; remove dead `queue_chatbox_event`/`queue_system_func_event` | scripting/quest.rs, lua_quest_functions.rs, lua_game_functions.rs, script_function_context.rs | ~300 |
| 5 | Clean up bootstrap: extract VFS index-root helper in `create_virtual_filesystem`; batch the ~30 single-system `AppState::Game` registrations; trim 96 diagnostic `log::info!` calls (incl. 140-line STARRY SKY log block); remove deprecated SummerSettings fields | lib.rs, season_settings.rs | ~450 |

**Additional candidates (verify then decide):** orphaned `zone_debug_diagnostics` + `zone_memory_profiler_system` + `zone_render_validation_system` cluster (~1,500 LOC in resources+systems), `command.rs` NextCommand duplicate constructors (~90), `ui_resources.rs` dialog fields vs HashMap duplication (~110), `main.rs` clap 3→4 migration (~80).

**Bottom line:** conservative estimate **~1,900-2,300 LOC** removable/reducible in this scope without behavior change; the single biggest structural decision is what to do with the hand-rolled Lua 4.0 VM (keep-but-compact vs replace), which gates a further ~800 LOC.

---
*Report generated by research-only sub-agent. No files were modified.*
