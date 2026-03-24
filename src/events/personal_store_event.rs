use bevy::prelude::{Entity, Message};

use rose_data::Item;
use rose_game_common::components::Money;

#[derive(Message)]
pub enum PersonalStoreEvent {
    OpenEntityStore(Entity),
    SetItemList {
        sell_items: Vec<(u8, Item, Money)>,
        buy_items: Vec<(u8, Item, Money)>,
    },
    BuyItem {
        slot_index: usize,
        item: Item,
    },
    UpdateBuyList {
        entity: Entity,
        item_list: Vec<(usize, Option<Item>)>,
    },
    UpdateSellList {
        entity: Entity,
        item_list: Vec<(usize, Option<Item>)>,
    },
}
