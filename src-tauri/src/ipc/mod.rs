//! Private Tauri command adapters grouped by product behavior.

pub(crate) mod ats;
pub(crate) mod automation;
pub(crate) mod bookmarklet;
pub(crate) mod cache;
pub(crate) mod config;
pub(crate) mod credentials;
pub(crate) mod deeplinks;
pub(crate) mod errors;
pub(crate) mod external_ai;
pub(crate) mod feedback;
pub(crate) mod geo;
pub(crate) mod ghost;
pub(crate) mod health;
pub(crate) mod import;
pub(crate) mod jobs;
pub(crate) mod limits;
pub(crate) mod linkedin_auth;
pub(crate) mod linkedin_workbench;
pub(crate) mod market;
pub(crate) mod native_file_drop;
pub(crate) mod opportunity_case;
pub(crate) mod pack_execution;
pub(crate) mod pack_management;
pub(crate) mod recovery;
pub(crate) mod resume;
mod resume_file_names;
pub(crate) mod salary;
pub(crate) mod scoring;
pub(crate) mod semantic_matching;
pub(crate) mod user_data;

#[cfg(feature = "embedded-ml")]
pub(crate) mod ml;

mod registry;

#[cfg(test)]
mod tests;

pub(crate) use registry::jobsentinel_command_handlers;
