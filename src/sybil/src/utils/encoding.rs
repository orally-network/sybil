use ic_web3_rs::{
    ethabi::{ParamType, Token},
    types::U256,
};

use anyhow::{anyhow, Result};
use thiserror::Error;

use super::{address, canister::CanisterError, web3::Web3Error};

pub fn encode_packed(tokens: &[Token]) -> Result<Vec<u8>> {
    let mut max = 0;
    for token in tokens {
        check(token)?;
        max += max_encoded_length(token);
    }

    let mut bytes = Vec::with_capacity(max);
    for token in tokens {
        encode_token(token, &mut bytes, false);
    }
    Ok(bytes)
}

fn max_encoded_length(token: &Token) -> usize {
    match token {
        Token::Int(_) | Token::Uint(_) | Token::FixedBytes(_) => 32,
        Token::Address(_) => 20,
        Token::Bool(_) => 1,
        Token::Array(vec) | Token::FixedArray(vec) | Token::Tuple(vec) => vec
            .iter()
            .map(|token| max_encoded_length(token).max(32))
            .sum(),
        Token::Bytes(b) => b.len(),
        Token::String(s) => s.len(),
    }
}

fn check(token: &Token) -> Result<()> {
    match token {
        Token::FixedBytes(vec) if vec.len() > 32 => Err(anyhow!("Invalid token: {:?}", token)),

        Token::Tuple(_) => Err(anyhow!("Invalid token: {:?}", token)),
        Token::Array(vec) | Token::FixedArray(vec) => {
            for t in vec.iter() {
                if t.is_dynamic() || matches!(t, Token::Array(_)) {
                    return Err(anyhow!("Invalid token: {:?}", token));
                }
                check(t)?;
            }
            Ok(())
        }

        _ => Ok(()),
    }
}

fn encode_token(token: &Token, out: &mut Vec<u8>, in_array: bool) {
    match token {
        Token::Address(addr) => {
            if in_array {
                out.extend_from_slice(&[0; 12]);
            }
            out.extend_from_slice(&addr.0)
        }
        Token::Int(n) | Token::Uint(n) => {
            let mut buf = [0; 32];
            n.to_big_endian(&mut buf);
            out.extend_from_slice(&buf);
        }
        Token::Bool(b) => {
            if in_array {
                out.extend_from_slice(&[0; 31]);
            }
            out.push((*b) as u8);
        }
        Token::FixedBytes(bytes) => {
            out.extend_from_slice(bytes);
            if in_array {
                let mut remaining = vec![0; 32 - bytes.len()];
                out.append(&mut remaining);
            }
        }

        Token::Bytes(bytes) => out.extend_from_slice(bytes),
        Token::String(s) => out.extend_from_slice(s.as_bytes()),
        Token::Array(vec) | Token::FixedArray(vec) => {
            for token in vec {
                encode_token(token, out, true);
            }
        }

        token => ic_cdk::trap(&format!("Uncaught invalid token: {token:?}")),
    }
}

