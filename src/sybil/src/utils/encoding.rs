use ic_web3_rs::ethabi::Token;

use anyhow::{anyhow, Result};

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
