use serde::{Deserialize, Serialize};
pub type TenantId = uuid::Uuid;
pub type UserId = uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    SuperAdmin,
    OrgOwner,
    OfficeAdmin,
    Notary,
    Ppat,
    Lawyer,
    LegalStaff,
    DocumentOfficer,
    FinanceOfficer,
    Auditor,
    Client,
    Reviewer,
}
impl Role {
    pub fn all() -> [Role; 12] {
        use Role::*;
        [
            SuperAdmin,
            OrgOwner,
            OfficeAdmin,
            Notary,
            Ppat,
            Lawyer,
            LegalStaff,
            DocumentOfficer,
            FinanceOfficer,
            Auditor,
            Client,
            Reviewer,
        ]
    }
    pub fn parse(s: &str) -> Option<Role> {
        Some(match s {
            "super_admin" => Role::SuperAdmin,
            "org_owner" => Role::OrgOwner,
            "office_admin" => Role::OfficeAdmin,
            "notary" => Role::Notary,
            "ppat" => Role::Ppat,
            "lawyer" => Role::Lawyer,
            "legal_staff" => Role::LegalStaff,
            "document_officer" => Role::DocumentOfficer,
            "finance_officer" => Role::FinanceOfficer,
            "auditor" => Role::Auditor,
            "client" => Role::Client,
            "reviewer" => Role::Reviewer,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Role::SuperAdmin => "super_admin",
            Role::OrgOwner => "org_owner",
            Role::OfficeAdmin => "office_admin",
            Role::Notary => "notary",
            Role::Ppat => "ppat",
            Role::Lawyer => "lawyer",
            Role::LegalStaff => "legal_staff",
            Role::DocumentOfficer => "document_officer",
            Role::FinanceOfficer => "finance_officer",
            Role::Auditor => "auditor",
            Role::Client => "client",
            Role::Reviewer => "reviewer",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    View,
    Create,
    Update,
    Delete,
    Approve,
    Reject,
    Export,
    Download,
    Share,
    Sign,
    Verify,
    Assign,
}
impl Permission {
    pub fn all() -> [Permission; 12] {
        use Permission::*;
        [
            View, Create, Update, Delete, Approve, Reject, Export, Download, Share, Sign, Verify,
            Assign,
        ]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn twelve_roles_and_permissions() {
        assert_eq!(Role::all().len(), 12);
        assert_eq!(Permission::all().len(), 12);
    }
}

// ---- Trust ledger (client trust accounts, double-entry guard) ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustAccount {
    pub id: i64,
    pub currency: String,
    pub balance_minor: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrustEntryKind {
    Debit,
    Credit,
}

impl TrustEntryKind {
    pub fn parse(s: &str) -> Option<TrustEntryKind> {
        Some(match s {
            "debit" => TrustEntryKind::Debit,
            "credit" => TrustEntryKind::Credit,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            TrustEntryKind::Debit => "debit",
            TrustEntryKind::Credit => "credit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustEntryLine {
    pub account_id: i64,
    pub kind: TrustEntryKind,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustError {
    NonPositiveAmount,
    CurrencyMismatch,
    InsufficientFunds,
    UnbalancedLines,
}

impl std::fmt::Display for TrustError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustError::NonPositiveAmount => write!(f, "amount must be positive"),
            TrustError::CurrencyMismatch => write!(f, "currency mismatch"),
            TrustError::InsufficientFunds => write!(f, "insufficient trust balance"),
            TrustError::UnbalancedLines => write!(f, "unbalanced lines: sum(lines) != amount"),
        }
    }
}

impl std::error::Error for TrustError {}

/// Single-sided posting guard: amount > 0, and debits must not drive the
/// balance negative. Credits always succeed.
pub fn validate_trust_post(
    balance_minor: i64,
    amount_minor: i64,
    kind: TrustEntryKind,
) -> Result<i64, TrustError> {
    if amount_minor <= 0 {
        return Err(TrustError::NonPositiveAmount);
    }
    match kind {
        TrustEntryKind::Credit => Ok(balance_minor + amount_minor),
        TrustEntryKind::Debit => {
            if balance_minor < amount_minor {
                return Err(TrustError::InsufficientFunds);
            }
            Ok(balance_minor - amount_minor)
        }
    }
}

/// Double-entry transfer guard: amount > 0, both legs share one currency,
/// source covers the amount. Callers post one debit leg (source) and one
/// credit leg (destination) so sum(lines) == 2 * amount with net zero.
pub fn validate_trust_transfer(
    from: &TrustAccount,
    to: &TrustAccount,
    amount_minor: i64,
) -> Result<(), TrustError> {
    if amount_minor <= 0 {
        return Err(TrustError::NonPositiveAmount);
    }
    if from.currency != to.currency {
        return Err(TrustError::CurrencyMismatch);
    }
    if from.balance_minor < amount_minor {
        return Err(TrustError::InsufficientFunds);
    }
    Ok(())
}

/// Explicit double-entry balance check: sum(debits) must equal sum(credits)
/// must equal the transfer amount.
pub fn validate_trust_lines(lines: &[TrustEntryLine], amount_minor: i64) -> Result<(), TrustError> {
    if amount_minor <= 0 {
        return Err(TrustError::NonPositiveAmount);
    }
    let mut debits: i64 = 0;
    let mut credits: i64 = 0;
    for l in lines {
        if l.amount_minor <= 0 {
            return Err(TrustError::NonPositiveAmount);
        }
        match l.kind {
            TrustEntryKind::Debit => debits += l.amount_minor,
            TrustEntryKind::Credit => credits += l.amount_minor,
        }
    }
    if debits != amount_minor || credits != amount_minor {
        return Err(TrustError::UnbalancedLines);
    }
    Ok(())
}

#[cfg(test)]
mod trust_tests {
    use super::*;

    fn acct(id: i64, currency: &str, balance: i64) -> TrustAccount {
        TrustAccount {
            id,
            currency: currency.into(),
            balance_minor: balance,
        }
    }

    #[test]
    fn transfer_balanced_ok() {
        let from = acct(1, "IDR", 1_000);
        let to = acct(2, "IDR", 0);
        assert!(validate_trust_transfer(&from, &to, 400).is_ok());
        let lines = vec![
            TrustEntryLine {
                account_id: 1,
                kind: TrustEntryKind::Debit,
                amount_minor: 400,
            },
            TrustEntryLine {
                account_id: 2,
                kind: TrustEntryKind::Credit,
                amount_minor: 400,
            },
        ];
        assert!(validate_trust_lines(&lines, 400).is_ok());
    }

    #[test]
    fn transfer_insufficient_funds() {
        let from = acct(1, "IDR", 100);
        let to = acct(2, "IDR", 0);
        assert_eq!(
            validate_trust_transfer(&from, &to, 500),
            Err(TrustError::InsufficientFunds)
        );
    }

    #[test]
    fn transfer_rejects_zero_and_negative() {
        let from = acct(1, "IDR", 1_000);
        let to = acct(2, "IDR", 0);
        assert_eq!(
            validate_trust_transfer(&from, &to, 0),
            Err(TrustError::NonPositiveAmount)
        );
        assert_eq!(
            validate_trust_transfer(&from, &to, -50),
            Err(TrustError::NonPositiveAmount)
        );
    }

    #[test]
    fn transfer_rejects_currency_mismatch() {
        let from = acct(1, "IDR", 1_000);
        let to = acct(2, "USD", 0);
        assert_eq!(
            validate_trust_transfer(&from, &to, 100),
            Err(TrustError::CurrencyMismatch)
        );
    }

    #[test]
    fn post_guards_debit_without_funds() {
        assert_eq!(
            validate_trust_post(100, 500, TrustEntryKind::Debit),
            Err(TrustError::InsufficientFunds)
        );
        assert_eq!(
            validate_trust_post(100, 0, TrustEntryKind::Credit),
            Err(TrustError::NonPositiveAmount)
        );
        assert_eq!(
            validate_trust_post(100, 50, TrustEntryKind::Credit),
            Ok(150)
        );
        assert_eq!(validate_trust_post(100, 50, TrustEntryKind::Debit), Ok(50));
    }

    #[test]
    fn lines_reject_imbalance() {
        let lines = vec![
            TrustEntryLine {
                account_id: 1,
                kind: TrustEntryKind::Debit,
                amount_minor: 400,
            },
            TrustEntryLine {
                account_id: 2,
                kind: TrustEntryKind::Credit,
                amount_minor: 300,
            },
        ];
        assert_eq!(
            validate_trust_lines(&lines, 400),
            Err(TrustError::UnbalancedLines)
        );
    }
}