#[derive(Error, Debug)]
pub enum ParseTokensError {
    #[error("Invalid number of params")]
    InvalidNumberOfParams,
    #[error("Address Error: {0}")]
    AddressError(#[from] address::AddressError),
    #[error("Web3 error: {0}")]
    Web3Error(#[from] Web3Error),
    #[error("Canister Error: {0}")]
    CanisterError(#[from] CanisterError),
}

pub fn parse_tokens(inputs: &[ParamType], params: String) -> Result<Vec<Token>, ParseTokensError> {
    let mut tokens = vec![];
    let mut remaining_params = params.clone();

    for input in inputs {
        if remaining_params.is_empty() {
            return Err(ParseTokensError::InvalidNumberOfParams);
        }

        let param = remaining_params
            .split_once([','])
            .map(|(a, b)| (a.to_owned(), b.to_owned()));

        let trim_param = if let Some((param, other_params)) = param {
            remaining_params = other_params.trim().to_string();
            param.trim().to_owned()
        } else {
            remaining_params.clone()
        };

        match input {
            ParamType::Uint(_) => {
                tokens.push(Token::Uint(U256::from_dec_str(&trim_param).unwrap()));
            }
            ParamType::Int(_) => {
                tokens.push(Token::Int(trim_param.parse::<U256>().unwrap()));
            }
            ParamType::Address => {
                tokens.push(Token::Address(address::to_h160(&trim_param).unwrap()));
            }
            ParamType::Bool => {
                tokens.push(Token::Bool(trim_param.parse::<bool>().unwrap()));
            }
            ParamType::String => {
                tokens.push(Token::String(trim_param.to_string()));
            }
            ParamType::Bytes => {
                let bytes = hex::decode(address::trim_prefix(&trim_param)).unwrap();
                tokens.push(Token::Bytes(bytes));
            }
            ParamType::FixedBytes(size) => {
                let bytes = hex::decode(address::trim_prefix(&trim_param)).unwrap();
                assert_eq!(bytes.len(), *size);
                tokens.push(Token::FixedBytes(bytes));
            }
            ParamType::Array(ref inner_param_type) => {
                if trim_param.starts_with("[") {
                    let array_str = if let Some((array, other_params)) =
                        remaining_params.clone().split_once("]")
                    {
                        remaining_params = other_params
                            .split_once(',')
                            .unwrap_or(("", ""))
                            .1
                            .to_string();

                        let array = format!("{}, {}", &trim_param[1..trim_param.len()], array);

                        array.trim().to_owned()
                    } else {
                        panic!();
                    };

                    let words = array_str.chars().filter(|c| *c == ',').count() + 1;
                    let inner_param_types = (0..words)
                        .map(|_| *inner_param_type.clone())
                        .collect::<Vec<_>>();

                    let array_tokens = parse_tokens(&inner_param_types, array_str.to_string())?;
                    tokens.push(Token::Array(array_tokens));
                } else {
                    panic!();
                }
            }
            ParamType::FixedArray(ref inner_param_type, size) => {
                if trim_param.starts_with("[") {
                    let array_str = if let Some((array, other_params)) =
                        remaining_params.clone().split_once("]")
                    {
                        remaining_params = other_params
                            .split_once(',')
                            .unwrap_or(("", ""))
                            .1
                            .to_string();

                        let array = format!("{}, {}", &trim_param[1..trim_param.len()], array);

                        array.trim().to_owned()
                    } else {
                        panic!();
                    };

                    let inner_param_types = (0..*size)
                        .map(|_| *inner_param_type.clone())
                        .collect::<Vec<_>>();

                    let array_tokens = parse_tokens(&inner_param_types, array_str.to_string())?;
                    tokens.push(Token::Array(array_tokens));
                } else {
                    panic!();
                }
            }
            ParamType::Tuple(ref inner_param_types) => {
                if trim_param.starts_with("(") {
                    let array_str = if let Some((array, other_params)) =
                        remaining_params.clone().split_once(")")
                    {
                        remaining_params = other_params
                            .split_once(',')
                            .unwrap_or(("", ""))
                            .1
                            .to_string();

                        let array = format!("{}, {}", &trim_param[1..trim_param.len()], array);

                        array.trim().to_owned()
                    } else {
                        panic!();
                    };

                    let array_tokens = parse_tokens(inner_param_types, array_str.to_string())?;
                    tokens.push(Token::Array(array_tokens));
                } else {
                    panic!();
                }
            }
        };
    }

    Ok(tokens)
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn test_parse_tokens() {
        let str = r###"(aboba, 123, 1010, true, aboba, 1010)"###;
        let token_params = vec![
            ParamType::String,
            ParamType::Uint(256),
            ParamType::Bytes,
            ParamType::Bool,
            ParamType::String,
            ParamType::Bytes,
        ];
        let expected_tokens = vec![
            Token::String("aboba".to_string()),
            Token::Uint(U256::from(123)),
            Token::Bytes(vec![0x10, 0x10]),
            Token::Bool(true),
            Token::String("aboba".to_string()),
            Token::Bytes(vec![0x10, 0x10]),
        ];
        let fact_tokens = parse_tokens(&token_params, str[1..str.len() - 1].to_string()).unwrap();

        assert_eq!(expected_tokens, fact_tokens);

        let str = r###"(aboba, 123, 1234, true, aboba, aabb, [aboba1, aboba2])"###;
        let token_params = vec![
            ParamType::String,
            ParamType::Uint(256),
            ParamType::Bytes,
            ParamType::Bool,
            ParamType::String,
            ParamType::Bytes,
            ParamType::Array(Box::new(ParamType::String)),
        ];
        let expected_tokens = vec![
            Token::String("aboba".to_string()),
            Token::Uint(U256::from(123)),
            Token::Bytes(vec![0x12, 0x34]),
            Token::Bool(true),
            Token::String("aboba".to_string()),
            Token::Bytes(vec![0xaa, 0xbb]),
            Token::Array(vec![
                Token::String("aboba1".to_string()),
                Token::String("aboba2".to_string()),
            ]),
        ];
        let fact_tokens = parse_tokens(&token_params, str[1..str.len() - 1].to_string()).unwrap();

        assert_eq!(expected_tokens, fact_tokens);

        let str = r###"([aboba1, aboba2], [123, 456], [1010, aabb, bbaa])"###;
        let token_params = vec![
            ParamType::Array(Box::new(ParamType::String)),
            ParamType::Array(Box::new(ParamType::Uint(256))),
            ParamType::Array(Box::new(ParamType::Bytes)),
        ];
        let expected_tokens = vec![
            Token::Array(vec![
                Token::String("aboba1".to_string()),
                Token::String("aboba2".to_string()),
            ]),
            Token::Array(vec![
                Token::Uint(U256::from(123)),
                Token::Uint(U256::from(456)),
            ]),
            Token::Array(vec![
                Token::Bytes(vec![0x10, 0x10]),
                Token::Bytes(vec![0xaa, 0xbb]),
                Token::Bytes(vec![0xbb, 0xaa]),
            ]),
        ];

        let fact_tokens = parse_tokens(&token_params, str[1..str.len() - 1].to_string()).unwrap();

        assert_eq!(expected_tokens, fact_tokens);

        let str = r###"(654DFF41D51c230FA400205A633101C5C1f1969C, 123, [aboba1, aboba2], [aabb, bbaa, abab], (string, 123, aabb, [array_string1, array_string2]))"###;

        let token_params = vec![
            ParamType::Address,
            ParamType::Uint(256),
            ParamType::Array(Box::new(ParamType::String)),
            ParamType::Array(Box::new(ParamType::Bytes)),
            ParamType::Tuple(vec![
                ParamType::String,
                ParamType::Uint(256),
                ParamType::Bytes,
                ParamType::Array(Box::new(ParamType::String)),
            ]),
        ];

        let expected_tokens = vec![
            Token::Address(address::to_h160("654DFF41D51c230FA400205A633101C5C1f1969C").unwrap()),
            Token::Uint(U256::from(123)),
            Token::Array(vec![
                Token::String("aboba1".to_string()),
                Token::String("aboba2".to_string()),
            ]),
            Token::Array(vec![
                Token::Bytes(vec![0xaa, 0xbb]),
                Token::Bytes(vec![0xbb, 0xaa]),
                Token::Bytes(vec![0xab, 0xab]),
            ]),
            Token::Array(vec![
                Token::String("string".to_string()),
                Token::Uint(U256::from(123)),
                Token::Bytes(vec![0xaa, 0xbb]),
                Token::Array(vec![
                    Token::String("array_string1".to_string()),
                    Token::String("array_string2".to_string()),
                ]),
            ]),
        ];

        let fact_tokens = parse_tokens(&token_params, str[1..str.len() - 1].to_string()).unwrap();

        assert_eq!(expected_tokens, fact_tokens);

        let str = r###"(654DFF41D51c230FA400205A633101C5C1f1969C)"###;

        let token_params = vec![ParamType::Address];

        let expected_tokens = vec![Token::Address(
            address::to_h160("654DFF41D51c230FA400205A633101C5C1f1969C").unwrap(),
        )];

        let fact_tokens = parse_tokens(&token_params, str[1..str.len() - 1].to_string()).unwrap();

        assert_eq!(expected_tokens, fact_tokens);
    }
}
