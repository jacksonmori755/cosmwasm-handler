use cosmwasm_std::{
    entry_point, from_binary, from_slice, to_binary, wasm_execute, Binary, Deps, DepsMut, Env,
    MessageInfo, ReplyOn, Response, StdError, StdResult, SubMsg,
};

use crate::error::ContractError;
use crate::helper::{
    abi_encode_string, assert_sent_sufficient_coin, get_request_packet, validate_name,
};
use crate::msg::{
    ConfigResponse, ExecuteMsg, GatewayMsg, InstantiateMsg, QueryMsg, ResolveRecordResponse,
};
use crate::state::{Config, NameRecord, CONFIG, NAME_RESOLVER, REQUEST};

const MIN_NAME_LENGTH: u64 = 3;
const MAX_NAME_LENGTH: u64 = 64;
const ISEND_ID: u64 = 125;

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

pub fn execute_i_receive(
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

pub fn execute_i_ack(
    deps: DepsMut,
    env: Env,
    request_identifier: u64,
    exec_status: bool,
    exec_data: Binary,
) -> Result<Response, ContractError> {
    REQUEST.save(deps.storage, &exec_data)?;
    let result_txt = format!("Ack from handler contract:\naddress:{}\nrequest_identifier: {}\nexec_status:{}\nexec_data:{:?}", 
    env.contract.address.to_string(), request_identifier, exec_status, exec_data);
    let result = abi_encode_string(&result_txt);
    Ok(Response::new().set_data(result))
}

pub fn execute_i_send(
    deps: DepsMut,
    env: Env,
    version: u64,
    route_amount: u64,
    route_recipient: String,
    dest_chain_id: String,
    request_metadata: Binary,
    gateway_address: String,
    handler_address: String,
    payload: Binary,
) -> Result<Response, ContractError> {
    let request_packet = get_request_packet(handler_address, payload);
    let i_send_msg = GatewayMsg::ISend {
        version,
        route_amount,
        route_recipient,
        dest_chain_id,
        request_metadata,
        request_packet,
    };
    let gateway_send_msg = wasm_execute(gateway_address, &i_send_msg, vec![])?;
    let submsg = SubMsg {
        id: ISEND_ID,
        gas_limit: None,
        reply_on: ReplyOn::Always,
        msg: gateway_send_msg.into(),
    };
    let response = Response::new().add_submessage(submsg);
    Ok(response)
}
