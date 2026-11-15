use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AcademicReadError {
    #[error("no authorized academic records")]
    Forbidden,
    #[error("academic records are unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcademicMark {
    pub subject: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcademicSummary {
    pub student_ref: String,
    pub display_label: String,
    pub level: String,
    pub marks: Vec<AcademicMark>,
}

#[async_trait]
pub trait GuardianStudentRepository: Send + Sync {
    async fn linked_students(
        &self,
        organization: Uuid,
        user: Uuid,
    ) -> Result<Vec<String>, AcademicReadError>;
}

#[async_trait]
pub trait AcademicRecordReader: Send + Sync {
    async fn summary(
        &self,
        student_ref: &str,
    ) -> Result<Option<AcademicSummary>, AcademicReadError>;
}

pub struct ReadGuardianAcademicSummary<G, R> {
    guardians: G,
    records: R,
}

impl<G, R> ReadGuardianAcademicSummary<G, R>
where
    G: GuardianStudentRepository,
    R: AcademicRecordReader,
{
    pub fn new(guardians: G, records: R) -> Self {
        Self { guardians, records }
    }

    pub async fn execute(
        &self,
        organization: Uuid,
        user: Uuid,
    ) -> Result<Vec<AcademicSummary>, AcademicReadError> {
        let students = self.guardians.linked_students(organization, user).await?;
        if students.is_empty() {
            return Err(AcademicReadError::Forbidden);
        }
        let mut summaries = Vec::with_capacity(students.len());
        for student in students {
            let summary = self
                .records
                .summary(&student)
                .await?
                .ok_or(AcademicReadError::Unavailable)?;
            summaries.push(summary);
        }
        Ok(summaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Guardians(Vec<String>);

    #[async_trait]
    impl GuardianStudentRepository for Guardians {
        async fn linked_students(
            &self,
            _organization: Uuid,
            _user: Uuid,
        ) -> Result<Vec<String>, AcademicReadError> {
            Ok(self.0.clone())
        }
    }

    #[derive(Clone)]
    struct Records {
        reads: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl AcademicRecordReader for Records {
        async fn summary(
            &self,
            student_ref: &str,
        ) -> Result<Option<AcademicSummary>, AcademicReadError> {
            self.reads
                .lock()
                .map_err(|_| AcademicReadError::Unavailable)?
                .push(student_ref.to_owned());
            Ok(Some(AcademicSummary {
                student_ref: student_ref.to_owned(),
                display_label: "Estudiante ficticio".to_owned(),
                level: "Demostración".to_owned(),
                marks: vec![AcademicMark {
                    subject: "Matemática".to_owned(),
                    value: "16/20".to_owned(),
                }],
            }))
        }
    }

    #[tokio::test]
    async fn reads_only_students_linked_to_authenticated_user() {
        let reads = Arc::new(Mutex::new(Vec::new()));
        let use_case = ReadGuardianAcademicSummary::new(
            Guardians(vec!["demo-student-001".into()]),
            Records {
                reads: reads.clone(),
            },
        );
        let result = use_case
            .execute(Uuid::new_v4(), Uuid::new_v4())
            .await
            .expect("fixture returns linked student");
        assert_eq!(result.len(), 1);
        assert_eq!(
            *reads.lock().expect("test mutex poisoned"),
            ["demo-student-001"]
        );
    }

    #[tokio::test]
    async fn refuses_to_query_records_without_guardian_links() {
        let reads = Arc::new(Mutex::new(Vec::new()));
        let use_case = ReadGuardianAcademicSummary::new(
            Guardians(Vec::new()),
            Records {
                reads: reads.clone(),
            },
        );
        assert_eq!(
            use_case.execute(Uuid::new_v4(), Uuid::new_v4()).await,
            Err(AcademicReadError::Forbidden)
        );
        assert!(reads.lock().expect("test mutex poisoned").is_empty());
    }
}
