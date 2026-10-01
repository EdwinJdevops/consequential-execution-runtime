use crate::identity::{validate_jcs_value, FingerprintError};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::Serialize;
use serde_json::{Map, Number, Value};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ActionArguments(Value);

impl ActionArguments {
    pub fn from_serializable<T: Serialize>(value: &T) -> Result<Self, ArgumentError> {
        let value =
            serde_json::to_value(value).map_err(|error| ArgumentError::Json(error.to_string()))?;
        Self::from_value(value)
    }

    pub fn from_json_str(input: &str) -> Result<Self, ArgumentError> {
        Self::from_json_slice(input.as_bytes())
    }

    pub fn from_json_slice(input: &[u8]) -> Result<Self, ArgumentError> {
        let mut deserializer = serde_json::Deserializer::from_slice(input);
        let StrictValue(value) = StrictValue::deserialize(&mut deserializer)
            .map_err(|error| ArgumentError::Json(error.to_string()))?;
        deserializer
            .end()
            .map_err(|error| ArgumentError::Json(error.to_string()))?;
        Self::from_value(value)
    }

    pub fn as_value(&self) -> &Value {
        &self.0
    }

    fn from_value(value: Value) -> Result<Self, ArgumentError> {
        validate_jcs_value(&value).map_err(ArgumentError::Profile)?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgumentError {
    Json(String),
    Profile(FingerprintError),
}

impl Display for ArgumentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message) => write!(formatter, "invalid action arguments JSON: {message}"),
            Self::Profile(error) => write!(formatter, "invalid action arguments profile: {error}"),
        }
    }
}

impl Error for ArgumentError {}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value with unique object member names")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(StrictValue)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        StrictValue::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        while let Some(StrictValue(value)) = sequence.next_element::<StrictValue>()? {
            values.push(value);
        }
        Ok(StrictValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = Map::new();

        while let Some(key) = object.next_key::<String>()? {
            if entries.contains_key(&key) {
                return Err(de::Error::custom(format!(
                    "duplicate object member name: {key}"
                )));
            }

            let StrictValue(value) = object.next_value::<StrictValue>()?;
            entries.insert(key, value);
        }

        Ok(StrictValue(Value::Object(entries)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_duplicate_object_member_names() {
        let error = ActionArguments::from_json_str(r#"{"role":"prod","role":"dev"}"#).unwrap_err();

        assert!(matches!(
            error,
            ArgumentError::Json(message)
                if message.contains("duplicate object member name: role")
        ));
    }

    #[test]
    fn rejects_duplicate_names_after_json_unescaping() {
        let error =
            ActionArguments::from_json_str(r#"{"role":"prod","\u0072ole":"dev"}"#).unwrap_err();

        assert!(matches!(
            error,
            ArgumentError::Json(message)
                if message.contains("duplicate object member name: role")
        ));
    }

    #[test]
    fn rejects_nested_duplicate_object_member_names() {
        let error =
            ActionArguments::from_json_str(r#"{"outer":{"target":1,"target":2}}"#).unwrap_err();

        assert!(matches!(
            error,
            ArgumentError::Json(message)
                if message.contains("duplicate object member name: target")
        ));
    }

    #[test]
    fn rejects_trailing_json_value() {
        let error = ActionArguments::from_json_str(r#"{"a":1} {"b":2}"#).unwrap_err();

        assert!(matches!(error, ArgumentError::Json(_)));
    }

    #[test]
    fn raw_ingress_enforces_numeric_profile() {
        let error =
            ActionArguments::from_json_str(r#"{"id":9007199254740992}"#).unwrap_err();

        assert_eq!(
            error,
            ArgumentError::Profile(FingerprintError::NonInteroperableInteger(
                "9007199254740992".into()
            ))
        );
    }

    #[test]
    fn serializable_input_enforces_same_profile() {
        let error =
            ActionArguments::from_serializable(&json!({"id": 9_007_199_254_740_992_u64}))
                .unwrap_err();

        assert!(matches!(
            error,
            ArgumentError::Profile(FingerprintError::NonInteroperableInteger(_))
        ));
    }

    #[test]
    fn valid_raw_json_is_retained_as_structured_value() {
        let arguments =
            ActionArguments::from_json_str(r#"{"count":2,"options":{"mode":"safe"}}"#).unwrap();

        assert_eq!(
            arguments.as_value(),
            &json!({"count": 2, "options": {"mode": "safe"}})
        );
    }
}
