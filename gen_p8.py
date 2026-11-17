#!/usr/bin/env python3
"""Final push part 4: small-crate edits + server extension (OCR/deeds/AI/RAG/finance/webhooks/hardening)."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")
def edit(rel, old, new, count=1):
    p = R/rel; t = p.read_text(encoding="utf-8")
    assert old in t, f"anchor missing in {rel}"
    p.write_text(t.replace(old, new, count), encoding="utf-8", newline="\n")

# --- workflow: parse/as_str ---
edit("crates/workflow/src/lib.rs",
 "pub fn transition(from: DeedState, to: DeedState) -> Result<DeedState, TransitionError> {",
 """impl DeedState {
    pub fn as_str(self) -> &'static str {
        match self { DeedState::Draft => "draft", DeedState::Collection => "collection", DeedState::Verification => "verification", DeedState::LegalReview => "legal_review", DeedState::Revision => "revision", DeedState::Approval => "approval", DeedState::Signing => "signing", DeedState::Registration => "registration", DeedState::Completion => "completion", DeedState::Archive => "archive" }
    }
    pub fn parse(s: &str) -> Option<DeedState> {
        Some(match s { "draft" => DeedState::Draft, "collection" => DeedState::Collection, "verification" => DeedState::Verification, "legal_review" => DeedState::LegalReview, "revision" => DeedState::Revision, "approval" => DeedState::Approval, "signing" => DeedState::Signing, "registration" => DeedState::Registration, "completion" => DeedState::Completion, "archive" => DeedState::Archive, _ => return None })
    }
}
pub fn transition(from: DeedState, to: DeedState) -> Result<DeedState, TransitionError> {""")
# --- application: RateLimited ---
edit("crates/application/src/lib.rs", "    #[error(\"conflict: {0}\")] Conflict(String),",
     "    #[error(\"conflict: {0}\")] Conflict(String),\n    #[error(\"rate limited\")] RateLimited,")
edit("crates/application/src/lib.rs", 'AppError::Conflict(_) => StatusCode::CONFLICT,',
     'AppError::Conflict(_) => StatusCode::CONFLICT,\n        AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,')
edit("crates/application/src/lib.rs", 'AppError::Conflict(_) => "CONFLICT",',
     'AppError::Conflict(_) => "CONFLICT",\n        AppError::RateLimited => "RATE_LIMITED",')
# --- server cargo: set-header ---
edit("apps/server/Cargo.toml", '"trace", "cors", "request-id", "limit"', '"trace", "cors", "request-id", "limit", "set-header"')
print("P4 OK")
