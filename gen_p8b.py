#!/usr/bin/env python3
"""Final push part 5: server route extension."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def edit(rel, old, new, count=1):
    p = R/rel; t = p.read_text(encoding="utf-8")
    assert old in t, f"anchor missing in {rel}: {old[:60]}"
    p.write_text(t.replace(old, new, count), encoding="utf-8", newline="\n")

H = '''use legalrust_storage::{LocalStore, ObjectStore};'''
edit("apps/server/src/lib.rs", H, H + '''
use legalrust_database::{DeedRow, InvoiceRow, OcrJobRow, OpsStore, TemplateRow};
use legalrust_workflow::{DeedState, transition};
use std::time::{Duration, Instant};''')

edit("apps/server/src/lib.rs", '''#[derive(Clone)]
pub struct Stores {
    pub users: Arc<dyn UserStore>,
    pub core: Arc<dyn CoreStore>,
    pub objects: Arc<dyn ObjectStore>,
    pub jwt_secret: Vec<u8>,
}''', '''#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<std::sync::Mutex<HashMap<String, (u32, Instant)>>>,
    max: u32,
}
impl RateLimiter {
    pub fn new(max: u32) -> Self { Self { inner: Arc::new(std::sync::Mutex::new(HashMap::new())), max } }
    pub fn check(&self, key: &str) -> bool {
        let mut m = self.inner.lock().unwrap();
        let now = Instant::now();
        let e = m.entry(key.into()).or_insert((0, now));
        if now.duration_since(e.1) > Duration::from_secs(60) { *e = (0, now); }
        if e.0 >= self.max { return false; }
        e.0 += 1;
        true
    }
}

#[derive(Clone)]
pub struct Stores {
    pub users: Arc<dyn UserStore>,
    pub core: Arc<dyn CoreStore>,
    pub ops: Arc<dyn OpsStore>,
    pub objects: Arc<dyn ObjectStore>,
    pub jwt_secret: Vec<u8>,
    pub integration_secret: Vec<u8>,
    pub integrations: Vec<String>,
    pub limiter: RateLimiter,
}
impl Stores {
    pub fn new(users: Arc<dyn UserStore>, core: Arc<dyn CoreStore>, ops: Arc<dyn OpsStore>, objects: Arc<dyn ObjectStore>, jwt_secret: Vec<u8>) -> Self {
        Self { users, core, ops, objects, jwt_secret, integration_secret: Vec::new(), integrations: vec!["dukcapil".into(), "bpn".into(), "psre".into()], limiter: RateLimiter::new(20) }
    }
}''')

edit("apps/server/src/lib.rs", '''async fn register(State(s): State<FullState>, Json(b): Json<RegisterReq>) -> Result<Json<Value>, AppError> {
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;''',
'''async fn register(State(s): State<FullState>, Json(b): Json<RegisterReq>) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("auth") { return Err(AppError::RateLimited); }
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;''')
edit("apps/server/src/lib.rs", '''async fn login(State(s): State<FullState>, Json(b): Json<LoginReq>) -> Result<Json<Value>, AppError> {
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;''',
'''async fn login(State(s): State<FullState>, Json(b): Json<LoginReq>) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("auth") { return Err(AppError::RateLimited); }''')
print("P5a OK")
