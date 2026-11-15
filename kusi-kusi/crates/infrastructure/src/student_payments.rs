use async_trait::async_trait;
use kusi_application::student_payments::{StudentPaymentError, StudentPaymentsPlatform};
use kusi_domain::{
    local::{LocalRole, LocalUser},
    student_payment::{PaymentState, ProcessState, StudentPayment, VoucherUseState},
};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[derive(Debug, thiserror::Error)]
pub enum ReceiptOcrError {
    #[error("image format is invalid")]
    InvalidImage,
    #[error("receipt reference was not detected")]
    NoReference,
    #[error("local OCR is unavailable")]
    Unavailable,
}

/// Decodes a bounded image in memory, extracts receipt-reference text locally, and returns
/// only the normalized reference. The photo and full OCR text are never persisted or logged.
pub fn extract_receipt_reference(bytes: &[u8]) -> Result<String, ReceiptOcrError> {
    if bytes.is_empty() || bytes.len() > 5 * 1024 * 1024 {
        return Err(ReceiptOcrError::InvalidImage);
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| ReceiptOcrError::InvalidImage)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|_| ReceiptOcrError::InvalidImage)?;
    if u64::from(decoded.width()) * u64::from(decoded.height()) > 20_000_000 {
        return Err(ReceiptOcrError::InvalidImage);
    }
    let mut safe_image = std::io::Cursor::new(Vec::new());
    decoded
        .write_to(&mut safe_image, image::ImageFormat::Png)
        .map_err(|_| ReceiptOcrError::InvalidImage)?;

    let mut child = Command::new("timeout")
        .args(["10s", "tesseract", "stdin", "stdout", "-l", "spa"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ReceiptOcrError::Unavailable)?;
    let Some(mut input) = child.stdin.take() else {
        return Err(ReceiptOcrError::Unavailable);
    };
    input
        .write_all(safe_image.get_ref())
        .map_err(|_| ReceiptOcrError::Unavailable)?;
    drop(input);
    let output = child
        .wait_with_output()
        .map_err(|_| ReceiptOcrError::Unavailable)?;
    if !output.status.success() {
        return Err(ReceiptOcrError::Unavailable);
    }
    let ocr_text = String::from_utf8_lossy(&output.stdout);
    extract_receipt_pattern(&ocr_text).ok_or(ReceiptOcrError::NoReference)
}

fn extract_receipt_pattern(text: &str) -> Option<String> {
    let compact = text
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_uppercase())
        .collect::<String>();
    let bytes = compact.as_bytes();
    bytes
        .windows(12)
        .enumerate()
        .find_map(|(index, candidate)| {
            (candidate[0] == b'V' && candidate[1..].iter().all(u8::is_ascii_digit)).then(|| {
                format!(
                    "{}-{}",
                    &compact[index..index + 4],
                    &compact[index + 4..index + 12]
                )
            })
        })
}

/// Local stand-in for the student payment platform that the hackathon team is building.
/// Every record is synthetic and this adapter performs no network calls.
#[derive(Clone, Copy, Default)]
pub struct LocalStudentPaymentsPlatform;

impl LocalStudentPaymentsPlatform {
    /// Fixed synthetic identifier for the loopback-only student payments demo.
    pub const DEMO_DNI: &'static str = "70000001";

    pub fn demo_user() -> LocalUser {
        LocalUser {
            id: uuid::Uuid::from_u128(0x4b555349000000000000000000000001),
            organization_id: uuid::Uuid::from_u128(0x4b555349000000000000000000000002),
            organization: "uncp-demo".into(),
            username: "estudiante-demo".into(),
            display_name: "Estudiante ficticio".into(),
            role: LocalRole::Viewer,
            platform_admin: false,
            disabled: false,
        }
    }

