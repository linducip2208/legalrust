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

/// Bounded exponential-backoff retry policy (millisecond precision).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(5, 1_000)
    }
}

impl RetryPolicy {
    pub const MAX_DELAY_MS: u64 = 3_600_000;

    pub fn new(max_retries: u32, base_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms: base_delay_ms.max(1),
        }
    }

    /// Delay before the next retry for zero-based `attempt`.
    /// Returns `None` once `attempt >= max_retries`.
    pub fn next_delay(&self, attempt: u32) -> Option<u64> {
        if attempt >= self.max_retries {
            return None;
        }
        let shift = attempt.min(10);
        let delay = self.base_delay_ms.saturating_mul(1u64 << shift);
        Some(delay.min(Self::MAX_DELAY_MS))
    }
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
    #[test]
    fn retry_policy_exponential() {
        let p = RetryPolicy::new(3, 1_000);
        assert_eq!(p.next_delay(0), Some(1_000));
        assert_eq!(p.next_delay(1), Some(2_000));
        assert_eq!(p.next_delay(2), Some(4_000));
        assert_eq!(p.next_delay(3), None);
    }
    #[test]
    fn retry_policy_capped() {
        let p = RetryPolicy::new(20, 1_000_000);
        let d = p.next_delay(10).unwrap();
        assert!(d <= RetryPolicy::MAX_DELAY_MS);
        assert_eq!(RetryPolicy::default().next_delay(0), Some(1_000));
    }
}
