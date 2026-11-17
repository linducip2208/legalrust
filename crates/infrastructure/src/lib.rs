use std::env;
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub s3_endpoint: String,
    pub s3_bucket: String,
    pub jwt_secret_set: bool,
}
impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "mysql://root:legalrust@127.0.0.1:3306/legalrust".into()),
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            s3_endpoint: env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:9000".into()),
            s3_bucket: env::var("S3_BUCKET").unwrap_or_else(|_| "legalrust".into()),
            jwt_secret_set: env::var("JWT_SECRET")
                .map(|s| !s.is_empty())
                .unwrap_or(false),
        }
    }
}
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}
#[derive(Debug, Clone)]
pub struct AppState {
    pub version: &'static str,
    pub pool: Option<sqlx::MySqlPool>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_apply() {
        env::remove_var("PORT");
        assert_eq!(Config::from_env().port, 8080);
    }
}
