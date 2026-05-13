use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Atomic request counters for observability.
#[derive(Debug, Default)]
pub struct Counters {
    pub requests_total: AtomicU64,
    pub errors_total: AtomicU64,
    pub upstream_calls: AtomicU64,
    pub upstream_errors: AtomicU64,
}

impl Counters {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn inc_request(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_upstream(&self) {
        self.upstream_calls.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_upstream_error(&self) {
        self.upstream_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> CountersSnapshot {
        CountersSnapshot {
            requests_total: self.requests_total.load(Ordering::Relaxed),
            errors_total: self.errors_total.load(Ordering::Relaxed),
            upstream_calls: self.upstream_calls.load(Ordering::Relaxed),
            upstream_errors: self.upstream_errors.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CountersSnapshot {
    pub requests_total: u64,
    pub errors_total: u64,
    pub upstream_calls: u64,
    pub upstream_errors: u64,
}
