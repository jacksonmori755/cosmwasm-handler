use cosmwasm_std::{
    from_binary, wasm_execute, Binary, DepsMut, Env, Event, 
    MessageInfo, ReplyOn, Response, StdError, SubMsg, to_binary,
};
use router_wasm_bindings::ethabi::Contract;

use crate::error::ContractError;
use crate::helper::{
    abi_decode_to_binary, abi_encode_string, assert_sent_sufficient_coin, get_request_packet,
    validate_name,
};
use crate::msg::{
    ExecuteMsg, GatewayMsg, CustomExecuteMsg,
};
use crate::state::{NameRecord, CONFIG, NAME_RESOLVER, REQUEST, RESULT};

use crate::consts::ISEND_ID;

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
    let record = NameRecord {
        owner: info.sender.clone(),
    };

    if (NAME_RESOLVER.may_load(deps.storage, key)?).is_some() {
        // name is already taken
        return Err(ContractError::NameTaken { name });
    }

    // name is available
    NAME_RESOLVER.save(deps.storage, key, &record)?;
    let result_txt = format!(
        "execute_register, name: {}, owner: {}",
        name,
        info.sender.to_string()
    );
    let result = abi_encode_string(&result_txt);
    RESULT.save(deps.storage, &result)?;
    let response = Response::new().set_data(result);
    Ok(response)
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
    let result_txt = format!("execute_register, name: {}, to: {}", name, to);
    let result = abi_encode_string(&result_txt);
    RESULT.save(deps.storage, &result)?;
    let response = Response::new().set_data(result);
    Ok(response)
}

pub fn execute_i_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _src_chain_id: String,
    _request_sender: String,
    payload: Binary,
) -> Result<Response, ContractError> {
    // abi decode payload, which was encoded by request sender
    let decoded = abi_decode_to_binary(&payload)?;

    // save request packet for debug
    REQUEST.save(deps.storage, &decoded)?;

    let msg: CustomExecuteMsg = from_binary(&decoded)?;
    match msg {
        CustomExecuteMsg::Register { name } => execute_register(deps, env, info, name),
        CustomExecuteMsg::Transfer { name, to } => execute_transfer(deps, env, info, name, to),
    }
}

pub fn execute_i_ack(
    deps: DepsMut,
    env: Env,
    request_identifier: u64,
    exec_status: bool,
    exec_data: Binary,
) -> Result<Response, ContractError> {
    // abi decode payload, which was encoded by request sender
    let decoded = abi_decode_to_binary(&exec_data)?;

    // save ack packet in store for debug
    REQUEST.save(deps.storage, &decoded)?;

    let result_txt = format!("Ack from handler contract:\naddress: {}\nrequest_identifier: {}\nexec_status:{}\nexec_data:{:?}", 
    env.contract.address.to_string(), request_identifier, exec_status, decoded);

    // abi encode result_txt
    let result = abi_encode_string(&result_txt);

    // save result in store for debug
    RESULT.save(deps.storage, &result)?;

    Ok(Response::new().set_data(result))
}

pub fn execute_i_send(
    _deps: DepsMut,
    _env: Env,
    version: u64,
    route_amount: u64,
    route_recipient: String,
    dest_chain_id: String,
    request_metadata: Binary,
    gateway_address: String,
    handler_address: String,
    payload: Binary,
) -> Result<Response, ContractError> {
    // request_packet = abi encode(handler_address, payload)
    let request_packet = get_request_packet(&handler_address, &payload);

    // create ISend msg of gateway contract
    let i_send_msg = GatewayMsg::ISend {
        version,
        route_amount,
        route_recipient,
        dest_chain_id: dest_chain_id.clone(),
        request_metadata,
        request_packet: request_packet.clone(),
    };

    let gateway_send_msg = wasm_execute(gateway_address, &i_send_msg, vec![])?;
    let submsg = SubMsg {
        id: ISEND_ID,
        gas_limit: None,
        reply_on: ReplyOn::Always,
        msg: gateway_send_msg.into(),
    };

    let response = Response::new()
        .add_event(Event::new("ISend")
                .add_attribute("dest_chain_id", dest_chain_id)
                .add_attribute("handler_address", handler_address)
                .add_attribute("payload", payload.to_base64())
                .add_attribute("request_packet", request_packet.to_base64()),
        )
        .add_submessage(submsg);

    Ok(response)
}

pub fn set_dapp_metadata(
    _deps: DepsMut,
    fee_payer_address: String,
    gateway_address: String,
) -> Result<Response, ContractError> {
    // create SetDappMetadata msg of gateway contract
    let set_dapp_metadata_msg = GatewayMsg::SetDappMetadata {
        fee_payer_address: fee_payer_address.clone(),
    };

    let gateway_send_msg = wasm_execute(gateway_address, &set_dapp_metadata_msg, vec![])?;
    let submsg = SubMsg {
        id: ISEND_ID,
        gas_limit: None,
        reply_on: ReplyOn::Always,
        msg: gateway_send_msg.into(),
    };

    let response = Response::new()
        .add_submessage(submsg)
        .add_attribute("fee_payer_address", fee_payer_address);

    Ok(response)
}
