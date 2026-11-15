use async_trait::async_trait;
use kusi_domain::{local::LocalUser, student_payment::StudentPayment};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StudentPaymentError {
    #[error("payment code is invalid")]
    Invalid,
    #[error("payment record not found")]
    NotFound,
    #[error("student payments unavailable")]
    Unavailable,
}

/// Port to a student payment platform. Identity is resolved from the verified session/channel user.
#[async_trait]
pub trait StudentPaymentsPlatform: Send + Sync {
    async fn list_for_student(
        &self,
        actor: &LocalUser,
    ) -> Result<Vec<StudentPayment>, StudentPaymentError>;
    async fn find_for_student(
        &self,
        actor: &LocalUser,
        payment_code: &str,
    ) -> Result<StudentPayment, StudentPaymentError>;
}

pub async fn list_payments<P: StudentPaymentsPlatform>(
    platform: &P,
    actor: &LocalUser,
) -> Result<Vec<StudentPayment>, StudentPaymentError> {
    platform.list_for_student(actor).await
}

pub async fn validate_payment<P: StudentPaymentsPlatform>(
    platform: &P,
    actor: &LocalUser,
    code: &str,
) -> Result<StudentPayment, StudentPaymentError> {
    let code = code.trim();
    if code.is_empty()
        || code.len() > 40
        || !code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(StudentPaymentError::Invalid);
    }
    platform
        .find_for_student(actor, &code.to_ascii_uppercase())
        .await
}

/// Extract one exact code from dictated or written input, without fuzzy matches or inventing zeros.
pub fn extract_payment_code(input: &str) -> Result<String, StudentPaymentError> {
    if input.len() > 2000 {
        return Err(StudentPaymentError::Invalid);
    }
    let words = input
        .to_lowercase()
        .replace(['-', '.', ',', ':', ';', '!', '?'], " ");
    let compact: String = words
        .split_whitespace()
        .map(|word| match word {
            "uve" | "ve" => "v",
            "cero" => "0",
            "uno" => "1",
            "dos" => "2",
            "tres" => "3",
            "cuatro" => "4",
            "cinco" => "5",
            "seis" => "6",
            "siete" => "7",
            "ocho" => "8",
            "nueve" => "9",
            "guion" | "guión" => "",
            _ => word,
        })
        .collect();
    let bytes = compact.as_bytes();
    let mut candidates = Vec::new();
    for (prefix, digits) in [("v", 11), ("uncpexp", 6), ("uncpeval", 6)] {
        let width = prefix.len() + digits;
        for (index, window) in bytes.windows(width).enumerate() {
            if window.starts_with(prefix.as_bytes())
                && window[prefix.len()..].iter().all(u8::is_ascii_digit)
                && !bytes.get(index + width).is_some_and(u8::is_ascii_digit)
                && let Ok(code) = std::str::from_utf8(window)
            {
                candidates.push(code.to_ascii_uppercase());
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }
    if candidates.is_empty()
        && !input.contains(char::is_whitespace)
        && input.len() <= 40
        && input
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Ok(input.to_ascii_uppercase());
    }
    Err(StudentPaymentError::Invalid)
}
#[cfg(test)]
mod dictated_code_tests {
    use super::*;
    #[test]
    fn extracts_exact_dictation_and_rejects_ambiguity() {
        assert_eq!(
            extract_payment_code("Consulta el código V 364 guion 00011432.")
                .ok()
                .as_deref(),
            Some("V36400011432")
        );
        assert_eq!(
            extract_payment_code("uve tres seis cuatro cero cero cero uno uno cuatro tres dos")
                .ok()
                .as_deref(),
            Some("V36400011432")
        );
        assert_eq!(
            extract_payment_code("consulta UNCP EXP 004821")
                .ok()
                .as_deref(),
            Some("UNCPEXP004821")
        );
        assert!(extract_payment_code("V36400011432 y V99900004821").is_err());
        assert!(extract_payment_code("consulta el código ve tres seis").is_err());
    }
}
