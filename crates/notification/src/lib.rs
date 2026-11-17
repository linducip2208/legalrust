#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Channel {
    InApp,
    Email,
    Sms,
    WhatsApp,
    Push,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Notification {
    pub tenant_id: String,
    pub channel: Channel,
    pub template: String,
    pub payload: serde_json::Value,
    pub attempts: u32,
}
pub fn retry_delay_secs(attempt: u32) -> u64 {
    60 * 2u64.pow(attempt.min(5))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backoff() {
        assert_eq!(
            (
                retry_delay_secs(0),
                retry_delay_secs(1),
                retry_delay_secs(5)
            ),
            (60, 120, 1920)
        );
    }
}
