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
// ---- Marketing lead pipeline ----

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn twelve_roles_and_permissions() {
        assert_eq!(Role::all().len(), 12);
        assert_eq!(Permission::all().len(), 12);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketingLeadStatus {
    New,
    Contacted,
    Qualified,
    Demo,
    Proposal,
    Negotiation,
    Won,
    Lost,
}

impl MarketingLeadStatus {
    pub fn parse(s: &str) -> Option<MarketingLeadStatus> {
        Some(match s {
            "new" => MarketingLeadStatus::New,
            "contacted" => MarketingLeadStatus::Contacted,
            "qualified" => MarketingLeadStatus::Qualified,
            "demo" => MarketingLeadStatus::Demo,
            "proposal" => MarketingLeadStatus::Proposal,
            "negotiation" => MarketingLeadStatus::Negotiation,
            "won" => MarketingLeadStatus::Won,
            "lost" => MarketingLeadStatus::Lost,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            MarketingLeadStatus::New => "new",
            MarketingLeadStatus::Contacted => "contacted",
            MarketingLeadStatus::Qualified => "qualified",
            MarketingLeadStatus::Demo => "demo",
            MarketingLeadStatus::Proposal => "proposal",
            MarketingLeadStatus::Negotiation => "negotiation",
            MarketingLeadStatus::Won => "won",
            MarketingLeadStatus::Lost => "lost",
        }
    }

    /// Forward-only pipeline with terminal won/lost; lost may reopen to new.
    pub fn allowed_transition(from: MarketingLeadStatus, to: MarketingLeadStatus) -> bool {
        use MarketingLeadStatus::*;
        matches!(
            (from, to),
            (New, Contacted)
                | (New, Lost)
                | (Contacted, Qualified)
                | (Contacted, Lost)
                | (Qualified, Demo)
                | (Qualified, Lost)
                | (Demo, Proposal)
                | (Demo, Lost)
                | (Proposal, Negotiation)
                | (Proposal, Lost)
                | (Negotiation, Won)
                | (Negotiation, Lost)
                | (Lost, New)
        )
    }
}

#[cfg(test)]
mod marketing_tests {
    use super::*;

    #[test]
    fn parse_all_statuses_roundtrip() {
        for s in [
            "new",
            "contacted",
            "qualified",
            "demo",
            "proposal",
            "negotiation",
            "won",
            "lost",
        ] {
            let st = MarketingLeadStatus::parse(s).expect("parse");
            assert_eq!(st.as_str(), s);
        }
        assert_eq!(MarketingLeadStatus::parse("bogus"), None);
    }

    #[test]
    fn happy_path_pipeline_is_allowed() {
        use MarketingLeadStatus::*;
        let path = [
            (New, Contacted),
            (Contacted, Qualified),
            (Qualified, Demo),
            (Demo, Proposal),
            (Proposal, Negotiation),
            (Negotiation, Won),
        ];
        for (from, to) in path {
            assert!(MarketingLeadStatus::allowed_transition(from, to));
        }
    }

    #[test]
    fn skip_ahead_is_illegal() {
        use MarketingLeadStatus::*;
        assert!(!MarketingLeadStatus::allowed_transition(New, Qualified));
        assert!(!MarketingLeadStatus::allowed_transition(New, Demo));
        assert!(!MarketingLeadStatus::allowed_transition(Contacted, Demo));
        assert!(!MarketingLeadStatus::allowed_transition(Demo, Won));
    }

    #[test]
    fn lost_from_any_stage_but_terminal_won() {
        use MarketingLeadStatus::*;
        for from in [New, Contacted, Qualified, Demo, Proposal, Negotiation] {
            assert!(MarketingLeadStatus::allowed_transition(from, Lost));
        }
        assert!(!MarketingLeadStatus::allowed_transition(Won, Lost));
        assert!(!MarketingLeadStatus::allowed_transition(Lost, Won));
    }

    #[test]
    fn lost_can_reopen_to_new() {
        use MarketingLeadStatus::*;
        assert!(MarketingLeadStatus::allowed_transition(Lost, New));
        assert!(!MarketingLeadStatus::allowed_transition(Lost, Contacted));
    }

    #[test]
    fn terminal_won_has_no_outgoing() {
        use MarketingLeadStatus::*;
        for to in [
            New,
            Contacted,
            Qualified,
            Demo,
            Proposal,
            Negotiation,
            Won,
            Lost,
        ] {
            assert!(!MarketingLeadStatus::allowed_transition(Won, to));
        }
    }

    #[test]
    fn self_transition_is_illegal() {
        use MarketingLeadStatus::*;
        for st in [
            New,
            Contacted,
            Qualified,
            Demo,
            Proposal,
            Negotiation,
            Won,
            Lost,
        ] {
            assert!(!MarketingLeadStatus::allowed_transition(st, st));
        }
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

// ---- CRM (client meetings + tags) ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetingInput {
    pub title: String,
    pub notes: Option<String>,
    pub happened_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeetingError {
    TitleTooShort,
    BadTimestamp,
}

impl std::fmt::Display for MeetingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeetingError::TitleTooShort => write!(f, "title must be at least 3 chars"),
            MeetingError::BadTimestamp => write!(f, "happened_at must be YYYY-MM-DD HH:MM:SS"),
        }
    }
}

impl std::error::Error for MeetingError {}

/// Title needs >= 3 chars; happened_at must parse as "%Y-%m-%d %H:%M:%S".
pub fn validate_meeting(input: &MeetingInput) -> Result<(), MeetingError> {
    if input.title.trim().chars().count() < 3 {
        return Err(MeetingError::TitleTooShort);
    }
    if chrono::NaiveDateTime::parse_from_str(&input.happened_at, "%Y-%m-%d %H:%M:%S").is_err() {
        return Err(MeetingError::BadTimestamp);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagName(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagError {
    Empty,
    TooLong,
}

impl std::fmt::Display for TagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagError::Empty => write!(f, "tag name must not be empty"),
            TagError::TooLong => write!(f, "tag name must be at most 64 chars"),
        }
    }
}

impl std::error::Error for TagError {}

impl TagName {
    pub fn parse(raw: &str) -> Result<TagName, TagError> {
        let name = raw.trim();
        if name.is_empty() {
            return Err(TagError::Empty);
        }
        if name.chars().count() > 64 {
            return Err(TagError::TooLong);
        }
        Ok(TagName(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod crm_tests {
    use super::*;

    #[test]
    fn valid_meeting_passes() {
        let input = MeetingInput {
            title: "Kickoff call".into(),
            notes: Some("discussed scope".into()),
            happened_at: "2026-11-18 10:00:00".into(),
        };
        assert!(validate_meeting(&input).is_ok());
    }

    #[test]
    fn short_title_rejected() {
        for bad in ["", "  ", "ab"] {
            let input = MeetingInput {
                title: bad.into(),
                notes: None,
                happened_at: "2026-11-18 10:00:00".into(),
            };
            assert_eq!(validate_meeting(&input), Err(MeetingError::TitleTooShort));
        }
    }

    #[test]
    fn bad_timestamp_rejected() {
        for bad in ["", "2026-11-18", "2026-13-40 99:99:99", "18/11/2026 10:00"] {
            let input = MeetingInput {
                title: "Kickoff call".into(),
                notes: None,
                happened_at: bad.into(),
            };
            assert_eq!(validate_meeting(&input), Err(MeetingError::BadTimestamp));
        }
    }

    #[test]
    fn tag_parse_trims_and_roundtrips() {
        let tag = TagName::parse("  vip  ").expect("parse");
        assert_eq!(tag.as_str(), "vip");
    }

    #[test]
    fn tag_rejects_empty() {
        for bad in ["", "   "] {
            assert_eq!(TagName::parse(bad), Err(TagError::Empty));
        }
    }

    #[test]
    fn tag_rejects_overlong() {
        let long = "x".repeat(65);
        assert_eq!(TagName::parse(&long), Err(TagError::TooLong));
        let max = "y".repeat(64);
        assert!(TagName::parse(&max).is_ok());
    }
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
