use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalRole {
    Owner,
    Editor,
    Approver,
    Viewer,
}
impl LocalRole {
    pub fn may_edit(self) -> bool {
        matches!(self, Self::Owner | Self::Editor)
    }
    pub fn may_approve(self) -> bool {
        matches!(self, Self::Owner | Self::Approver)
    }
    pub fn is_owner(self) -> bool {
        self == Self::Owner
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Editor => "editor",
            Self::Approver => "approver",
            Self::Viewer => "viewer",
        }
    }
    pub fn permits(self, method: &str, path: &str, platform_admin: bool) -> bool {
        if path == "/v1/local/bootstrap" {
            return false;
        }
        if path == "/v1/organizations" || path.starts_with("/v1/local/workspaces") {
            return platform_admin;
        }
        if method == "GET" || method == "HEAD" {
            return true;
        }
        if method == "DELETE" && path.starts_with("/v1/local/sessions/") {
            return true;
        }
        if path == "/v1/local/resource-access/execute" {
            return self == Self::Viewer;
        }
        if path.starts_with("/v1/local/cases/") {
            if path.ends_with("/responses") {
                return self.may_approve();
            }
            if path.ends_with("/approve") || path.ends_with("/simulate-send") {
                return self == Self::Viewer;
            }
            if path.ends_with("/guidance") || path.ends_with("/requests") {
                return self == Self::Viewer;
            }
        }
        if path == "/local/auth/logout" {
            return true;
        }
        if path == "/v1/local/managed-queries" {
            return true;
        }
        if path.starts_with("/v1/local/managed-executions/")
            && (path.ends_with("/cancel") || path.ends_with("/resume"))
        {
            return true;
        }
        if path.starts_with("/v1/local/authorized-memories") {
            return true;
        }
        if self.is_owner() {
            return true;
        }
        if path.starts_with("/v1/local/users")
            || path.starts_with("/v1/oauth/")
            || path.starts_with("/v1/ai/")
            || path == "/v1/creative/budget"
            || path.starts_with("/v1/local/retention")
        {
            return false;
        }
        if path.ends_with("/approve") {
            return self.may_approve();
        }
        if path.ends_with("/cancel") {
            return self.may_edit() || self.may_approve();
        }
        if path.starts_with("/v1/local/assets")
            || path.starts_with("/v1/local/conversations")
            || path.starts_with("/v1/local/publications")
            || path.starts_with("/v1/local/link-codes")
        {
            return self.may_edit();
        }
        if path == "/v1/creative/jobs" || path.ends_with("/cancel") {
            return self.may_edit();
        }
        false
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalUser {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization: String,
    pub username: String,
    pub display_name: String,
    pub role: LocalRole,
    pub platform_admin: bool,
    pub disabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalIntent {
    CreateVideo,
    CreateImage,
    PreparePublication(&'static str),
    AcademicSummary,
    ListStudentPayments,
    LookupStudentPayment(String),
    Status,
    Help,
    Link(String),
    ApprovalHelp,
}
pub fn interpret_local(text: &str) -> LocalIntent {
    let lower = text.trim().to_lowercase();
    if matches!(
        lower.as_str(),
        "mis pagos" | "historial de pagos" | "mis pagos uncp"
    ) {
        return LocalIntent::ListStudentPayments;
    }
    let code = lower.split_whitespace().find(|part| {
        part.starts_with("uncp-")
            || part.starts_with("pa5700")
            || (part.starts_with('v') && part.chars().nth(1).is_some_and(|c| c.is_ascii_digit()))
    });
    if let Some(code) = code {
        let normalized = code.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
        if normalized.starts_with("uncp-")
            || normalized.starts_with("pa5700")
            || (normalized.starts_with('v')
                && normalized
                    .chars()
                    .nth(1)
                    .is_some_and(|c| c.is_ascii_digit()))
        {
            return LocalIntent::LookupStudentPayment(normalized.to_uppercase());
        }
    }
    if matches!(
        lower.as_str(),
        "mis calificaciones" | "mis notas" | "resumen académico" | "resumen academico"
    ) {
        return LocalIntent::AcademicSummary;
    }
    if let Some(code) = lower.strip_prefix("/vincular ") {
        return LocalIntent::Link(code.trim().to_owned());
    }
    if lower.starts_with("/aprobar") || lower == "aprobar" || lower == "confirmar" {
        return LocalIntent::ApprovalHelp;
    }
    let platform = if lower.contains("instagram") {
        Some("instagram")
    } else if lower.contains("tiktok") {
        Some("tiktok")
    } else if lower.contains("facebook") {
        Some("facebook")
    } else {
        None
    };
    if let Some(platform) = platform
        && (lower.contains("public")
            || lower.starts_with("y ")
            || lower.starts_with("para ")
            || lower.contains("prepara"))
    {
        return LocalIntent::PreparePublication(platform);
    }
    if lower.starts_with("/video") || lower.contains("video") || lower.contains("vídeo") {
        return LocalIntent::CreateVideo;
    }
    if lower.starts_with("/diseno")
        || lower.contains("diseño")
        || lower.contains("imagen")
        || lower.contains("anuncio")
    {
        return LocalIntent::CreateImage;
    }
    if lower.contains("estado") || lower == "como va" || lower == "cómo va" {
        return LocalIntent::Status;
    }
    LocalIntent::Help
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roles_enforce_approval_and_management_separation() {
        assert!(!LocalRole::Viewer.permits("POST", "/v1/creative/jobs/x/approve", false));
        assert!(!LocalRole::Editor.permits("POST", "/v1/creative/jobs/x/approve", false));
        assert!(LocalRole::Approver.permits("POST", "/v1/creative/jobs/x/approve", false));
        assert!(!LocalRole::Editor.permits("POST", "/v1/local/users", false));
        assert!(LocalRole::Viewer.permits("POST", "/v1/local/resource-access/execute", false));
        assert!(!LocalRole::Editor.permits("POST", "/v1/local/resource-access/execute", false));
        assert!(!LocalRole::Viewer.permits("POST", "/v1/local/resource-access/grants", false));
        assert!(LocalRole::Viewer.permits("DELETE", "/v1/local/sessions/123", false));
        assert!(!LocalRole::Owner.permits("POST", "/v1/organizations", false));
        assert!(LocalRole::Owner.permits("POST", "/v1/local/workspaces", true));
        assert!(LocalRole::Viewer.permits("POST", "/v1/local/cases/1/guidance", false));
        assert!(LocalRole::Viewer.permits("POST", "/v1/local/cases/1/requests/2/approve", false));
        assert!(!LocalRole::Editor.permits("POST", "/v1/local/cases/1/responses", false));
        assert!(LocalRole::Approver.permits("POST", "/v1/local/cases/1/responses", false));
    }
    #[test]
    fn local_commands_are_deterministic_and_never_publish_directly() {
        assert_eq!(
            interpret_local("Crea un video de este producto"),
            LocalIntent::CreateVideo
        );
        assert_eq!(
            interpret_local("Y para TikTok"),
            LocalIntent::PreparePublication("tiktok")
        );
        assert_eq!(interpret_local("aprobar"), LocalIntent::ApprovalHelp);
        assert_eq!(
            interpret_local("UNCP-EXP-004821"),
            LocalIntent::LookupStudentPayment("UNCP-EXP-004821".into())
        );
        assert_eq!(
            interpret_local("estado PA57002335"),
            LocalIntent::LookupStudentPayment("PA57002335".into())
        );
        assert_eq!(interpret_local("Dame el saldo bancario"), LocalIntent::Help);
        assert_eq!(
            interpret_local("mis calificaciones"),
            LocalIntent::AcademicSummary
        );
        assert_eq!(interpret_local("MIS NOTAS"), LocalIntent::AcademicSummary);
        assert_eq!(
            interpret_local("mis calificaciones y cambia la nota"),
            LocalIntent::Help
        );
    }
}
