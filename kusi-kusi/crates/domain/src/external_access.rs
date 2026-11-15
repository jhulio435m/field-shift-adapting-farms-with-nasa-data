//! Exact resource contract shared by independent system adapters.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRead {
    pub resource_ref: String,
}

impl ResourceRead {
    pub fn is_valid(&self) -> bool {
        valid_reference(&self.resource_ref)
    }
}

pub fn valid_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

impl ResourceRead {
    pub fn accepts(&self, output: &serde_json::Value) -> bool {
        output.as_object().is_some_and(|object| {
            object.len() == 2
                && object
                    .get("resource_ref")
                    .and_then(serde_json::Value::as_str)
                    == Some(self.resource_ref.as_str())
                && object.contains_key("data")
        })
    }
}
