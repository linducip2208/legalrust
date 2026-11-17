//! Offline-first sync core for the mobile shell.
//!
//! Pure in-memory outbox queue with a 1 MiB payload guard, FIFO ordering,
//! retry backoff capped at [`MAX_RETRIES`], and a last-writer-wins conflict
//! resolver. No I/O, no network, no fake production data.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

/// Maximum accepted `payload_json` size in bytes (1 MiB).
pub const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;

/// Maximum retry attempts before an op is exhausted (dead-lettered).
pub const MAX_RETRIES: u32 = 5;

/// Base delay (seconds) for exponential backoff: `BASE * 2^retries`.
pub const BACKOFF_BASE_SECS: u64 = 5;

/// Maximum backoff delay (seconds): `BASE * 2^MAX_RETRIES` = 160 s.
pub const MAX_BACKOFF_SECS: u64 = 160;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxOp {
    pub id: Uuid,
    pub kind: String,
    pub payload_json: String,
    pub created_at: DateTime<Utc>,
    pub retries: u32,
}

impl OutboxOp {
    pub fn new(kind: impl Into<String>, payload_json: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: kind.into(),
            payload_json: payload_json.into(),
            created_at: Utc::now(),
            retries: 0,
        }
    }

    pub fn payload_len(&self) -> usize {
        self.payload_json.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    Oversize { size: usize, max: usize },
    EmptyKind,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Oversize { size, max } => {
                write!(f, "payload too large: {size} bytes (max {max})")
            }
            SyncError::EmptyKind => write!(f, "op kind must not be empty"),
        }
    }
}

impl std::error::Error for SyncError {}

/// Outcome of marking an op as failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailOutcome {
    /// Op stays queued with updated retry count and delay hint (seconds).
    Retrying { retries: u32, delay_secs: u64 },
    /// Op exceeded [`MAX_RETRIES`] and was dropped from the queue.
    Exhausted,
}

/// Pure exponential backoff in seconds, capped: `min(BASE * 2^retries, MAX)`.
pub fn backoff_delay_secs(retries: u32) -> u64 {
    let shift = retries.min(MAX_RETRIES);
    let delay = BACKOFF_BASE_SECS.saturating_mul(1u64 << shift);
    delay.min(MAX_BACKOFF_SECS)
}

#[derive(Debug, Default)]
pub struct SyncQueue {
    ops: VecDeque<OutboxOp>,
}

