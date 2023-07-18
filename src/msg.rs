use crate::state::Config;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Coin};

#[cw_serde]
pub struct InstantiateMsg {
    pub purchase_price: Option<Coin>,
    pub transfer_price: Option<Coin>,
}

#[cw_serde]
pub enum ExecuteMsg {
    ISend {
        version: u64,
        route_amount: u64,
        route_recipient: String,
        dest_chain_id: String,
        request_metadata: Binary,
        gateway_address: String,
        handler_address: String,
        payload: Binary,
    },
    IReceive {
        src_chain_id: String,
        request_sender: String,
        payload: Binary,
    },
    IAck {
        request_identifier: u64,
        exec_status: bool,
        exec_data: Binary
    },
    Register {
        name: String,
    },
    Transfer {
        name: String,
        to: String,
    }
}

#[cw_serde]
pub enum CustomExecuteMsg {
    Register {
        name: String,
    },
    Transfer {
        name: String,
        to: String,
    } 
}

#[cw_serde]
pub enum GatewayMsg {
    ISend {
        version: u64,
        route_amount: u64,
        route_recipient: String,
        dest_chain_id: String,
        request_metadata: Binary,
        request_packet: Binary,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // ResolveAddress returns the current address that the name resolves to
    #[returns(ResolveRecordResponse)]
    ResolveRecord { name: String },
    #[returns(ConfigResponse)]
    Config {},
    #[returns(RequestResponse)]
    Request {},
}

pub enum CustomQueryMsg {
        // ResolveAddress returns the current address that the name resolves to
        ResolveRecord { name: String },
        Config {},
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ResolveRecordResponse {
    pub address: Option<String>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub purchase_price: Option<Coin>,
    pub transfer_price: Option<Coin>,
}

impl From<Config> for ConfigResponse {
    fn from(config: Config) -> ConfigResponse {
        ConfigResponse {
            purchase_price: config.purchase_price,
            transfer_price: config.transfer_price,
        }
    }
}


#[cw_serde]
pub struct RequestResponse {
    pub request: Option<Binary>,
}


#[cw_serde]
pub struct ResolveResultResponse {
    pub result: Option<Binary>,
}