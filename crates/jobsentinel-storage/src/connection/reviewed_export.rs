// Produces bounded reviewed exports and records their durable recovery outcomes.

use super::{
    finish_recovery_operation_record,
    reviewed_export_inspect::{
        inspect_reviewed_export_inner, ExportComplete, ExportHeader, ExportRecord,
    },
    reviewed_export_sanitize::sanitize_export_fields,
    reviewed_export_schema::{
        count_query, export_query, validate_export_tables, ExportTable, EXCLUDED_DATA,
        EXCLUDED_TABLES, EXPORT_TABLES,
    },
    Database, MIGRATOR,
};
use futures::TryStreamExt;
use serde::Serialize;
use serde_json::Value;
use sqlx::{sqlite::SqlitePool, Row, SqliteConnection};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
use tempfile::NamedTempFile;
use uuid::Uuid;

pub(super) const EXPORT_SCHEMA: &str = "jobsentinel.v3.reviewed-export.v1";
pub(super) const EXPORT_FORMAT_VERSION: u32 = 1;
pub(super) const MAX_EXPORT_LINE_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReviewedExportSelection {
    pub(super) include_protected_records: bool,
}

impl ReviewedExportSelection {
    #[must_use]
    pub const fn including_protected_records() -> Self {
        Self {
            include_protected_records: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewedExportPlan {
    info: ReviewedExportInfo,
    migration_sequence: i64,
    excluded_data: Vec<&'static str>,
}

impl ReviewedExportPlan {
    #[must_use]
    pub const fn user_review_required(&self) -> bool {
        true
    }

    #[must_use]
    pub const fn contains_secrets(&self) -> bool {
        false
    }

    #[must_use]
    pub fn record_count(&self, section: &str) -> Option<u64> {
        self.info.section_counts.get(section).copied()
    }

    #[must_use]
    pub const fn total_record_count(&self) -> u64 {
        self.info.record_count
    }

    #[must_use]
    pub fn section_counts(&self) -> &BTreeMap<String, u64> {
        &self.info.section_counts
    }

    #[must_use]
    pub const fn protected_records_included(&self) -> bool {
        self.info.protected_records_included
    }

    #[must_use]
    pub const fn protected_application_answer_count(&self) -> u64 {
        self.info.protected_application_answer_count
    }

    #[must_use]
    pub const fn protected_resume_draft_count(&self) -> u64 {
        self.info.protected_resume_draft_count
    }

    #[must_use]
    pub fn excluded_data(&self) -> &[&'static str] {
        &self.excluded_data
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewedExportInfo {
    pub export_id: String,
    pub created_at: String,
    pub format_version: u32,
    pub record_count: u64,
    pub section_counts: BTreeMap<String, u64>,
    pub protected_records_included: bool,
    pub protected_application_answer_count: u64,
    pub protected_resume_draft_count: u64,
}

impl Database {
    pub async fn review_plaintext_export(
        &self,
        selection: ReviewedExportSelection,
    ) -> Result<ReviewedExportPlan, sqlx::Error> {
        let mut connection = self.pool.acquire().await?;
        let migration_sequence = verify_export_schema(&mut connection).await?;
        let (section_counts, protected_application_answer_count, protected_resume_draft_count) =
            export_counts(&mut connection, selection).await?;
        let record_count = sum_counts(&section_counts)?;
        let mut excluded_data = EXCLUDED_DATA.to_vec();
        if !selection.include_protected_records {
            excluded_data.push("protected answers and structured resume fields until selected");
        }
        Ok(ReviewedExportPlan {
            info: ReviewedExportInfo {
                export_id: Uuid::new_v4().to_string(),
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                format_version: EXPORT_FORMAT_VERSION,
                record_count,
                section_counts,
                protected_records_included: selection.include_protected_records,
                protected_application_answer_count,
                protected_resume_draft_count,
            },
            migration_sequence,
            excluded_data,
        })
    }

    pub async fn create_reviewed_export(
        &self,
        destination: &Path,
        plan: ReviewedExportPlan,
    ) -> Result<ReviewedExportInfo, sqlx::Error> {
        export_destination_parent(destination)?;
        self.start_export_operation(&plan).await?;
        let prepared = match self.prepare_reviewed_export(destination, &plan).await {
            Ok(prepared) => prepared,
            Err(error) => {
                self.fail_export_operation(&plan.info.export_id, &error)
                    .await;
                return Err(error);
            }
        };
        let info = match prepared.publish(destination) {
            Ok(info) => info,
            Err(error) => {
                self.fail_export_operation(&plan.info.export_id, &error)
                    .await;
                return Err(error);
            }
        };
        self.finish_export_operation_detached(&plan.info.export_id);
        Ok(info)
    }

    async fn prepare_reviewed_export(
        &self,
        destination: &Path,
        plan: &ReviewedExportPlan,
    ) -> Result<PreparedExport, sqlx::Error> {
        let parent = export_destination_parent(destination)?;
        let mut transaction = self.pool.begin().await?;
        let migration_sequence = verify_export_schema(&mut transaction).await?;
        let selection = ReviewedExportSelection {
            include_protected_records: plan.info.protected_records_included,
        };
        let (counts, protected_answer_count, protected_draft_count) =
            export_counts(&mut transaction, selection).await?;
        if migration_sequence != plan.migration_sequence
            || counts != plan.info.section_counts
            || protected_answer_count != plan.info.protected_application_answer_count
            || protected_draft_count != plan.info.protected_resume_draft_count
        {
            return Err(protocol_error(
                "Reviewed export data changed; review it again before writing",
            ));
        }

        let mut temporary = tempfile::Builder::new()
            .prefix(".jobsentinel_export_")
            .tempfile_in(parent)
            .map_err(sqlx::Error::Io)?;
        jobsentinel_platform::ensure_private_file(temporary.path()).map_err(sqlx::Error::Io)?;
        let mut exported_records = 0_u64;
        {
            let mut writer = BufWriter::new(temporary.as_file_mut());
            write_json_line(&mut writer, &ExportHeader::from(&plan.info))?;
            for table in EXPORT_TABLES
                .iter()
                .filter(|table| table_selected(table, selection))
            {
                let query = export_query(table)?;
                let mut rows = sqlx::query(sqlx::AssertSqlSafe(query)).fetch(&mut *transaction);
                while let Some(row) = rows.try_next().await? {
                    let mut data: Value = serde_json::from_str(row.try_get(0)?)
                        .map_err(|_| protocol_error("Reviewed export row could not be encoded"))?;
                    sanitize_export_fields(&mut data, table, selection)?;
                    write_json_line(
                        &mut writer,
                        &ExportRecord {
                            kind: "record".to_string(),
                            section: table.section.to_string(),
                            table: table.table.to_string(),
                            data,
                        },
                    )?;
                    exported_records = exported_records
                        .checked_add(1)
                        .ok_or_else(|| protocol_error("Reviewed export record count overflowed"))?;
                }
            }
            if exported_records != plan.info.record_count {
                return Err(protocol_error(
                    "Reviewed export record count changed while writing",
                ));
            }
            write_json_line(&mut writer, &ExportComplete::from(&plan.info))?;
            writer.flush().map_err(sqlx::Error::Io)?;
        }
        temporary.as_file().sync_all().map_err(sqlx::Error::Io)?;
        transaction.rollback().await?;
        let inspected = Self::inspect_reviewed_export(temporary.path())?;
        if inspected != plan.info {
            return Err(protocol_error("Reviewed export verification failed"));
        }
        Ok(PreparedExport {
            temporary,
            info: inspected,
        })
    }

    pub fn inspect_reviewed_export(path: &Path) -> Result<ReviewedExportInfo, sqlx::Error> {
        inspect_reviewed_export_inner(path)
            .map_err(|_| protocol_error("Reviewed export could not be verified"))
    }

    async fn start_export_operation(&self, plan: &ReviewedExportPlan) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO v3_recovery_operations(
                operation_id, operation_kind, outcome,
                source_migration_sequence, target_migration_sequence
             ) VALUES (?, 'export', 'started', ?, ?)",
        )
        .bind(&plan.info.export_id)
        .bind(plan.migration_sequence)
        .bind(plan.migration_sequence)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn finish_export_operation(
        &self,
        operation_id: &str,
        error_kind: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        finish_export_operation(&self.pool, operation_id, error_kind).await
    }

    fn finish_export_operation_detached(&self, operation_id: &str) {
        let pool = self.pool.clone();
        let operation_id = operation_id.to_string();
        tokio::spawn(async move {
            if let Err(error) = finish_export_operation(&pool, &operation_id, None).await {
                tracing::warn!(
                    error_kind = crate::database_error_kind(&error),
                    "Reviewed export provenance update failed after publication"
                );
            }
        });
    }

    async fn fail_export_operation(&self, operation_id: &str, error: &sqlx::Error) {
        let _ = self
            .finish_export_operation(operation_id, Some(crate::database_error_kind(error)))
            .await;
    }
}

struct PreparedExport {
    temporary: NamedTempFile,
    info: ReviewedExportInfo,
}

impl PreparedExport {
    fn publish(self, destination: &Path) -> Result<ReviewedExportInfo, sqlx::Error> {
        let parent = export_destination_parent(destination)?;
        self.temporary
            .persist_noclobber(destination)
            .map_err(|error| sqlx::Error::Io(error.error))?;
        sync_parent_dir(parent);
        Ok(self.info)
    }
}

async fn verify_export_schema(connection: &mut SqliteConnection) -> Result<i64, sqlx::Error> {
    validate_export_tables()?;
    let migration_sequence: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM main._sqlx_migrations")
            .fetch_one(&mut *connection)
            .await?;
    let failed_migrations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM main._sqlx_migrations WHERE success = 0")
            .fetch_one(&mut *connection)
            .await?;
    let compiled_migration = MIGRATOR
        .iter()
        .filter(|migration| migration.migration_type.is_up_migration())
        .map(|migration| migration.version)
        .max()
        .unwrap_or_default();
    if migration_sequence != compiled_migration || failed_migrations != 0 {
        return Err(protocol_error(
            "Reviewed export requires the current compatible database schema",
        ));
    }

    let actual: BTreeSet<String> = sqlx::query_scalar(
        "SELECT name FROM main.sqlite_master
         WHERE type = 'table' AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\'
         ORDER BY name",
    )
    .fetch_all(&mut *connection)
    .await?
    .into_iter()
    .collect();
    let allowed: BTreeSet<&str> = EXPORT_TABLES
        .iter()
        .map(|table| table.table)
        .chain(EXCLUDED_TABLES.iter().copied())
        .collect();
    if actual.iter().any(|table| !allowed.contains(table.as_str())) {
        tracing::warn!("Reviewed export refused an unsupported table");
        return Err(protocol_error("Reviewed export found an unsupported table"));
    }
    if let Some(table) = EXPORT_TABLES
        .iter()
        .map(|table| table.table)
        .find(|table| !actual.contains(*table))
    {
        tracing::warn!(table, "Reviewed export required table is missing");
        return Err(protocol_error("Reviewed export schema is incomplete"));
    }
    Ok(migration_sequence)
}

async fn export_counts(
    connection: &mut SqliteConnection,
    selection: ReviewedExportSelection,
) -> Result<(BTreeMap<String, u64>, u64, u64), sqlx::Error> {
    let mut counts = BTreeMap::new();
    let mut protected_application_answer_count = 0_u64;
    let mut protected_resume_draft_count = 0_u64;
    for table in EXPORT_TABLES {
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(count_query(table)?))
            .fetch_one(&mut *connection)
            .await?;
        let count = u64::try_from(count)
            .map_err(|_| protocol_error("Reviewed export found an invalid record count"))?;
        if table.protected {
            protected_application_answer_count = protected_application_answer_count
                .checked_add(count)
                .ok_or_else(|| protocol_error("Reviewed export record count overflowed"))?;
        }
        if table.protected_json {
            protected_resume_draft_count = protected_resume_draft_count
                .checked_add(count)
                .ok_or_else(|| protocol_error("Reviewed export record count overflowed"))?;
        }
        if table_selected(table, selection) {
            let section_count = counts.entry(table.section.to_string()).or_insert(0_u64);
            *section_count = section_count
                .checked_add(count)
                .ok_or_else(|| protocol_error("Reviewed export record count overflowed"))?;
        }
    }
    Ok((
        counts,
        protected_application_answer_count,
        protected_resume_draft_count,
    ))
}

pub(super) fn table_selected(table: &ExportTable, selection: ReviewedExportSelection) -> bool {
    !table.protected || selection.include_protected_records
}

async fn finish_export_operation(
    pool: &SqlitePool,
    operation_id: &str,
    error_kind: Option<&str>,
) -> Result<(), sqlx::Error> {
    if finish_recovery_operation_record(pool, operation_id, error_kind).await? {
        Ok(())
    } else {
        Err(protocol_error("Reviewed export provenance update failed"))
    }
}

fn write_json_line(writer: &mut impl Write, value: &impl Serialize) -> Result<(), sqlx::Error> {
    serde_json::to_writer(&mut *writer, value).map_err(|error| {
        error.io_error_kind().map_or_else(
            || protocol_error("Reviewed export could not be encoded"),
            |kind| {
                sqlx::Error::Io(std::io::Error::new(
                    kind,
                    "Reviewed export could not be written",
                ))
            },
        )
    })?;
    writer.write_all(b"\n").map_err(sqlx::Error::Io)
}

pub(super) fn sum_counts(counts: &BTreeMap<String, u64>) -> Result<u64, sqlx::Error> {
    counts.values().try_fold(0_u64, |total, count| {
        total
            .checked_add(*count)
            .ok_or_else(|| protocol_error("Reviewed export record count overflowed"))
    })
}

fn export_destination_parent(destination: &Path) -> Result<&Path, sqlx::Error> {
    if destination.exists() {
        return Err(sqlx::Error::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Reviewed export destination already exists",
        )));
    }
    destination
        .parent()
        .filter(|parent| parent.is_dir())
        .ok_or_else(|| {
            sqlx::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Reviewed export destination directory does not exist",
            ))
        })
}

pub(super) fn protocol_error(message: &str) -> sqlx::Error {
    sqlx::Error::Protocol(message.to_string())
}

#[cfg(unix)]
fn sync_parent_dir(parent: &Path) {
    let _ = File::open(parent).and_then(|directory| directory.sync_all());
}

#[cfg(not(unix))]
fn sync_parent_dir(_parent: &Path) {}

#[cfg(test)]
mod encoding_tests {
    use super::*;

    struct FullDevice;

    impl Write for FullDevice {
        fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::StorageFull))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn json_writer_errors_remain_io_errors() {
        let error = write_json_line(&mut FullDevice, &0).unwrap_err();
        assert!(matches!(error, sqlx::Error::Io(error)
            if error.kind() == std::io::ErrorKind::StorageFull));
    }
}