impl SyncQueue {
    pub fn new() -> Self {
        Self {
            ops: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Oldest queued op without removing it.
    pub fn pending(&self) -> Option<&OutboxOp> {
        self.ops.front()
    }

    /// FIFO snapshot in insertion order.
    pub fn list(&self) -> Vec<&OutboxOp> {
        self.ops.iter().collect()
    }

    /// Enqueue an op; rejects oversize payloads and empty kinds.
    pub fn push(&mut self, op: OutboxOp) -> Result<(), SyncError> {
        let size = op.payload_json.len();
        if size > MAX_PAYLOAD_BYTES {
            return Err(SyncError::Oversize {
                size,
                max: MAX_PAYLOAD_BYTES,
            });
        }
        if op.kind.trim().is_empty() {
            return Err(SyncError::EmptyKind);
        }
        self.ops.push_back(op);
        Ok(())
    }

    /// Acknowledge (remove) an op by id. Returns `true` if removed.
    /// No-op on an empty queue or unknown id.
    pub fn ack(&mut self, id: Uuid) -> bool {
        if let Some(pos) = self.ops.iter().position(|op| op.id == id) {
            self.ops.remove(pos);
            true
        } else {
            false
        }
    }

    /// Record a failure for an op by id: bump `retries` and report the
    /// capped backoff delay. Drops the op and returns [`FailOutcome::Exhausted`]
    /// once retries exceed [`MAX_RETRIES`]. Returns `None` for unknown ids
    /// (including the empty-queue case).
    pub fn fail(&mut self, id: Uuid) -> Option<FailOutcome> {
        let pos = self.ops.iter().position(|op| op.id == id)?;
        let next = self.ops[pos].retries.saturating_add(1);
        if next > MAX_RETRIES {
            self.ops.remove(pos);
            Some(FailOutcome::Exhausted)
        } else {
            self.ops[pos].retries = next;
            Some(FailOutcome::Retrying {
                retries: next,
                delay_secs: backoff_delay_secs(next),
            })
        }
    }
}

/// Last-writer-wins conflict resolver over epoch-second timestamps.
/// Ties favour [`Winner::Local`] (local op is already applied on-device).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Winner {
    Local,
    Remote,
}

pub fn resolve_conflict(local_updated: i64, remote_updated: i64) -> Winner {
    if remote_updated > local_updated {
        Winner::Remote
    } else {
        Winner::Local
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(kind: &str, payload: &str) -> OutboxOp {
        OutboxOp::new(kind, payload)
    }

    #[test]
    fn enqueue_ok() {
        let mut q = SyncQueue::new();
        assert!(q.push(op("doc.create", r#"{"a":1}"#)).is_ok());
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn ack_removes_op() {
        let mut q = SyncQueue::new();
        let o = op("doc.create", "{}");
        let id = o.id;
        q.push(o).unwrap();
        assert!(q.ack(id));
        assert!(q.is_empty());
    }

    #[test]
    fn fifo_order_preserved() {
        let mut q = SyncQueue::new();
        for k in ["a", "b", "c"] {
            q.push(op(k, "{}")).unwrap();
        }
        let kinds: Vec<&str> = q.list().iter().map(|o| o.kind.as_str()).collect();
        assert_eq!(kinds, vec!["a", "b", "c"]);
        let first = q.pending().unwrap().id;
        assert!(q.ack(first));
        assert_eq!(q.pending().unwrap().kind, "b");
    }

    #[test]
    fn fail_backoff_grows() {
        assert!(backoff_delay_secs(1) > backoff_delay_secs(0));
        assert!(backoff_delay_secs(2) > backoff_delay_secs(1));
        let mut q = SyncQueue::new();
        let o = op("doc.sync", "{}");
        let id = o.id;
        q.push(o).unwrap();
        match q.fail(id) {
            Some(FailOutcome::Retrying {
                retries,
                delay_secs,
            }) => {
                assert_eq!(retries, 1);
                assert_eq!(delay_secs, backoff_delay_secs(1));
            }
            other => panic!("expected Retrying, got {other:?}"),
        }
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn backoff_capped_at_max() {
        assert_eq!(backoff_delay_secs(5), MAX_BACKOFF_SECS);
        assert_eq!(backoff_delay_secs(100), MAX_BACKOFF_SECS);
        assert_eq!(backoff_delay_secs(u32::MAX), MAX_BACKOFF_SECS);
    }

    #[test]
    fn fail_exhausts_after_max_retries() {
        let mut q = SyncQueue::new();
        let o = op("doc.sync", "{}");
        let id = o.id;
        q.push(o).unwrap();
        for _ in 0..MAX_RETRIES {
            assert!(matches!(q.fail(id), Some(FailOutcome::Retrying { .. })));
        }
        assert_eq!(q.len(), 1);
        assert_eq!(q.fail(id), Some(FailOutcome::Exhausted));
        assert!(q.is_empty());
    }

    #[test]
    fn lww_local_wins_on_newer_local() {
        assert_eq!(resolve_conflict(200, 100), Winner::Local);
    }

    #[test]
    fn lww_remote_wins_on_newer_remote() {
        assert_eq!(resolve_conflict(100, 200), Winner::Remote);
    }

    #[test]
    fn lww_tie_favours_local() {
        assert_eq!(resolve_conflict(150, 150), Winner::Local);
    }

    #[test]
    fn oversize_payload_rejected() {
        let mut q = SyncQueue::new();
        let big = "x".repeat(MAX_PAYLOAD_BYTES + 1);
        let err = q.push(op("doc.create", &big)).unwrap_err();
        assert_eq!(
            err,
            SyncError::Oversize {
                size: MAX_PAYLOAD_BYTES + 1,
                max: MAX_PAYLOAD_BYTES,
            }
        );
        assert!(q.is_empty());
    }

    #[test]
    fn boundary_payload_accepted() {
        let mut q = SyncQueue::new();
        let exact = "x".repeat(MAX_PAYLOAD_BYTES);
        assert!(q.push(op("doc.create", &exact)).is_ok());
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn empty_queue_noop() {
        let mut q = SyncQueue::new();
        assert!(q.pending().is_none());
        assert!(q.list().is_empty());
        assert!(!q.ack(Uuid::new_v4()));
        assert_eq!(q.fail(Uuid::new_v4()), None);
    }
}
