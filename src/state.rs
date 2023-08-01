use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Binary};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub purchase_price: Option<Coin>,
    pub transfer_price: Option<Coin>,
}

#[cw_serde]
pub struct NameRecord {
    pub owner: Addr,
}

#[cw_serde]
pub struct PendingRequests {
    pub requests: Vec<u64>
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const NAME_RESOLVER: Map<&[u8], NameRecord> = Map::new("name_resolver");

pub const REQUEST: Item<Binary> = Item::new("request");
pub const RESULT: Item<Binary> = Item::new("result");
pub const NONCE: Item<u64> = Item::new("nonce");

pub const PENDING: Item<PendingRequests> = Item::new("pending");
