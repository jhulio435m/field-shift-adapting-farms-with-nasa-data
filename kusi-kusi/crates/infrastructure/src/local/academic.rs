use super::LocalStore;
use kusi_application::academic::{
    AcademicReadError, AcademicRecordReader, AcademicSummary, GuardianStudentRepository,
    ReadGuardianAcademicSummary,
};
use kusi_application::local::LocalError;
use kusi_domain::local::LocalUser;
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct DemoStudent {
    pub student_ref: String,
    pub display_label: String,
    pub level: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct GuardianStudentLink {
    pub user_id: Uuid,
    pub user_name: String,
    pub student_ref: String,
    pub student_label: String,
    pub created_at: i64,
}

fn fixture_summaries() -> Vec<AcademicSummary> {
    vec![
        AcademicSummary {
            student_ref: "demo-student-001".into(),
            display_label: "Estudiante ficticio A".into(),
            level: "2.° de secundaria · demostración".into(),
            marks: vec![
                kusi_application::academic::AcademicMark {
                    subject: "Matemática".into(),
                    value: "16/20".into(),
                },
                kusi_application::academic::AcademicMark {
                    subject: "Comunicación".into(),
                    value: "18/20".into(),
                },
            ],
        },
        AcademicSummary {
            student_ref: "demo-student-002".into(),
            display_label: "Estudiante ficticio B".into(),
            level: "4.° de primaria · demostración".into(),
            marks: vec![kusi_application::academic::AcademicMark {
                subject: "Ciencia y Tecnología".into(),
                value: "17/20".into(),
            }],
        },
    ]
}

fn fixture_student(student_ref: &str) -> Option<DemoStudent> {
    fixture_summaries()
        .into_iter()
        .find(|student| student.student_ref == student_ref)
        .map(|student| DemoStudent {
            student_ref: student.student_ref,
            display_label: student.display_label,
            level: student.level,
        })
}

#[derive(Clone)]
struct PgAcademicGuardians {
    pool: PgPool,
}

#[async_trait::async_trait]
impl GuardianStudentRepository for PgAcademicGuardians {
    async fn linked_students(
        &self,
        organization: Uuid,
        user: Uuid,
    ) -> Result<Vec<String>, AcademicReadError> {
        sqlx::query_scalar::<_, String>(
            "SELECT l.student_ref FROM local_academic_guardian_links l \
             JOIN local_users u ON u.organization_id=l.organization_id AND u.id=l.user_id \
             WHERE l.organization_id=$1 AND l.user_id=$2 AND u.role='viewer' AND NOT u.disabled \
             ORDER BY l.student_ref",
        )
        .bind(organization)
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AcademicReadError::Unavailable)
    }
}

#[derive(Clone, Default)]
struct FixtureAcademicRecords;

#[async_trait::async_trait]
impl AcademicRecordReader for FixtureAcademicRecords {
    async fn summary(
        &self,
        student_ref: &str,
    ) -> Result<Option<AcademicSummary>, AcademicReadError> {
        Ok(fixture_summaries()
            .into_iter()
            .find(|student| student.student_ref == student_ref))
    }
}

impl LocalStore {
    pub async fn read_academic_summary(
        &self,
        organization: Uuid,
        user: Uuid,
    ) -> Result<Vec<AcademicSummary>, LocalError> {
        ReadGuardianAcademicSummary::new(
            PgAcademicGuardians {
                pool: self.pool.clone(),
            },
            FixtureAcademicRecords,
        )
        .execute(organization, user)
        .await
        .map_err(|error| match error {
            AcademicReadError::Forbidden => LocalError::Forbidden,
            AcademicReadError::Unavailable => LocalError::Unavailable,
        })
    }

