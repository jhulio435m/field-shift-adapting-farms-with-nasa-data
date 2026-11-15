use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentState {
    Confirmed,
    Pending,
    NotFound,
    Reversed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessState {
    NotStarted,
    InProgress,
    Interrupted,
    Completed,
}

/// Explicit fixture/platform state; never inferred from payment confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoucherUseState {
    Available,
    Used,
    ReviewRequired,
}

#[derive(Debug, Clone, Serialize)]
pub struct StudentPayment {
    pub payment_code: String,
    pub procedure_code: String,
    pub concept: String,
    pub amount_cents: i64,
    pub currency: String,
    pub receipt_reference_masked: String,
    pub paid_at: Option<i64>,
    pub payment_state: PaymentState,
    pub voucher_use: VoucherUseState,
    pub process_state: ProcessState,
    pub platform_name: String,
    pub synthetic: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_states_serialize_as_stable_api_values() {
        assert_eq!(
            serde_json::to_string(&PaymentState::Confirmed).unwrap_or_default(),
            "\"confirmed\""
        );
        assert_eq!(
            serde_json::to_string(&ProcessState::Interrupted).unwrap_or_default(),
            "\"interrupted\""
        );
    }
}
