//! Type definitions for Application Tracking System

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Application status in the job search pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationStatus {
    ToApply,
    Applied,
    ScreeningCall,
    PhoneInterview,
    TechnicalInterview,
    OnsiteInterview,
    OfferReceived,
    OfferAccepted,
    OfferRejected,
    Rejected,
    Ghosted,
    Withdrawn,
}

impl fmt::Display for ApplicationStatus {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ToApply => "to_apply",
            Self::Applied => "applied",
            Self::ScreeningCall => "screening_call",
            Self::PhoneInterview => "phone_interview",
            Self::TechnicalInterview => "technical_interview",
            Self::OnsiteInterview => "onsite_interview",
            Self::OfferReceived => "offer_received",
            Self::OfferAccepted => "offer_accepted",
            Self::OfferRejected => "offer_rejected",
            Self::Rejected => "rejected",
            Self::Ghosted => "ghosted",
            Self::Withdrawn => "withdrawn",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for ApplicationStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "to_apply" => Ok(Self::ToApply),
            "applied" => Ok(Self::Applied),
            "screening_call" => Ok(Self::ScreeningCall),
            "phone_interview" => Ok(Self::PhoneInterview),
            "technical_interview" => Ok(Self::TechnicalInterview),
            "onsite_interview" => Ok(Self::OnsiteInterview),
            "offer_received" => Ok(Self::OfferReceived),
            "offer_accepted" => Ok(Self::OfferAccepted),
            "offer_rejected" => Ok(Self::OfferRejected),
            "rejected" => Ok(Self::Rejected),
            "ghosted" => Ok(Self::Ghosted),
            "withdrawn" => Ok(Self::Withdrawn),
            _ => Err(anyhow!("Invalid status: {}", s)),
        }
    }
}

/// Application data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: i64,
    pub job_hash: String,
    pub status: ApplicationStatus,
    pub applied_at: Option<DateTime<Utc>>,
    pub last_contact: Option<DateTime<Utc>>,
    pub next_followup: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub recruiter_name: Option<String>,
    pub recruiter_email: Option<String>,
    pub recruiter_phone: Option<String>,
    pub salary_expectation: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Applications grouped by status for Kanban board
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ApplicationsByStatus {
    pub to_apply: Vec<ApplicationWithJob>,
    pub applied: Vec<ApplicationWithJob>,
    pub screening_call: Vec<ApplicationWithJob>,
    pub phone_interview: Vec<ApplicationWithJob>,
    pub technical_interview: Vec<ApplicationWithJob>,
    pub onsite_interview: Vec<ApplicationWithJob>,
    pub offer_received: Vec<ApplicationWithJob>,
    pub offer_accepted: Vec<ApplicationWithJob>,
    pub offer_rejected: Vec<ApplicationWithJob>,
    pub rejected: Vec<ApplicationWithJob>,
    pub ghosted: Vec<ApplicationWithJob>,
    pub withdrawn: Vec<ApplicationWithJob>,
}

/// Application with job details (for Kanban display)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationWithJob {
    pub id: i64,
    pub job_hash: String,
    pub status: String,
    pub applied_at: Option<String>,
    pub last_contact: Option<String>,
    pub notes: Option<String>,
    pub job_title: String,
    pub company: String,
    pub score: f64,
}

/// Application statistics for analytics dashboard
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ApplicationStats {
    pub total: i32,
    pub by_status: StatusCounts,
    pub response_rate: f64,
    pub offer_rate: f64,
    pub weekly_applications: Vec<WeeklyData>,
}

/// Counts by status
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StatusCounts {
    pub to_apply: i32,
    pub applied: i32,
    pub screening_call: i32,
    pub phone_interview: i32,
    pub technical_interview: i32,
    pub onsite_interview: i32,
    pub offer_received: i32,
    pub offer_accepted: i32,
    pub offer_rejected: i32,
    pub rejected: i32,
    pub ghosted: i32,
    pub withdrawn: i32,
}

/// Weekly application data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyData {
    pub week: String,
    pub count: i32,
}

/// Interview types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterviewType {
    Phone,
    Screening,
    Technical,
    Behavioral,
    Onsite,
    Final,
    Other,
}

impl fmt::Display for InterviewType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterviewType::Phone => write!(f, "phone"),
            InterviewType::Screening => write!(f, "screening"),
            InterviewType::Technical => write!(f, "technical"),
            InterviewType::Behavioral => write!(f, "behavioral"),
            InterviewType::Onsite => write!(f, "onsite"),
            InterviewType::Final => write!(f, "final"),
            InterviewType::Other => write!(f, "other"),
        }
    }
}

impl FromStr for InterviewType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "phone" => Ok(InterviewType::Phone),
            "screening" => Ok(InterviewType::Screening),
            "technical" => Ok(InterviewType::Technical),
            "behavioral" => Ok(InterviewType::Behavioral),
            "onsite" => Ok(InterviewType::Onsite),
            "final" => Ok(InterviewType::Final),
            _ => Ok(InterviewType::Other),
        }
    }
}

/// Interview with job details (for display)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterviewWithJob {
    pub id: i64,
    pub application_id: i64,
    pub job_hash: String,
    pub interview_type: String,
    pub scheduled_at: String,
    pub duration_minutes: i32,
    pub location: Option<String>,
    pub interviewer_name: Option<String>,
    pub interviewer_title: Option<String>,
    pub notes: Option<String>,
    pub completed: bool,
    pub outcome: Option<String>,
    pub post_interview_notes: Option<String>,
    pub job_title: String,
    pub company: String,
}

/// Pending reminder with job details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingReminder {
    pub id: i64,
    pub application_id: i64,
    pub reminder_type: String,
    pub reminder_time: String,
    pub message: Option<String>,
    pub job_hash: String,
    pub job_title: String,
    pub company: String,
}
