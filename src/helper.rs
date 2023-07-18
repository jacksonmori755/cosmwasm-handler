use cosmwasm_std::{Binary, StdError};
pub use ethabi; 
pub use hex; 
use ethabi::{ethereum_types::{Address, U256}, decode, ParamType, Token, encode};

use crate::ContractError;

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
    relayer_fees: u64,
    ack_type: u8,
    is_read_call: bool,
    asm_address: String,
) -> Binary {
    let input = vec![
        SolidityDataType::NumberWithShift(U256::from(gas_limit), TakeLastXBytes(64)),
        SolidityDataType::NumberWithShift(U256::from(gas_price), TakeLastXBytes(64)),
        SolidityDataType::NumberWithShift(U256::from(ack_gas_limit), TakeLastXBytes(64)),
        SolidityDataType::NumberWithShift(U256::from(ack_gas_price), TakeLastXBytes(64)),
        SolidityDataType::NumberWithShift(U256::from(relayer_fees), TakeLastXBytes(64)),
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
    let payload = decode(&param_types, enc.as_slice()).or_else(|e| {
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