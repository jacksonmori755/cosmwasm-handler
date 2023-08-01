use cw_utils::{parse_reply_execute_data, ParseReplyError};
use cosmwasm_std::{DepsMut, Env, Reply, Response, StdError, entry_point, to_binary, from_binary};

use crate::{consts::ISEND_ID, ContractError, state::{PENDING, PendingRequests}};

#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    // NONCE.save(deps.storage, &reply.id)?;
    // REQUEST.save(deps.storage, &to_binary(&reply)?)?;
    // Ok(Response::new())
    match reply.id {
        ISEND_ID => handle_i_send_reply(deps, reply),
        _ => {
            let err = StdError::generic_err("=========================invalid reply id==========================");
            return Err(ContractError::Std(err));
        }
    }
}

fn handle_i_send_reply(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    let execute_response = parse_reply_execute_data(reply);
    let request_identifier: u64;
    match execute_response {
        Ok(ok_resp) => {
            request_identifier = from_binary(&ok_resp.data.unwrap())?;
            let pending_requests: PendingRequests = PENDING.load(deps.storage)
                .unwrap_or(PendingRequests { requests: vec![] });
            let mut requests = pending_requests.requests.clone();
            requests.push(request_identifier);
            let new_pending_requests = PendingRequests {requests: requests};
            PENDING.save(deps.storage, &new_pending_requests)?;
        },
        Err(err) =>  {
            let err_str = match err {
                ParseReplyError::SubMsgFailure(str1) => format!("SubMsgFailure: {}", str1),
                ParseReplyError::ParseFailure(str2) => format!("ParseFailure: {}", str2),
                ParseReplyError::BrokenUtf8(_) => "BrokenUtf8".to_string(),
            };
            let err = StdError::generic_err(err_str);
            return Err(ContractError::Std(err));
        }
    }
    let response = Response::new().set_data(to_binary(&format!("handle_i_send_reply, request_identifier: {}", request_identifier))?);
    Ok(response)
}