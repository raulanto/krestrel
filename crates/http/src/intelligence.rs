use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JwtStatus {
    Valid,
    Expired,
    NotYetValid,
    NoExpiration,
    AlgNone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JwtInfo {
    pub raw: String,
    pub header: serde_json::Value,
    pub payload: serde_json::Value,
    pub signature_len: usize,
    pub status: JwtStatus,
    pub exp: Option<i64>,
    pub nbf: Option<i64>,
    pub iat: Option<i64>,
    pub is_jwe: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimestampUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampInfo {
    pub raw: String,
    pub timestamp_sec: i64,
    pub unit: TimestampUnit,
    pub formatted_utc: String,
    pub is_iso8601: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightKind {
    Jwt(JwtInfo),
    Timestamp(TimestampInfo),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Insight {
    pub range: Range<usize>,
    pub kind: InsightKind,
    pub confidence: Confidence,
    pub path: Option<String>,
}

pub trait Detector {
    fn detect(&self, text: &str, path_context: Option<&str>) -> Vec<Insight>;
}

pub struct ResponseIntelligence {
    jwt_detector: JwtDetector,
    timestamp_detector: TimestampDetector,
}

impl Default for ResponseIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseIntelligence {
    pub fn new() -> Self {
        Self {
            jwt_detector: JwtDetector,
            timestamp_detector: TimestampDetector,
        }
    }

    pub fn analyze(&self, text: &str) -> Vec<Insight> {
        let mut insights = Vec::new();
        insights.extend(self.jwt_detector.detect(text, None));
        insights.extend(self.timestamp_detector.detect(text, None));

        // Limit to 500 findings for performance on large bodies
        if insights.len() > 500 {
            insights.truncate(500);
        }

        insights
    }
}

pub struct JwtDetector;

impl Detector for JwtDetector {
    fn detect(&self, text: &str, path_context: Option<&str>) -> Vec<Insight> {
        let mut insights = Vec::new();

        // Search for potential JWTs (eyJ...)
        for (start_idx, _) in text.match_indices("eyJ") {
            let candidate_slice = &text[start_idx..];
            let end_idx = candidate_slice
                .find(|c: char| {
                    c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '\\' || c == '}'
                })
                .unwrap_or(candidate_slice.len());

            let token = &candidate_slice[..end_idx];
            let parts: Vec<&str> = token.split('.').collect();

            // Check if JWE (5 parts)
            if parts.len() == 5 {
                insights.push(Insight {
                    range: start_idx..start_idx + token.len(),
                    kind: InsightKind::Jwt(JwtInfo {
                        raw: token.to_string(),
                        header: serde_json::json!({"type": "JWE"}),
                        payload: serde_json::json!({"encrypted": true}),
                        signature_len: 0,
                        status: JwtStatus::NoExpiration,
                        exp: None,
                        nbf: None,
                        iat: None,
                        is_jwe: true,
                    }),
                    confidence: Confidence::High,
                    path: path_context.map(String::from),
                });
                continue;
            }

            if parts.len() != 3 {
                continue;
            }

            let Some(header_bytes) = base64_url_decode(parts[0]) else {
                continue;
            };
            let Some(payload_bytes) = base64_url_decode(parts[1]) else {
                continue;
            };

            let Ok(header) = serde_json::from_slice::<serde_json::Value>(&header_bytes) else {
                continue;
            };
            let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&payload_bytes) else {
                continue;
            };

            if !header.is_object() || !payload.is_object() {
                continue;
            }

            let exp = payload.get("exp").and_then(|v| v.as_i64());
            let nbf = payload.get("nbf").and_then(|v| v.as_i64());
            let iat = payload.get("iat").and_then(|v| v.as_i64());

            let now = Utc::now().timestamp();
            let alg = header.get("alg").and_then(|v| v.as_str()).unwrap_or("");

            let status = if alg.eq_ignore_ascii_case("none") {
                JwtStatus::AlgNone
            } else if let Some(exp_ts) = exp {
                if now > exp_ts {
                    JwtStatus::Expired
                } else if let Some(nbf_ts) = nbf {
                    if now < nbf_ts {
                        JwtStatus::NotYetValid
                    } else {
                        JwtStatus::Valid
                    }
                } else {
                    JwtStatus::Valid
                }
            } else {
                JwtStatus::NoExpiration
            };

            insights.push(Insight {
                range: start_idx..start_idx + token.len(),
                kind: InsightKind::Jwt(JwtInfo {
                    raw: token.to_string(),
                    header,
                    payload,
                    signature_len: parts[2].len(),
                    status,
                    exp,
                    nbf,
                    iat,
                    is_jwe: false,
                }),
                confidence: Confidence::High,
                path: path_context.map(String::from),
            });
        }

        insights
    }
}

pub struct TimestampDetector;

impl Detector for TimestampDetector {
    fn detect(&self, text: &str, path_context: Option<&str>) -> Vec<Insight> {
        let mut insights = Vec::new();

        // 1. ISO 8601 detection (e.g. 2025-01-01T00:00:00Z)
        let mut search_pos = 0;
        while search_pos < text.len() {
            let slice = &text[search_pos..];
            if let Some(t_idx) = slice.find('T') {
                let abs_t = search_pos + t_idx;
                if abs_t >= 10 && abs_t + 9 <= text.len() {
                    let start = abs_t - 10;
                    let candidate_str = &text[start..];
                    let end_rel = candidate_str
                        .find(|c: char| {
                            c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}'
                        })
                        .unwrap_or(candidate_str.len());
                    let candidate = &candidate_str[..end_rel];

                    if let Ok(dt) = DateTime::parse_from_rfc3339(candidate) {
                        let utc = dt.with_timezone(&Utc);
                        insights.push(Insight {
                            range: start..start + candidate.len(),
                            kind: InsightKind::Timestamp(TimestampInfo {
                                raw: candidate.to_string(),
                                timestamp_sec: utc.timestamp(),
                                unit: TimestampUnit::Seconds,
                                formatted_utc: utc.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                                is_iso8601: true,
                            }),
                            confidence: Confidence::High,
                            path: path_context.map(String::from),
                        });
                        search_pos = start + candidate.len();
                        continue;
                    }
                }
            }
            break;
        }

        // 2. Numeric Unix timestamp detection (seconds: 10 digits [946684800..4102444800], ms: 13 digits)
        let mut in_digits = false;
        let mut digit_start = 0;

        for (i, b) in text.bytes().enumerate() {
            if b.is_ascii_digit() {
                if !in_digits {
                    in_digits = true;
                    digit_start = i;
                }
            } else if in_digits {
                in_digits = false;
                let num_str = &text[digit_start..i];
                if let Some(ts_info) = parse_numeric_timestamp(num_str) {
                    let is_key_suggestive = path_context
                        .map(|p| {
                            let p_lower = p.to_lowercase();
                            p_lower.contains("time")
                                || p_lower.contains("date")
                                || p_lower.contains("exp")
                                || p_lower.contains("created")
                                || p_lower.contains("updated")
                                || p_lower.contains("at")
                        })
                        .unwrap_or(false);

                    let confidence = if is_key_suggestive {
                        Confidence::High
                    } else {
                        Confidence::Medium
                    };

                    insights.push(Insight {
                        range: digit_start..i,
                        kind: InsightKind::Timestamp(ts_info),
                        confidence,
                        path: path_context.map(String::from),
                    });
                }
            }
        }

        if in_digits {
            let num_str = &text[digit_start..text.len()];
            if let Some(ts_info) = parse_numeric_timestamp(num_str) {
                insights.push(Insight {
                    range: digit_start..text.len(),
                    kind: InsightKind::Timestamp(ts_info),
                    confidence: Confidence::Medium,
                    path: path_context.map(String::from),
                });
            }
        }

        insights
    }
}

fn parse_numeric_timestamp(num_str: &str) -> Option<TimestampInfo> {
    let len = num_str.len();
    let val = num_str.parse::<i64>().ok()?;

    // Plausible timestamp range: 2000-01-01 (946684800) to 2100-01-01 (4102444800)
    if len == 10 && (946_684_800..=4_102_444_800).contains(&val) {
        let utc = Utc.timestamp_opt(val, 0).single()?;
        Some(TimestampInfo {
            raw: num_str.to_string(),
            timestamp_sec: val,
            unit: TimestampUnit::Seconds,
            formatted_utc: utc.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            is_iso8601: false,
        })
    } else if len == 13 && (946_684_800_000..=4_102_444_800_000).contains(&val) {
        let sec = val / 1000;
        let nsec = ((val % 1000) * 1_000_000) as u32;
        let utc = Utc.timestamp_opt(sec, nsec).single()?;
        Some(TimestampInfo {
            raw: num_str.to_string(),
            timestamp_sec: sec,
            unit: TimestampUnit::Milliseconds,
            formatted_utc: utc.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            is_iso8601: false,
        })
    } else {
        None
    }
}

fn base64_url_decode(input: &str) -> Option<Vec<u8>> {
    let mut s = input.replace('-', "+").replace('_', "/");
    match s.len() % 4 {
        2 => s.push_str("=="),
        3 => s.push('='),
        _ => {}
    }
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
    fn test_jwt_detection_valid_and_claims() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.signature";
        let detector = JwtDetector;
        let insights = detector.detect(jwt, None);
        assert_eq!(insights.len(), 1);

        if let InsightKind::Jwt(info) = &insights[0].kind {
            assert_eq!(info.header["alg"], "HS256");
            assert_eq!(info.payload["name"], "John Doe");
            assert_eq!(info.iat, Some(1516239022));
            assert!(!info.is_jwe);
        } else {
            panic!("Expected JWT insight");
        }
    }

    #[test]
    fn test_timestamp_numeric_and_iso() {
        let detector = TimestampDetector;
        let text = "Created at 1735689600 and updated 2025-01-01T00:00:00Z";
        let insights = detector.detect(text, None);
        assert!(insights.len() >= 2);

        let sec_ts = insights.iter().find(
            |i| matches!(&i.kind, InsightKind::Timestamp(ts) if ts.timestamp_sec == 1735689600),
        );
        assert!(sec_ts.is_some());
    }
}
