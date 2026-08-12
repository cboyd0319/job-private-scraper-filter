//! Type definitions for database integrity checking

/// Result of an integrity check
#[derive(Debug, Clone)]
pub(super) struct CheckResult {
    pub is_ok: bool,
}

/// Status of database integrity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegrityStatus {
    Healthy,
    Corrupted,
    ForeignKeyViolations,
}
