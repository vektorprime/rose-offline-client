# Personal Store Drag and Drop Buy

## Personal Store (Vending Skill) Items Could Not Be Bought by Dragging (Fixed 2026-08-02)

### Problem
Browsing a personal store (player/bot using the vending skill) worked, but dragging a store item
onto the inventory did nothing: the ghost icon followed the cursor, yet no drop highlight appeared
and releasing was a no-op. Buying was only possible via double-click, and when buying a stackable
item it always purchased the **entire stack** with no way to choose a quantity.

### Root Cause
Two separate gaps in the UI drag-and-drop wiring:

1. `ui_inventory_system.rs`'s `drag_accepts()` only accepted `DragAndDropId::NpcStore(_, _)`
   (NPC store). `DragAndDropId::PersonalStoreSell(_)` was not accepted, so the inventory slots
   never showed the drop highlight and `dropped_item` was never set for them.
2. The personal store buy path (`ui_personal_store_system.rs`) only had a double-click flow that
   sent `PersonalStoreEvent::BuyItem` with the store item at its full quantity. There was no
   quantity selection step.

### Solution
- `src/events/personal_store_event.rs`: added `RequestBuyItem { slot_index }`.
- `src/ui/ui_inventory_system.rs`: `drag_accepts()` now accepts
  `DragAndDropId::PersonalStoreSell(_)`; `ui_add_inventory_slot` emits `RequestBuyItem` when such a
  drop lands on an inventory slot.
- `src/ui/ui_personal_store_system.rs`: handles `RequestBuyItem` by looking up the item/price in
  its store state, then:
  - stackable item with quantity > 1 → `NumberInputDialogEvent::Show` (max = available quantity);
    the OK callback builds the item at the chosen quantity via
    `StackableItem::new(item.get_item_reference(), q)` (plain data, so it can be captured in the
    `'static` callback without borrowing `Res<GameData>`), then chains a `MessageBoxEvent` confirm
    showing `quantity x name for total Zuly`, whose OK writes `PersonalStoreEvent::BuyItem`;
  - non-stackable / single item → existing confirm box unchanged.
- The server (`rose-offline` `personal_store_buy_item`) already prices by
  `unit_price * quantity` and uses `try_take_quantity(quantity)`, so partial-stack purchases work
  end to end.

### Files Modified
- `src/events/personal_store_event.rs`
- `src/ui/ui_inventory_system.rs`
- `src/ui/ui_personal_store_system.rs`

### Lesson Learned
When a drop target needs data owned by another system's `Local` state (the personal store item
list), pass only an index through an event and let the owning system resolve the item. For
`'static` UI callbacks, capture only owned/plain data (`Item` clones, `usize`, `Money`) — never a
`Res` borrow — or the closure will fail to compile against the `Send + Sync` bound.
