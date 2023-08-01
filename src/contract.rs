use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, from_binary,
};
use cw_storage_plus::KeyDeserialize;

use crate::error::ContractError;

use crate::helper::{abi_decode_to_binary, abi_encode_string};
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ResolveRecordResponse, LoadStatesResponse, CustomQueryMsg};
use crate::state::{Config, CONFIG, NAME_RESOLVER, REQUEST, NONCE, PENDING, PendingRequests, RESULT};
use crate::execute::*;

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
            packet,
        } => execute_i_receive(deps, env, info, src_chain_id, request_sender, packet),
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
        } => execute_i_send(deps, env, version, route_amount, route_recipient, dest_chain_id, request_metadata, gateway_address, handler_address, payload),
        ExecuteMsg::SetDappMetadata { 
            fee_payer_address,
            gateway_address
         } => set_dapp_metadata(deps, fee_payer_address, gateway_address)
    }
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ResolveRecord { name } => query_resolver(deps, env, name),
        QueryMsg::Config {} => to_binary::<ConfigResponse>(&CONFIG.load(deps.storage)?.into()),
        QueryMsg::LoadStates {} => load_states(deps),
        QueryMsg::IQuery { packet } => i_query(deps, env, packet),
    }
}

fn i_query(deps: Deps, _env: Env, payload: Binary) -> StdResult<Binary> {
    let decoded = abi_decode_to_binary(&payload).or(Err(StdError::generic_err("abi_decode_error".to_string())))?;
    let query_msg: CustomQueryMsg = from_binary(&decoded)?;
    match query_msg {
        CustomQueryMsg::Config {} => {
            let config = CONFIG.load(deps.storage)?;
            let result = abi_encode_string(&format!("{:?}", config));
            return Ok(to_binary(&result)?)
        },
        CustomQueryMsg::ResolveRecord { name } => {
            let key = name.as_bytes();
            let address = match NAME_RESOLVER.may_load(deps.storage, key)? {
                Some(record) => Some(String::from(&record.owner)),
                None => None,
            };
            let resp = ResolveRecordResponse { address };
            to_binary(&abi_encode_string(&format!("{:?}", resp)))
        }
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

fn load_states(deps: Deps) -> StdResult<Binary> {
    let mut name_resolver: Vec<(String, String)> = vec![];
    for item in NAME_RESOLVER.range(deps.storage, None, None, cosmwasm_std::Order::Ascending) {
        match item {
            Ok((key, namerecord)) => {
                let name = String::from_slice(&key)?;
                let addresss = namerecord.owner.to_string();
                name_resolver.push((name, addresss));
            },
            Err(_) => {continue;}
        }
    }

    let request = REQUEST.load(deps.storage).unwrap_or(Binary::from(b"empty"));
    let result = RESULT.load(deps.storage).unwrap_or(Binary::from(b"empty"));
    let nonce = NONCE.load(deps.storage).unwrap_or(1000000000000000000);
    let pending = PENDING.load(deps.storage).unwrap_or(PendingRequests {requests: vec![]}).requests;

    let load_states_response = LoadStatesResponse {
        name_resolver,
        request,
        result,
        nonce,
        pending
    };

    let res = to_binary(&load_states_response)?;
    Ok(res)

}