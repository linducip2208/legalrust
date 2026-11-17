use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use legalrust_application::AppError;
use legalrust_database::{StoreError, UserStore};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("hash failed")]
    Hash,
    #[error("invalid credentials")]
    Invalid,
    #[error("token error: {0}")]
    Token(String),
}
pub fn hash_password(plain: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AuthError::Hash)
}
pub fn verify_password(hash: &str, plain: &str) -> Result<(), AuthError> {
    let parsed = PasswordHash::new(hash).map_err(|_| AuthError::Invalid)?;
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .map_err(|_| AuthError::Invalid)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub org: String,
    pub roles: Vec<String>,
    pub sid: String,
    pub exp: usize,
}
pub fn issue_token(secret: &[u8], claims: &Claims) -> Result<String, AuthError> {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        claims,
        &jsonwebtoken::EncodingKey::from_secret(secret),
    )
    .map_err(|e| AuthError::Token(e.to_string()))
}
pub fn validate_token(secret: &[u8], token: &str) -> Result<Claims, AuthError> {
    jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret),
        &jsonwebtoken::Validation::default(),
    )
    .map(|d| d.claims)
    .map_err(|e| AuthError::Token(e.to_string()))
}
pub fn recovery_codes(n: usize) -> Vec<String> {
    use rand_core::RngCore;
    (0..n)
        .map(|_| {
            let mut b = [0u8; 10];
            OsRng.fill_bytes(&mut b);
            hex::encode(b)
        })
        .collect()
}
fn map_store(e: StoreError) -> AppError {
    match e {
        StoreError::NotFound => AppError::NotFound("record".into()),
        StoreError::Conflict(m) => AppError::Conflict(m),
        StoreError::Db(m) => AppError::Internal(anyhow::Error::msg(m)),
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub user_id: String,
    pub org_id: String,
    pub token: String,
    pub session_id: String,
}
fn expiry() -> (chrono::NaiveDateTime, usize) {
    let dt = chrono::Utc::now() + chrono::Duration::hours(24);
    (dt.naive_utc(), dt.timestamp() as usize)
}
pub fn check_secret(secret: &[u8]) -> Result<(), AppError> {
    if secret.is_empty() {
        return Err(AppError::BlockedExternal(
            "JWT_SECRET not configured".into(),
        ));
    }
    Ok(())
}
pub async fn register(
    store: &dyn UserStore,
    secret: &[u8],
    org: &str,
    email: &str,
    name: &str,
    pw: &str,
) -> Result<Session, AppError> {
    check_secret(secret)?;
    if !email.contains('@') || pw.len() < 8 {
        return Err(AppError::Validation("email/password invalid".into()));
    }
    let hash = hash_password(pw).map_err(|_| AppError::Internal(anyhow::Error::msg("hash")))?;
    let (org_id, uid) = store
        .create_org_owner(org, email, name, &hash)
        .await
        .map_err(map_store)?;
    let (exp_dt, exp) = expiry();
    let sid = store
        .create_session(&uid, exp_dt)
        .await
        .map_err(map_store)?;
    let token = issue_token(
        secret,
        &Claims {
            sub: uid.clone(),
            org: org_id.clone(),
            roles: vec!["org_owner".into()],
            sid: sid.clone(),
            exp,
        },
    )
    .map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;
    Ok(Session {
        user_id: uid,
        org_id,
        token,
        session_id: sid,
    })
}
pub async fn login(
    store: &dyn UserStore,
    secret: &[u8],
    email: &str,
    pw: &str,
) -> Result<Session, AppError> {
    check_secret(secret)?;
    let u = store
        .find_by_email(email)
        .await
        .map_err(map_store)?
        .ok_or(AppError::Unauthorized)?;
    if u.status != "active" {
        return Err(AppError::Unauthorized);
    }
    verify_password(&u.pw_hash, pw).map_err(|_| AppError::Unauthorized)?;
    let (exp_dt, exp) = expiry();
    let sid = store
        .create_session(&u.id, exp_dt)
        .await
        .map_err(map_store)?;
    let token = issue_token(
        secret,
        &Claims {
            sub: u.id.clone(),
            org: u.org_id.clone(),
            roles: vec!["org_owner".into()],
            sid: sid.clone(),
            exp,
        },
    )
    .map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;
    Ok(Session {
        user_id: u.id,
        org_id: u.org_id,
        token,
        session_id: sid,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use legalrust_database::memory::MemoryStores;
    #[test]
    fn password_roundtrip() {
        let h = hash_password("s3cret-pass").unwrap();
        assert!(verify_password(&h, "s3cret-pass").is_ok());
        assert!(verify_password(&h, "wrong").is_err());
    }
    #[test]
    fn jwt_roundtrip() {
        let c = Claims {
            sub: "u1".into(),
            org: "o1".into(),
            roles: vec!["notary".into()],
            sid: "s1".into(),
            exp: 9_999_999_999,
        };
        let t = issue_token(b"test-secret", &c).unwrap();
        let back = validate_token(b"test-secret", &t).unwrap();
        assert_eq!(back.sub, "u1");
        assert!(validate_token(b"other", &t).is_err());
    }
    #[tokio::test]
    async fn register_then_login_memory() {
        let s = MemoryStores::default();
        let r = register(&s, b"secret", "Acme", "a@x.id", "Ayu", "password1")
            .await
            .unwrap();
        assert_eq!(r.org_id.chars().count(), 5);
        let l = login(&s, b"secret", "a@x.id", "password1").await.unwrap();
        assert_eq!(l.org_id, r.org_id);
        assert!(login(&s, b"secret", "a@x.id", "nope-nope").await.is_err());
        assert!(register(&s, b"", "Acme", "b@x.id", "Bud", "password1")
            .await
            .is_err());
    }
}
