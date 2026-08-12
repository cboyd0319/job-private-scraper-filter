ALTER TABLE resume_job_matches
ADD COLUMN feedback_label TEXT
CHECK (feedback_label IN ('useful', 'not_relevant') OR feedback_label IS NULL);

ALTER TABLE resume_job_matches
ADD COLUMN feedback_recorded_at TEXT
CHECK (
    (feedback_label IS NULL AND feedback_recorded_at IS NULL)
    OR (feedback_label IS NOT NULL AND feedback_recorded_at IS NOT NULL)
);
