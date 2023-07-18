use cosmwasm_std::{
    entry_point, from_slice, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, from_binary, wasm_execute, SubMsg, ReplyOn,
};

use crate::error::ContractError;
use crate::helper::{get_request_packet, abi_encode_string, assert_sent_sufficient_coin};
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ResolveRecordResponse, GatewayMsg};
use crate::state::{Config, NameRecord, CONFIG, NAME_RESOLVER, REQUEST};
use crate::execute::*;

const MIN_NAME_LENGTH: u64 = 3;
const MAX_NAME_LENGTH: u64 = 64;
const ISEND_ID: u64 = 125;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, StdError> {
    let config = Config {
        purchase_price: msg.purchase_price,
        transfer_price: msg.transfer_price,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Register { name } => execute_register(deps, env, info, name),
        ExecuteMsg::Transfer { name, to } => execute_transfer(deps, env, info, name, to),
        ExecuteMsg::IReceive {
            src_chain_id,
            request_sender,
            payload,
        } => execute_i_receive(deps, env, info, src_chain_id, request_sender, payload),
        ExecuteMsg::IAck {
            request_identifier, 
            exec_status, 
            exec_data 
        } => execute_i_ack(deps, env, request_identifier, exec_status, exec_data),
        ExecuteMsg::ISend { 
            version, 
            route_amount, 
            route_recipient, 
            dest_chain_id, 
            request_metadata, 
            gateway_address,
            handler_address,
            payload 
        } => execute_i_send(deps, env, version, route_amount, route_recipient, dest_chain_id, request_metadata, gateway_address, handler_address, payload)
    }
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ResolveRecord { name } => query_resolver(deps, env, name),
        QueryMsg::Config {} => to_binary::<ConfigResponse>(&CONFIG.load(deps.storage)?.into()),
        QueryMsg::Request {} => to_binary::<Option<Binary>>(&REQUEST.may_load(deps.storage)?),
    }
}

fn query_resolver(deps: Deps, _env: Env, name: String) -> StdResult<Binary> {
    let key = name.as_bytes();

    let address = match NAME_RESOLVER.may_load(deps.storage, key)? {
        Some(record) => Some(String::from(&record.owner)),
        None => None,
    };
    let resp = ResolveRecordResponse { address };

    to_binary(&resp)
}

