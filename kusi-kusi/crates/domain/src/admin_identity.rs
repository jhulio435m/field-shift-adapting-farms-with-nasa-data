use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdminRole {
    Owner,
    Editor,
    Approver,
    Viewer,
}

impl AdminRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Editor => "editor",
            Self::Approver => "approver",
            Self::Viewer => "viewer",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "owner" => Some(Self::Owner),
            "editor" => Some(Self::Editor),
            "approver" => Some(Self::Approver),
            "viewer" => Some(Self::Viewer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminMembership {
    pub organization_id: Uuid,
    pub organization: String,
    pub role: AdminRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminIdentity {
    pub issuer: String,
    pub subject: String,
    pub memberships: Vec<AdminMembership>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_known_admin_roles() {
        assert_eq!(AdminRole::parse("owner"), Some(AdminRole::Owner));
        assert_eq!(AdminRole::parse("viewer"), Some(AdminRole::Viewer));
        assert_eq!(AdminRole::parse("platform_admin"), None);
        assert_eq!(AdminRole::Owner.as_str(), "owner");
    }
}
