use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use jobsentinel_domain::{v3_source_authorization::SourceGrantState, Job};

const MAX_PENDING_URL_IMPORTS: usize = 20;
const PENDING_URL_IMPORT_LIFETIME: Duration = Duration::minutes(30);

#[derive(Clone, Default)]
pub struct PendingUrlImports {
    entries: Arc<RwLock<Vec<PendingUrlImport>>>,
}

#[derive(Clone)]
struct PendingUrlImport {
    id: String,
    created_at: DateTime<Utc>,
    job: Job,
    grant: SourceGrantState,
}

impl PendingUrlImports {
    pub(super) fn queue(&self, job: Job, grant: SourceGrantState, now: DateTime<Utc>) -> String {
        let mut entries = self.write_entries();
        retain_current(&mut entries, now);
        entries.retain(|entry| entry.job.hash != job.hash);
        while entries.len() >= MAX_PENDING_URL_IMPORTS {
            entries.remove(0);
        }

        let id = Uuid::new_v4().to_string();
        entries.push(PendingUrlImport {
            id: id.clone(),
            created_at: now,
            job,
            grant,
        });
        id
    }

    pub(super) fn find(&self, id: &str, now: DateTime<Utc>) -> Option<(Job, SourceGrantState)> {
        let mut entries = self.write_entries();
        retain_current(&mut entries, now);
        entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| (entry.job.clone(), entry.grant.clone()))
    }

    pub(super) fn remove(&self, id: &str) {
        self.write_entries().retain(|entry| entry.id != id);
    }

    pub(crate) fn current_count(&self, now: DateTime<Utc>) -> usize {
        let mut entries = self.write_entries();
        retain_current(&mut entries, now);
        entries.len()
    }

    pub(crate) const fn capacity() -> usize {
        MAX_PENDING_URL_IMPORTS
    }

    fn write_entries(&self) -> std::sync::RwLockWriteGuard<'_, Vec<PendingUrlImport>> {
        match self.entries.write() {
            Ok(entries) => entries,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

fn retain_current(entries: &mut Vec<PendingUrlImport>, now: DateTime<Utc>) {
    entries.retain(|entry| now - entry.created_at < PENDING_URL_IMPORT_LIFETIME);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(hash: &str) -> Job {
        let now = Utc::now();
        let mut job = Job::newly_discovered(
            "Office Manager",
            "Example Services",
            "https://example.com/jobs/office-manager",
            None,
            "import",
            now,
        );
        job.hash = hash.to_string();
        job.remote = Some(false);
        job
    }

    #[test]
    fn queue_replaces_the_same_job_and_expires_old_entries() {
        let pending = PendingUrlImports::default();
        let now = Utc::now();
        let first_id = pending.queue(job("same"), SourceGrantState::Missing, now);
        let replacement_id = pending.queue(
            job("same"),
            SourceGrantState::Missing,
            now + Duration::minutes(1),
        );

        assert!(pending
            .find(&first_id, now + Duration::minutes(1))
            .is_none());
        assert!(pending
            .find(&replacement_id, now + Duration::minutes(31))
            .is_none());
    }

    #[test]
    fn queued_work_count_is_bounded_and_expires_without_exposing_jobs() {
        let pending = PendingUrlImports::default();
        let now = Utc::now();
        pending.queue(job("first"), SourceGrantState::Missing, now);
        pending.queue(job("second"), SourceGrantState::Missing, now);

        assert_eq!(pending.current_count(now), 2);
        assert_eq!(PendingUrlImports::capacity(), 20);
        assert_eq!(pending.current_count(now + Duration::minutes(30)), 0);
    }
}