    fn records() -> Vec<(StudentPayment, &'static [&'static str])> {
        vec![
            (
                StudentPayment {
                    payment_code: "UNCP-EXP-004821".into(),
                    procedure_code: "PA57002335".into(),
                    concept: "Constancia de expedito para optar título profesional".into(),
                    amount_cents: 19_800,
                    currency: "PEN".into(),
                    receipt_reference_masked: "V***-***4821".into(),
                    paid_at: None,
                    payment_state: PaymentState::Confirmed,
                    process_state: ProcessState::Interrupted,
                    voucher_use: VoucherUseState::ReviewRequired,
                    platform_name: "Clon local de pagos estudiantiles UNCP".into(),
                    synthetic: true,
                },
                &["UNCPEXP004821", "PA57002335", "V99900004821"],
            ),
            (
                StudentPayment {
                    payment_code: "UNCP-EVAL-006173".into(),
                    procedure_code: "1.3.2.3.1.99.10".into(),
                    concept: "Inscripción a proceso de evaluación".into(),
                    amount_cents: 300,
                    currency: "PEN".into(),
                    receipt_reference_masked: "V***-***1432".into(),
                    paid_at: None,
                    payment_state: PaymentState::Confirmed,
                    process_state: ProcessState::Completed,
                    voucher_use: VoucherUseState::Used,
                    platform_name: "Clon local de pagos estudiantiles UNCP".into(),
                    synthetic: true,
                },
                &["UNCPEVAL006173", "V998000006173", "V36400011432"],
            ),
        ]
    }

    fn normalized(value: &str) -> String {
        value
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .flat_map(char::to_uppercase)
            .collect()
    }
}

#[async_trait]
impl StudentPaymentsPlatform for LocalStudentPaymentsPlatform {
    async fn list_for_student(
        &self,
        actor: &LocalUser,
    ) -> Result<Vec<StudentPayment>, StudentPaymentError> {
        if actor.disabled || !matches!(actor.role, LocalRole::Viewer | LocalRole::Owner) {
            return Err(StudentPaymentError::NotFound);
        }
        Ok(Self::records()
            .into_iter()
            .map(|(payment, _)| payment)
            .collect())
    }

    async fn find_for_student(
        &self,
        actor: &LocalUser,
        payment_code: &str,
    ) -> Result<StudentPayment, StudentPaymentError> {
        if actor.disabled || !matches!(actor.role, LocalRole::Viewer | LocalRole::Owner) {
            return Err(StudentPaymentError::NotFound);
        }
        let code = Self::normalized(payment_code);
        Self::records()
            .into_iter()
            .find(|(payment, aliases)| {
                Self::normalized(&payment.payment_code) == code
                    || aliases.iter().any(|alias| Self::normalized(alias) == code)
            })
            .map(|(payment, _)| payment)
            .ok_or(StudentPaymentError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_ocr_returns_only_a_bounded_voucher_reference() {
        assert_eq!(
            extract_receipt_pattern("RECIBO V999 - 00004821 S/ 198.00"),
            Some("V999-00004821".into())
        );
        assert_eq!(extract_receipt_pattern("DNI 12345678 S/ 198.00"), None);
    }

    #[tokio::test]
    async fn supplied_demo_reference_matches_three_soles_without_claiming_real_validation() {
        let payment = kusi_application::student_payments::validate_payment(
            &LocalStudentPaymentsPlatform,
            &LocalStudentPaymentsPlatform::demo_user(),
            "V364-00011432",
        )
        .await
        .expect("registered demo reference");
        assert_eq!(payment.amount_cents, 300);
        assert_eq!(payment.payment_code, "UNCP-EVAL-006173");
        assert!(payment.synthetic);
        assert_eq!(payment.payment_state, PaymentState::Confirmed);
        assert_eq!(payment.voucher_use, VoucherUseState::Used);
    }
    #[tokio::test]
    async fn demo_platform_lists_and_matches_only_synthetic_payment_codes() {
        let platform = LocalStudentPaymentsPlatform;
        let user = LocalStudentPaymentsPlatform::demo_user();
        let payments = platform.list_for_student(&user).await.unwrap_or_default();
        assert_eq!(payments.len(), 2);
        assert!(payments.iter().all(|payment| payment.synthetic));
        let found = platform
            .find_for_student(&user, "V999-00004821")
            .await
            .expect("synthetic receipt code is available in the fixture");
        assert_eq!(found.payment_code, "UNCP-EXP-004821");
        assert_eq!(found.amount_cents, 19_800);
        assert!(matches!(
            platform.find_for_student(&user, "UNCP-EXP-999999").await,
            Err(StudentPaymentError::NotFound)
        ));
    }
}
