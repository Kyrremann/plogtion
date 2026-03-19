/// Minimal sysinfo stub for Scaleway deployment where the real sysinfo
/// crate requires a newer Rust than the runtime provides.
/// Returns 0 for available memory, which makes rust-s3 fall back to its
/// default of 3 concurrent multipart upload chunks.

pub struct System;

pub struct MemoryRefreshKind;

impl System {
    pub fn new() -> Self {
        System
    }

    pub fn refresh_memory_specifics(&mut self, _kind: MemoryRefreshKind) {}

    pub fn available_memory(&self) -> u64 {
        0
    }
}

impl MemoryRefreshKind {
    pub fn everything() -> Self {
        MemoryRefreshKind
    }
}
