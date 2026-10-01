use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

const MAX_INTEROPERABLE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionFingerprint([u8; 32]);

impl ActionFingerprint {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FingerprintError {
    Canonicalization(String),
    NonInteroperableInteger(String),
    UnicodeNoncharacter { codepoint: u32 },
}

impl Display for FingerprintError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonicalization(message) => {
                write!(formatter, "JCS canonicalization failed: {message}")
            }
            Self::NonInteroperableInteger(value) => {
                write!(
                    formatter,
                    "integer {value} is outside CER's exact I-JSON interoperability range"
                )
            }
            Self::UnicodeNoncharacter { codepoint } => {
                write!(
                    formatter,
                    "Unicode noncharacter U+{codepoint:04X} is not permitted by CER's JCS input profile"
                )
            }
        }
    }
}

impl Error for FingerprintError {}

pub(crate) fn fingerprint<T: Serialize>(value: &T) -> Result<ActionFingerprint, FingerprintError> {
    let canonical = canonical_bytes(value)?;
    let digest = Sha256::digest(canonical);
    let mut bytes = [0_u8; 32];
    bytes.copy_from_slice(&digest);
    Ok(ActionFingerprint(bytes))
}

pub(crate) fn validate_jcs_value(value: &Value) -> Result<(), FingerprintError> {
    match value {
        Value::Null | Value::Bool(_) => Ok(()),
        Value::Number(number) => validate_number(number),
        Value::String(string) => validate_string(string),
        Value::Array(values) => {
            for value in values {
                validate_jcs_value(value)?;
            }
            Ok(())
        }
        Value::Object(entries) => {
            for (key, value) in entries {
                validate_string(key)?;
                validate_jcs_value(value)?;
            }
            Ok(())
        }
    }
}

fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, FingerprintError> {
    serde_json_canonicalizer::to_vec(value)
        .map_err(|error| FingerprintError::Canonicalization(error.to_string()))
}

fn validate_number(number: &serde_json::Number) -> Result<(), FingerprintError> {
    if let Some(integer) = number.as_i64() {
        if !(-MAX_INTEROPERABLE_INTEGER..=MAX_INTEROPERABLE_INTEGER).contains(&integer) {
            return Err(FingerprintError::NonInteroperableInteger(
                number.to_string(),
            ));
        }
        return Ok(());
    }

    if let Some(integer) = number.as_u64() {
        if integer > MAX_INTEROPERABLE_INTEGER as u64 {
            return Err(FingerprintError::NonInteroperableInteger(
                number.to_string(),
            ));
        }
    }

    Ok(())
}

fn validate_string(value: &str) -> Result<(), FingerprintError> {
    for character in value.chars() {
        let codepoint = character as u32;
        if is_unicode_noncharacter(codepoint) {
            return Err(FingerprintError::UnicodeNoncharacter { codepoint });
        }
    }
    Ok(())
}

fn is_unicode_noncharacter(codepoint: u32) -> bool {
    (0xFDD0..=0xFDEF).contains(&codepoint)
        || (codepoint <= 0x10FFFF && matches!(codepoint & 0xFFFF, 0xFFFE | 0xFFFF))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn jcs_matches_rfc_8785_property_sorting_vector() {
        let source = r#"{
            "\u20ac": "Euro Sign",
            "\r": "Carriage Return",
            "\ufb33": "Hebrew Letter Dalet With Dagesh",
            "1": "One",
            "\ud83d\ude00": "Emoji: Grinning Face",
            "\u0080": "Control",
            "\u00f6": "Latin Small Letter O With Diaeresis"
        }"#;
        let value: Value = serde_json::from_str(source).unwrap();

        let canonical = String::from_utf8(canonical_bytes(&value).unwrap()).unwrap();
        let expected = "{\"\\r\":\"Carriage Return\",\"1\":\"One\",\"\":\"Control\",\"ö\":\"Latin Small Letter O With Diaeresis\",\"€\":\"Euro Sign\",\"😀\":\"Emoji: Grinning Face\",\"דּ\":\"Hebrew Letter Dalet With Dagesh\"}";

        assert_eq!(canonical, expected);
    }

    #[test]
    fn rejects_integer_outside_exact_interoperability_range() {
        let value = json!({"unsafe_integer": 9_007_199_254_740_992_u64});

        assert_eq!(
            validate_jcs_value(&value),
            Err(FingerprintError::NonInteroperableInteger(
                "9007199254740992".into()
            ))
        );
    }

    #[test]
    fn accepts_largest_exact_interoperable_integer() {
        let value = json!({"safe_integer": 9_007_199_254_740_991_u64});

        assert!(validate_jcs_value(&value).is_ok());
    }

    #[test]
    fn rejects_unicode_noncharacter_in_value() {
        let value = json!({"name": "\u{FDD0}"});

        assert_eq!(
            validate_jcs_value(&value),
            Err(FingerprintError::UnicodeNoncharacter { codepoint: 0xFDD0 })
        );
    }

    #[test]
    fn rejects_unicode_noncharacter_in_object_key() {
        let value = json!({"\u{FFFF}": "invalid key"});

        assert_eq!(
            validate_jcs_value(&value),
            Err(FingerprintError::UnicodeNoncharacter { codepoint: 0xFFFF })
        );
    }
}
