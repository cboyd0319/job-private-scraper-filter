use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use jobsentinel_domain::{v3_source_authorization::SourceGrantState, Job};

use super::super::BookmarkletRepository;

pub(super) struct TestBookmarkletRepository {
    authorization_allowed: AtomicBool,
    applied_hashes: Mutex<Vec<String>>,
    jobs: Mutex<Vec<Job>>,
}

impl Default for TestBookmarkletRepository {
    fn default() -> Self {
        Self {
            authorization_allowed: AtomicBool::new(true),
            applied_hashes: Mutex::new(Vec::new()),
            jobs: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl BookmarkletRepository for TestBookmarkletRepository {
    async fn authorize_browser_action(&self, _grant: &SourceGrantState) -> Result<bool, String> {
        Ok(self.authorization_allowed.load(Ordering::Relaxed))
    }

    async fn job_exists_by_hash(&self, hash: &str) -> Result<bool, String> {
        Ok(self.jobs().iter().any(|job| job.hash == hash))
    }

    async fn upsert_job(&self, job: &Job) -> Result<i64, String> {
        let mut jobs = self.jobs.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(existing) = jobs.iter().position(|stored| stored.hash == job.hash) {
            jobs[existing] = job.clone();
            return Ok(i64::try_from(existing + 1).unwrap_or(i64::MAX));
        }
        jobs.push(job.clone());
        Ok(i64::try_from(jobs.len()).unwrap_or(i64::MAX))
    }

    async fn mark_job_applied(&self, hash: &str) -> Result<(), String> {
        self.applied_hashes
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(hash.to_string());
        Ok(())
    }
}

impl TestBookmarkletRepository {
    pub(super) fn set_authorization_allowed(&self, allowed: bool) {
        self.authorization_allowed.store(allowed, Ordering::Relaxed);
    }

    pub(super) fn jobs(&self) -> Vec<Job> {
        self.jobs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(super) fn is_applied(&self, hash: &str) -> bool {
        self.applied_hashes
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .any(|applied| applied == hash)
    }
}

pub(super) async fn bookmarklet_test_database() -> Arc<TestBookmarkletRepository> {
    Arc::new(TestBookmarkletRepository::default())
}

pub(super) async fn stored_job_count(database: &TestBookmarkletRepository) -> usize {
    database.jobs().len()
}