    pub async fn demo_students(&self, actor: &LocalUser) -> Result<Vec<DemoStudent>, LocalError> {
        if !actor.role.is_owner() {
            return Err(LocalError::Forbidden);
        }
        Ok(fixture_summaries()
            .into_iter()
            .map(|student| DemoStudent {
                student_ref: student.student_ref,
                display_label: student.display_label,
                level: student.level,
            })
            .collect())
    }

    pub async fn guardian_links(
        &self,
        actor: &LocalUser,
    ) -> Result<Vec<GuardianStudentLink>, LocalError> {
        if !actor.role.is_owner() {
            return Err(LocalError::Forbidden);
        }
        sqlx::query_as::<_, GuardianStudentLink>(
            "SELECT l.user_id,u.display_name AS user_name,l.student_ref, \
             CASE l.student_ref WHEN 'demo-student-001' THEN 'Estudiante ficticio A' \
             ELSE 'Estudiante ficticio B' END AS student_label, \
             extract(epoch FROM l.created_at)::bigint AS created_at \
             FROM local_academic_guardian_links l \
             JOIN local_users u ON u.organization_id=l.organization_id AND u.id=l.user_id \
             WHERE l.organization_id=$1 AND u.role='viewer' AND NOT u.disabled \
             ORDER BY u.display_name,l.student_ref",
        )
        .bind(actor.organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| LocalError::Unavailable)
    }

    pub async fn create_guardian_link(
        &self,
        actor: &LocalUser,
        user_id: Uuid,
        student_ref: &str,
        relationship_verified: bool,
    ) -> Result<(), LocalError> {
        if !actor.role.is_owner() {
            return Err(LocalError::Forbidden);
        }
        if !relationship_verified {
            return Err(LocalError::Invalid);
        }
        if fixture_student(student_ref).is_none() {
            return Err(LocalError::Invalid);
        }
        let valid_user: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM local_users WHERE organization_id=$1 AND id=$2 \
             AND role='viewer' AND NOT disabled)",
        )
        .bind(actor.organization_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        if !valid_user {
            return Err(LocalError::NotFound);
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        sqlx::query(
            "INSERT INTO local_academic_guardian_links (organization_id,user_id,student_ref,created_by) \
             VALUES ($1,$2,$3,$4) ON CONFLICT (organization_id,user_id,student_ref) DO NOTHING",
        )
        .bind(actor.organization_id)
        .bind(user_id)
        .bind(student_ref)
        .bind(actor.id)
        .execute(&mut *tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        sqlx::query(
            "INSERT INTO local_audit (organization_id,user_id,action,target) \
             VALUES ($1,$2,'academic_demo_link_created',$3)",
        )
        .bind(actor.organization_id)
        .bind(actor.id)
        .bind(format!("{user_id}:{student_ref}"))
        .execute(&mut *tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)
    }

    pub async fn remove_guardian_link(
        &self,
        actor: &LocalUser,
        user_id: Uuid,
        student_ref: &str,
    ) -> Result<(), LocalError> {
        if !actor.role.is_owner() {
            return Err(LocalError::Forbidden);
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let result = sqlx::query(
            "DELETE FROM local_academic_guardian_links WHERE organization_id=$1 AND user_id=$2 AND student_ref=$3",
        )
        .bind(actor.organization_id)
        .bind(user_id)
        .bind(student_ref)
        .execute(&mut *tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        if result.rows_affected() == 0 {
            return Err(LocalError::NotFound);
        }
        sqlx::query(
            "INSERT INTO local_audit (organization_id,user_id,action,target) \
             VALUES ($1,$2,'academic_demo_link_removed',$3)",
        )
        .bind(actor.organization_id)
        .bind(actor.id)
        .bind(format!("{user_id}:{student_ref}"))
        .execute(&mut *tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_is_fixed_and_uses_only_fictional_identifiers() {
        let rows = fixture_summaries();
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .all(|row| row.student_ref.starts_with("demo-student-"))
        );
        assert!(
            rows.iter()
                .all(|row| row.display_label.contains("ficticio"))
        );
    }
}
