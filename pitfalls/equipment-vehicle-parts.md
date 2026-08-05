# Equipment: Vehicle Parts (Cart / Castle Gear)

## Cart wheels could not be equipped as move parts

### Problem
Cart body, engine, and accessory equipped fine, but cart wheels never equipped
into the move-part (Leg) slot, with no error shown. Castle gear legs equipped
normally in the same slot.

### Root cause
`check_vehicle_type_consistency` in the server's
`rose-offline-server/src/game/systems/equipment_event_system.rs` enforced the
"no mixing cart / castle gear parts" rule using a slot-index heuristic instead
of item data:

- `Body` / `Engine` slots -> type unknown (`None`, never rejected)
- `Leg` slot -> assumed `CastleGear`
- `Arms` slot -> assumed `CastleGear`

But cart wheels use the `Leg` slot and cart accessories use the `Arms` slot.
With a cart accessory equipped, equipping cart wheels was rejected as
"MixedVehicleTypes" (accessory misidentified as CastleGear vs new part type
Cart). Castle gear legs passed only because the wrong heuristic happened to
match. The rejection was silent (`Result` discarded with `.ok()`), so the
client simply showed nothing.

### Fix
Look up each equipped part's real `vehicle_type` from the item database, and
skip the target slot since its current item is swapped back to inventory:

```rust
fn check_vehicle_type_consistency(
    game_data: &GameData,
    equipment: &Equipment,
    vehicle_part_index: VehiclePartIndex,
    new_vehicle_type: VehicleType,
) -> bool {
    for (part_index, vehicle_item) in equipment.equipped_vehicle.iter() {
        if part_index == vehicle_part_index {
            continue;
        }
        if let Some(item) = vehicle_item {
            if let Some(item_data) = game_data.items.get_vehicle_item(item.item.item_number) {
                if item_data.vehicle_type != new_vehicle_type {
                    return false;
                }
            }
        }
    }
    true
}
```

### Files modified
- `rose-offline/rose-offline-server/src/game/systems/equipment_event_system.rs`
  (`check_vehicle_type_consistency` + call site in `equip_vehicle_from_inventory`)

### Lesson learned
Vehicle part slots are shared between vehicle types (`Leg` = cart wheels OR
castle gear legs, `Arms` = cart accessory OR castle gear weapon). Never infer
an item's type/class from its equipment slot index; always resolve the item in
the item database. Also note that failed equip requests are silently dropped,
so server-side validation bugs surface as "nothing happens" in the client.
