pub mod adaptive;
pub mod admin_identity;
pub mod bureaucratic_case;
pub mod creative;
pub mod external_access;
pub mod local;
pub mod student_payment;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OrganizationError {
    #[error("name must contain between 2 and 120 characters")]
    InvalidName,
    #[error("slug must contain 2 to 63 lowercase letters, digits, or hyphens")]
    InvalidSlug,
}

impl Organization {
    pub fn new(name: String, slug: String) -> Result<Self, OrganizationError> {
        let name = name.trim().to_owned();
        if !(2..=120).contains(&name.chars().count()) {
            return Err(OrganizationError::InvalidName);
        }
        let valid_slug = (2..=63).contains(&slug.len())
            && slug.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            && slug
                .as_bytes()
                .last()
                .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            && slug
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-');
        if !valid_slug {
            return Err(OrganizationError::InvalidSlug);
        }
        Ok(Self {
            id: Uuid::new_v4(),
            name,
            slug,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_organization_fields() {
        assert!(Organization::new("Instituto Demo".into(), "instituto-demo".into()).is_ok());
        assert_eq!(
            Organization::new("x".into(), "instituto".into()),
            Err(OrganizationError::InvalidName)
        );
        assert_eq!(
            Organization::new("Instituto".into(), "Bad Slug".into()),
            Err(OrganizationError::InvalidSlug)
        );
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    #[default]
    Read,
}

/// Organization-owned operation contract, independent of its transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    #[serde(default)]
    pub operation: Operation,
    pub identifier: String,
    pub version: String,
    pub description: String,
    pub adapter: String,
    pub required_permission: String,
    pub enabled: bool,
    pub timeout_ms: u32,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
}

impl Capability {
    pub fn is_valid(&self) -> bool {
        !self.identifier.is_empty()
            && self.identifier.len() <= 120
            && self
                .identifier
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'.' || c == b'_')
            && !self.version.is_empty()
            && self.version.len() <= 32
            && !self.adapter.is_empty()
            && self.adapter.len() <= 120
            && !self.required_permission.is_empty()
            && self.required_permission.len() <= 120
            && self.description.len() <= 2000
            && (100..=30000).contains(&self.timeout_ms)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEvent {
    pub event_id: String,
    pub sender: String,
    pub kind: String,
    pub content: String,
}

pub mod managed_integrations;
