use cosmwasm_std::{Binary, StdError};
use hex;
use router_wasm_bindings::ethabi::{ethereum_types::{Address, U256}, ParamType, decode, encode, Token};
use cosmwasm_std::Coin;

use crate::ContractError;

const MIN_NAME_LENGTH: u64 = 3;
const MAX_NAME_LENGTH: u64 = 64;
const ISEND_ID: u64 = 125;

pub struct TakeLastXBytes(pub usize);

pub enum SolidityDataType<'a> {
    String(&'a str),
    Address(Address),
    Bytes(&'a [u8]),
    Bool(bool),
    Number(U256),
    NumberWithShift(U256, TakeLastXBytes),
}

/// Pack a single `SolidityDataType` into bytes
fn pack<'a>(data_type: &'a SolidityDataType) -> Vec<u8> {
    let mut res = Vec::new();
    match data_type {
        SolidityDataType::String(s) => {
            res.extend(s.as_bytes());
        }
        SolidityDataType::Address(a) => {
            res.extend(a.0);
        }
        SolidityDataType::Number(n) => {
            for b in n.0.iter().rev() {
                let bytes = b.to_be_bytes();
                res.extend(bytes);
            }
        }
        SolidityDataType::Bytes(b) => {
            res.extend(*b);
        }
        SolidityDataType::Bool(b) => {
            if *b {
                res.push(1);
            } else {
                res.push(0);
            }
        }
        SolidityDataType::NumberWithShift(n, to_take) => {
            let local_res = n.0.iter().rev().fold(vec![], |mut acc, i| {
                let bytes = i.to_be_bytes();
                acc.extend(bytes);
                acc
            });

            let to_skip = local_res.len() - (to_take.0 / 8);
            let local_res = local_res.into_iter().skip(to_skip).collect::<Vec<u8>>();
            res.extend(local_res);
        }
    };
    return res;
}

pub fn encode_packed(items: &[SolidityDataType]) -> (Vec<u8>, String) {
    let res = items.iter().fold(Vec::new(), |mut acc, i| {
        let pack = pack(i);
        acc.push(pack);
        acc
    });
    let res = res.join(&[][..]);
    let hexed = hex::encode(&res);
    (res, hexed)
}

pub fn get_request_packet(handler_address: String, payload: Binary) -> Binary {
    let handler_token = Token::String(handler_address);
    let payload_token = Token::Bytes(payload.as_slice().to_vec());
    let enc = encode(&vec![handler_token, payload_token]);
    Binary::from(enc)
}

pub fn get_request_metadata(
    gas_limit: u64,
    gas_price: u64,
    ack_gas_limit: u64,
    ack_gas_price: u64,
    relayer_fees: u128,
    ack_type: u8,
    is_read_call: bool,
    asm_address: String,
) -> Binary {
    let input = vec![
        SolidityDataType::NumberWithShift(U256::from(gas_limit), TakeLastXBytes(64)),
        SolidityDataType::NumberWithShift(U256::from(gas_price), TakeLastXBytes(64)),
        SolidityDataType::NumberWithShift(U256::from(ack_gas_limit), TakeLastXBytes(64)),
        SolidityDataType::NumberWithShift(U256::from(ack_gas_price), TakeLastXBytes(64)),
        SolidityDataType::NumberWithShift(U256::from(relayer_fees), TakeLastXBytes(128)),
        SolidityDataType::NumberWithShift(U256::from(ack_type), TakeLastXBytes(8)),
        SolidityDataType::Bool(is_read_call),
        SolidityDataType::String(asm_address.as_str())
    ];
    let (enc, _ )= encode_packed(&input);
    let enc_bin = Binary::from(enc);
    enc_bin
}

pub fn abi_decode_to_binary(enc: &Binary) -> Result<Binary, ContractError> {
    let param_types = vec![ParamType::Bytes];
    let payload = decode(&param_types, enc.as_slice()).or_else(|_| {
        Err(ContractError::Std(StdError::generic_err("error: abi_decode_to_binary")))
    })?;
    let payload_byte = match payload[0].clone() {
        Token::Bytes(payload) => payload,
        _ => vec![],
    };
    Ok(Binary::from(payload_byte))
}

pub fn abi_encode_string(stri: &String) -> Binary {
    let stri_token = Token::String(stri.clone());
    let enc = encode(&vec![stri_token]);
    return Binary::from(enc);
}

pub fn assert_sent_sufficient_coin(
    sent: &[Coin],
    required: Option<Coin>,
) -> Result<(), ContractError> {
    if let Some(required_coin) = required {
        let required_amount = required_coin.amount.u128();
        if required_amount > 0 {
            let sent_sufficient_funds = sent.iter().any(|coin| {
                // check if a given sent coin matches denom
                // and has sufficient amount
                coin.denom == required_coin.denom && coin.amount.u128() >= required_amount
            });

            if sent_sufficient_funds {
                return Ok(());
            } else {
                return Err(ContractError::InsufficientFundsSend {});
            }
        }
    }
    Ok(())
}

// let's not import a regexp library and just do these checks by hand
fn invalid_char(c: char) -> bool {
    let is_valid =
        c.is_ascii_digit() || c.is_ascii_lowercase() || (c == '.' || c == '-' || c == '_');
    !is_valid
}

/// validate_name returns an error if the name is invalid
/// (we require 3-64 lowercase ascii letters, numbers, or . - _)
pub fn validate_name(name: &str) -> Result<(), ContractError> {
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


#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::{coin, coins};

    #[test]
    fn assert_sent_sufficient_coin_works() {
        match assert_sent_sufficient_coin(&[], Some(coin(0, "token"))) {
            Ok(()) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        match assert_sent_sufficient_coin(&[], Some(coin(5, "token"))) {
            Ok(()) => panic!("Should have raised insufficient funds error"),
            Err(ContractError::InsufficientFundsSend {}) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        match assert_sent_sufficient_coin(&coins(10, "smokin"), Some(coin(5, "token"))) {
            Ok(()) => panic!("Should have raised insufficient funds error"),
            Err(ContractError::InsufficientFundsSend {}) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        match assert_sent_sufficient_coin(&coins(10, "token"), Some(coin(5, "token"))) {
            Ok(()) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        let sent_coins = vec![coin(2, "smokin"), coin(5, "token"), coin(1, "earth")];
        match assert_sent_sufficient_coin(&sent_coins, Some(coin(5, "token"))) {
            Ok(()) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        };
    }
}

#[test]
fn encode_string1() {
    let stri = "{\"resolve_record\": {\"name\": \"test5\"}}".to_string();
    let enc = abi_encode_string(&stri);
    print!("enc: {:?}", enc);
}

#[test]
fn decode_to_binary() {
    let bin = Binary::from_base64("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAJXsicmVzb2x2ZV9yZWNvcmQiOiB7Im5hbWUiOiAidGVzdDUifX0AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap();
    let dec = abi_decode_to_binary(&bin).unwrap();
    print!("{:?}", dec);
}

#[test]
fn get_metadata() {
    let metadata = get_request_metadata(0, 0, 0, 0, 0, 3, false, "".to_string());
    print!("{:?}", metadata);
}