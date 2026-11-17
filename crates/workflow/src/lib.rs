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
