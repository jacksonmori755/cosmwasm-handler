use cosmwasm_std::{
    entry_point, from_slice, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, from_binary, wasm_execute, SubMsg, ReplyOn,
};

use crate::coin_helpers::assert_sent_sufficient_coin;
use crate::error::ContractError;
use crate::helper::{get_request_packet, abi_encode_string};
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ResolveRecordResponse, GatewayMsg};
use crate::state::{Config, NameRecord, CONFIG, NAME_RESOLVER, REQUEST};

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

pub fn execute_register(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    // we only need to check here - at point of registration
    validate_name(&name)?;
    let config = CONFIG.load(deps.storage)?;
    assert_sent_sufficient_coin(&info.funds, config.purchase_price)?;

    let key = name.as_bytes();
    let record = NameRecord { owner: info.sender };

    if (NAME_RESOLVER.may_load(deps.storage, key)?).is_some() {
        // name is already taken
        return Err(ContractError::NameTaken { name });
    }

    // name is available
    NAME_RESOLVER.save(deps.storage, key, &record)?;

    Ok(Response::default())
}

pub fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
    to: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    assert_sent_sufficient_coin(&info.funds, config.transfer_price)?;

    let new_owner = deps.api.addr_validate(&to)?;
    let key = name.as_bytes();
    NAME_RESOLVER.update(deps.storage, key, |record| {
        if let Some(mut record) = record {
            if info.sender != record.owner {
                return Err(ContractError::Unauthorized {});
            }

            record.owner = new_owner.clone();
            Ok(record)
        } else {
            Err(ContractError::NameNotExists { name: name.clone() })
        }
    })?;
    Ok(Response::default())
}

fn execute_i_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _src_chain_id: String,
    _request_sender: String,
    payload: Binary,
) -> Result<Response, ContractError> {
    let msg = from_binary(&payload)?;
    match msg {
        ExecuteMsg::Register { name } => execute_register(deps, env, info, name),
        ExecuteMsg::Transfer { name, to } => execute_transfer(deps, env, info, name, to),
        _ => Err(StdError::generic_err("msg").into()),
    }
}

fn execute_i_ack(
    deps: DepsMut, 
    env:Env, 
    request_identifier: u64, 
    exec_status: bool, 
    exec_data: Binary
) -> Result<Response, ContractError> {
    REQUEST.save(deps.storage, &exec_data)?;
    let result_txt = format!("Ack from handler contract:\naddress:{}\nrequest_identifier: {}\nexec_status:{}\nexec_data:{:?}", 
    env.contract.address.to_string(), request_identifier, exec_status, exec_data);
    let result = abi_encode_string(&result_txt);
    Ok(Response::new().set_data(result))
}

fn execute_i_send (
    deps: DepsMut,
    env: Env,
    version: u64, 
    route_amount: u64, 
    route_recipient: String, 
    dest_chain_id: String, 
    request_metadata: Binary,
    gateway_address: String, 
    handler_address: String,
    payload: Binary
) -> Result<Response, ContractError> {
    let request_packet = get_request_packet(handler_address, payload);
    let i_send_msg = GatewayMsg::ISend { 
        version, 
        route_amount, 
        route_recipient, 
        dest_chain_id, 
        request_metadata, 
        request_packet 
    };
    let gateway_send_msg = wasm_execute(gateway_address, &i_send_msg, vec![])?;
    let submsg = SubMsg {
        id: ISEND_ID,
        gas_limit: None,
        reply_on: ReplyOn::Always,
        msg: gateway_send_msg.into()
    };
    let response = Response::new().add_submessage(submsg);
    Ok(response)
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

// let's not import a regexp library and just do these checks by hand
fn invalid_char(c: char) -> bool {
    let is_valid =
        c.is_ascii_digit() || c.is_ascii_lowercase() || (c == '.' || c == '-' || c == '_');
    !is_valid
}

/// validate_name returns an error if the name is invalid
/// (we require 3-64 lowercase ascii letters, numbers, or . - _)
fn validate_name(name: &str) -> Result<(), ContractError> {
    let length = name.len() as u64;
    if (name.len() as u64) < MIN_NAME_LENGTH {
        Err(ContractError::NameTooShort {
            length,
            min_length: MIN_NAME_LENGTH,
        })
    } else if (name.len() as u64) > MAX_NAME_LENGTH {
        Err(ContractError::NameTooLong {
            length,
            max_length: MAX_NAME_LENGTH,
        })
    } else {
        match name.find(invalid_char) {
            None => Ok(()),
            Some(bytepos_invalid_char_start) => {
                let c = name[bytepos_invalid_char_start..].chars().next().unwrap();
                Err(ContractError::InvalidCharacter { c })
            }
        }
    }
}
