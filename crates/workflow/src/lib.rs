#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeedState {
    Draft,
    Collection,
    Verification,
    LegalReview,
    Revision,
    Approval,
    Signing,
    Registration,
    Completion,
    Archive,
}
#[derive(Debug, thiserror::Error)]
#[error("illegal transition {0:?} -> {1:?}")]
pub struct TransitionError(pub DeedState, pub DeedState);
const FLOW: &[(DeedState, DeedState)] = &[
    (DeedState::Draft, DeedState::Collection),
    (DeedState::Collection, DeedState::Verification),
    (DeedState::Verification, DeedState::LegalReview),
    (DeedState::LegalReview, DeedState::Revision),
    (DeedState::Revision, DeedState::LegalReview),
    (DeedState::LegalReview, DeedState::Approval),
    (DeedState::Approval, DeedState::Signing),
    (DeedState::Signing, DeedState::Registration),
    (DeedState::Registration, DeedState::Completion),
    (DeedState::Completion, DeedState::Archive),
];
impl DeedState {
    pub fn as_str(self) -> &'static str {
        match self {
            DeedState::Draft => "draft",
            DeedState::Collection => "collection",
            DeedState::Verification => "verification",
            DeedState::LegalReview => "legal_review",
            DeedState::Revision => "revision",
            DeedState::Approval => "approval",
            DeedState::Signing => "signing",
            DeedState::Registration => "registration",
            DeedState::Completion => "completion",
            DeedState::Archive => "archive",
        }
    }
    pub fn parse(s: &str) -> Option<DeedState> {
        Some(match s {
            "draft" => DeedState::Draft,
            "collection" => DeedState::Collection,
            "verification" => DeedState::Verification,
            "legal_review" => DeedState::LegalReview,
            "revision" => DeedState::Revision,
            "approval" => DeedState::Approval,
            "signing" => DeedState::Signing,
            "registration" => DeedState::Registration,
            "completion" => DeedState::Completion,
            "archive" => DeedState::Archive,
            _ => return None,
        })
    }
}
pub fn transition(from: DeedState, to: DeedState) -> Result<DeedState, TransitionError> {
    if FLOW.contains(&(from, to)) {
        Ok(to)
    } else {
        Err(TransitionError(from, to))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn happy_path() {
        let mut s = DeedState::Draft;
        for next in [
            DeedState::Collection,
            DeedState::Verification,
            DeedState::LegalReview,
            DeedState::Approval,
        ] {
            s = transition(s, next).unwrap();
        }
        assert_eq!(s, DeedState::Approval);
    }
    #[test]
    fn skips_rejected() {
        assert!(transition(DeedState::Draft, DeedState::Signing).is_err());
    }
}
