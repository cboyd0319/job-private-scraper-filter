use super::{
    reviewed_export::{
        protocol_error, sum_counts, table_selected, ReviewedExportInfo, ReviewedExportSelection,
        EXPORT_FORMAT_VERSION, EXPORT_SCHEMA, MAX_EXPORT_LINE_BYTES,
    },
    reviewed_export_sanitize::sanitize_export_fields,
    reviewed_export_schema::EXPORT_TABLES,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportHeader {
    pub(super) kind: String,
    pub(super) schema: String,
    pub(super) export_id: String,
    pub(super) created_at: String,
    pub(super) format_version: u32,
    pub(super) protection: String,
    pub(super) user_review_required: bool,
    pub(super) contains_secrets: bool,
    pub(super) secret_scope: String,
    pub(super) user_authored_text_is_verbatim: bool,
    pub(super) integrity: String,
    pub(super) protected_records_included: bool,
    pub(super) protected_application_answer_count: u64,
    pub(super) protected_resume_draft_count: u64,
    pub(super) record_count: u64,
    pub(super) section_counts: BTreeMap<String, u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportRecord {
    pub(super) kind: String,
    pub(super) section: String,
    pub(super) table: String,
    pub(super) data: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportComplete {
    pub(super) kind: String,
    pub(super) export_id: String,
    pub(super) record_count: u64,
    pub(super) section_counts: BTreeMap<String, u64>,
}

impl From<&ReviewedExportInfo> for ExportHeader {
    fn from(info: &ReviewedExportInfo) -> Self {
        Self {
            kind: "export".to_string(),
            schema: EXPORT_SCHEMA.to_string(),
            export_id: info.export_id.clone(),
            created_at: info.created_at.clone(),
            format_version: info.format_version,
            protection: "reviewed_plaintext".to_string(),
            user_review_required: true,
            contains_secrets: false,
            secret_scope: "jobsentinel_managed_credentials".to_string(),
            user_authored_text_is_verbatim: true,
            integrity: "structural_completion_only".to_string(),
            protected_records_included: info.protected_records_included,
            protected_application_answer_count: info.protected_application_answer_count,
            protected_resume_draft_count: info.protected_resume_draft_count,
            record_count: info.record_count,
            section_counts: info.section_counts.clone(),
        }
    }
}

impl From<&ReviewedExportInfo> for ExportComplete {
    fn from(info: &ReviewedExportInfo) -> Self {
        Self {
            kind: "complete".to_string(),
            export_id: info.export_id.clone(),
            record_count: info.record_count,
            section_counts: info.section_counts.clone(),
        }
    }
}

pub(super) fn inspect_reviewed_export_inner(
    path: &Path,
) -> Result<ReviewedExportInfo, sqlx::Error> {
    let mut reader = BufReader::new(File::open(path).map_err(sqlx::Error::Io)?);
    let header: ExportHeader = serde_json::from_str(
        &read_export_line(&mut reader)?
            .ok_or_else(|| protocol_error("Reviewed export is empty"))?,
    )
    .map_err(|_| protocol_error("Reviewed export header is invalid"))?;
    validate_export_header(&header)?;
    let selection = ReviewedExportSelection {
        include_protected_records: header.protected_records_included,
    };
    let expected_sections: BTreeSet<&str> = EXPORT_TABLES
        .iter()
        .filter(|table| table_selected(table, selection))
        .map(|table| table.section)
        .collect();
    if header
        .section_counts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected_sections
    {
        return Err(protocol_error("Reviewed export sections are invalid"));
    }
    let mut section_counts = header
        .section_counts
        .keys()
        .map(|section| (section.clone(), 0_u64))
        .collect::<BTreeMap<_, _>>();
    let mut record_count = 0_u64;
    let mut complete = None;
    while let Some(line) = read_export_line(&mut reader)? {
        if complete.is_some() {
            return Err(protocol_error(
                "Reviewed export has data after its completion record",
            ));
        }
        let value: Value = serde_json::from_str(&line)
            .map_err(|_| protocol_error("Reviewed export record is invalid"))?;
        match value.get("kind").and_then(Value::as_str) {
            Some("record") => {
                let record: ExportRecord = serde_json::from_value(value)
                    .map_err(|_| protocol_error("Reviewed export record is invalid"))?;
                validate_export_record(&record, selection)?;
                *section_counts
                    .get_mut(&record.section)
                    .ok_or_else(|| protocol_error("Reviewed export section is invalid"))? += 1;
                record_count = record_count
                    .checked_add(1)
                    .ok_or_else(|| protocol_error("Reviewed export record count overflowed"))?;
            }
            Some("complete") => {
                complete = Some(
                    serde_json::from_value::<ExportComplete>(value)
                        .map_err(|_| protocol_error("Reviewed export completion is invalid"))?,
                );
            }
            _ => return Err(protocol_error("Reviewed export record kind is invalid")),
        }
    }
    let complete = complete
        .ok_or_else(|| protocol_error("Reviewed export is missing its completion record"))?;
    if complete.kind != "complete"
        || complete.export_id != header.export_id
        || complete.record_count != record_count
        || complete.section_counts != section_counts
        || header.record_count != record_count
        || header.section_counts != section_counts
    {
        return Err(protocol_error("Reviewed export completion does not match"));
    }
    Ok(ReviewedExportInfo {
        export_id: header.export_id,
        created_at: header.created_at,
        format_version: header.format_version,
        record_count,
        section_counts,
        protected_records_included: header.protected_records_included,
        protected_application_answer_count: header.protected_application_answer_count,
        protected_resume_draft_count: header.protected_resume_draft_count,
    })
}

fn validate_export_header(header: &ExportHeader) -> Result<(), sqlx::Error> {
    if header.kind != "export"
        || header.schema != EXPORT_SCHEMA
        || header.format_version != EXPORT_FORMAT_VERSION
        || header.protection != "reviewed_plaintext"
        || !header.user_review_required
        || header.contains_secrets
        || header.secret_scope != "jobsentinel_managed_credentials"
        || !header.user_authored_text_is_verbatim
        || header.integrity != "structural_completion_only"
        || Uuid::parse_str(&header.export_id)
            .map(|id| id.to_string() != header.export_id)
            .unwrap_or(true)
        || sum_counts(&header.section_counts)? != header.record_count
    {
        return Err(protocol_error("Reviewed export header is incompatible"));
    }
    Ok(())
}

fn validate_export_record(
    record: &ExportRecord,
    selection: ReviewedExportSelection,
) -> Result<(), sqlx::Error> {
    let table = EXPORT_TABLES
        .iter()
        .find(|table| {
            table.section == record.section
                && table.table == record.table
                && table_selected(table, selection)
        })
        .ok_or_else(|| protocol_error("Reviewed export table is invalid"))?;
    let data = record
        .data
        .as_object()
        .ok_or_else(|| protocol_error("Reviewed export row is invalid"))?;
    let expected: BTreeSet<&str> = table.columns.split(',').collect();
    if data.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err(protocol_error("Reviewed export row fields are invalid"));
    }
    let mut canonical = record.data.clone();
    sanitize_export_fields(&mut canonical, table, selection)?;
    if canonical != record.data {
        return Err(protocol_error("Reviewed export row is not canonical"));
    }
    Ok(())
}

fn read_export_line(reader: &mut impl BufRead) -> Result<Option<String>, sqlx::Error> {
    let mut bytes = Vec::new();
    let read = reader
        .take((MAX_EXPORT_LINE_BYTES + 1) as u64)
        .read_until(b'\n', &mut bytes)
        .map_err(sqlx::Error::Io)?;
    if read == 0 {
        return Ok(None);
    }
    if read > MAX_EXPORT_LINE_BYTES || !bytes.ends_with(b"\n") {
        return Err(protocol_error(
            "Reviewed export line is incomplete or too large",
        ));
    }
    bytes.pop();
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| protocol_error("Reviewed export is not UTF-8"))
}
