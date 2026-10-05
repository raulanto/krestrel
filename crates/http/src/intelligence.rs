//! Heuristics for response intelligence: JWT detection and timestamp formatting.
//! Never modifies original response body; strictly for highlighting.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodedJwt {
    pub header: serde_json::Value,
    pub payload: serde_json::Value,
}

pub fn try_decode_jwt(token: &str) -> Option<DecodedJwt> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let header_bytes = base64_url_decode(parts[0])?;
    let payload_bytes = base64_url_decode(parts[1])?;

    let header: serde_json::Value = serde_json::from_slice(&header_bytes).ok()?;
    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;

    Some(DecodedJwt { header, payload })
}

fn base64_url_decode(input: &str) -> Option<Vec<u8>> {
    let mut s = input.replace('-', "+").replace('_', "/");
    match s.len() % 4 {
        2 => s.push_str("=="),
        3 => s.push('='),
        _ => {}
    }
    // Standard decoding using a simple lookup table or standard lib if available
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut map = [255u8; 256];
    for (i, &b) in alphabet.iter().enumerate() {
        map[b as usize] = i as u8;
    }

    let mut buf = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            break;
        }
        let b0 = map[bytes[i] as usize];
        let b1 = map[bytes.get(i + 1).copied().unwrap_or(b'=') as usize];
        if b0 == 255 || b1 == 255 {
            return None;
        }
        buf.push((b0 << 2) | (b1 >> 4));

        if i + 2 < bytes.len() && bytes[i + 2] != b'=' {
            let b2 = map[bytes[i + 2] as usize];
            if b2 == 255 {
                return None;
            }
            buf.push((b1 << 4) | (b2 >> 2));

            if i + 3 < bytes.len() && bytes[i + 3] != b'=' {
                let b3 = map[bytes[i + 3] as usize];
                if b3 == 255 {
                    return None;
                }
                buf.push((b2 << 6) | b3);
            }
        }
        i += 4;
    }
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_detection() {
        // Sample valid JWT header: {"alg":"HS256","typ":"JWT"} -> eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9
        // Payload: {"sub":"1234567890","name":"John Doe","iat":1516239022} -> eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.signature";
        let decoded = try_decode_jwt(jwt).expect("Should decode jwt");
        assert_eq!(decoded.header["alg"], "HS256");
        assert_eq!(decoded.payload["name"], "John Doe");
    }
}
