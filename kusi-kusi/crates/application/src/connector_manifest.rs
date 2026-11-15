//! Versioned declarative connector contract; contains no credential values or executable code.
use crate::{connection_settings::ConnectionKind, local::LocalError};
use serde::Serialize;
use serde_json::{Value, json};
#[derive(Clone, Serialize)]
pub struct AuthField {
    pub name: &'static str,
    pub label: &'static str,
    pub secret: bool,
    pub required: bool,
}
#[derive(Clone, Serialize)]
pub struct ConnectorManifest {
    pub kind: ConnectionKind,
    pub name: &'static str,
    pub version: &'static str,
    pub auth_fields: Vec<AuthField>,
    pub operations: Vec<&'static str>,
    pub input_schema: Value,
    pub output_schema: Value,
    pub required_scopes: Vec<&'static str>,
    pub verification_probe: &'static str,
    pub timeout_ms: u32,
    pub max_concurrent_probes: u32,
}
pub fn manifests() -> Vec<ConnectorManifest> {
    [ConnectionKind::Plane,ConnectionKind::Frappe,ConnectionKind::Telegram].into_iter().map(|kind|{
 let (name,fields,op,probe)=match kind{
 ConnectionKind::Plane=>("Plane",vec![("api_key","API Key")],"plane.resource.read","plane.v2.current_user"),
 ConnectionKind::Frappe=>("ERP CONEIMERA",vec![("api_key","API Key"),("api_secret","API Secret")],"frappe.resource.read","frappe.v1.logged_user"),
 ConnectionKind::Telegram=>("Telegram",vec![("bot_token","Token de Telegram"),("webhook_secret","Secreto de webhook")],"telegram.context.read","telegram.bot_api.getMe")};
 ConnectorManifest{kind,name,version:"1",auth_fields:fields.into_iter().map(|(name,label)|AuthField{name,label,secret:true,required:true}).collect(),operations:vec![op],input_schema:json!({"type":"object","required":["resource_ref"],"additionalProperties":false,"properties":{"resource_ref":{"type":"string","minLength":1,"maxLength":255}}}),output_schema:json!({"type":"object","required":["resource_ref","evidence"],"additionalProperties":false,"properties":{"resource_ref":{"type":"string"},"evidence":{"type":"array","items":{"type":"string"}}}}),required_scopes:vec!["identity_check_is_not_resource_authorization"],verification_probe:probe,timeout_ms:10000,max_concurrent_probes:1}
 }).collect()
}
pub fn validate_operation(
    kind: ConnectionKind,
    version: &str,
    operation: &str,
    input: &Value,
) -> Result<(), LocalError> {
    let manifest = manifests()
        .into_iter()
        .find(|m| m.kind == kind)
        .ok_or(LocalError::Invalid)?;
    if manifest.version != version || !manifest.operations.contains(&operation) {
        return Err(LocalError::Invalid);
    }
    let obj = input.as_object().ok_or(LocalError::Invalid)?;
    if obj.len() != 1
        || !obj
            .get("resource_ref")
            .and_then(Value::as_str)
            .is_some_and(kusi_domain::external_access::valid_reference)
    {
        return Err(LocalError::Invalid);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_unknown_versions_operations_and_arguments() {
        assert!(
            validate_operation(
                ConnectionKind::Plane,
                "2",
                "plane.resource.read",
                &json!({"resource_ref":"a"})
            )
            .is_err()
        );
        assert!(
            validate_operation(
                ConnectionKind::Plane,
                "1",
                "plane.write",
                &json!({"resource_ref":"a"})
            )
            .is_err()
        );
        assert!(
            validate_operation(
                ConnectionKind::Plane,
                "1",
                "plane.resource.read",
                &json!({"resource_ref":"a","sql":"x"})
            )
            .is_err()
        );
        assert!(
            validate_operation(
                ConnectionKind::Plane,
                "1",
                "plane.resource.read",
                &json!({"resource_ref":"a"})
            )
            .is_ok()
        );
    }
}
