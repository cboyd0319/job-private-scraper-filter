use super::{DraftEducation, DraftExperience, DraftSkill, ResumeDraft};
use crate::resume::builder::deserialization::{optional_string_from_value, string_vec_from_value};
use chrono::{DateTime, Utc};
use jobsentinel_documents::{
    ResumeCertification, ResumeEducation, ResumePersonalInfo, ResumeProject, ResumeSkillCategory,
    StructuredResume,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
struct ResumeDraftWire {
    id: i64,
    contact: ResumePersonalInfo,
    summary: String,
    experience: Vec<DraftExperience>,
    education: Vec<DraftEducationWire>,
    skills: Vec<DraftSkill>,
    certifications: Vec<ResumeCertification>,
    projects: Vec<ResumeProject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    clearance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    military_info: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
struct DraftEducationWire {
    #[serde(default)]
    id: i64,
    institution: String,
    degree: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    field_of_study: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(
        default,
        alias = "graduation_year",
        deserialize_with = "optional_string_from_value"
    )]
    graduation_date: Option<String>,
    #[serde(default, deserialize_with = "optional_string_from_value")]
    gpa: Option<String>,
    #[serde(default, deserialize_with = "string_vec_from_value")]
    honors: Vec<String>,
}

impl From<DraftEducation> for DraftEducationWire {
    fn from(value: DraftEducation) -> Self {
        let education = value.education;
        Self {
            id: value.id,
            institution: education.institution,
            degree: education.degree,
            field_of_study: education.field_of_study,
            location: education.location,
            graduation_date: education.graduation_date,
            gpa: education.gpa,
            honors: education.honors,
        }
    }
}

impl From<DraftEducationWire> for DraftEducation {
    fn from(value: DraftEducationWire) -> Self {
        Self {
            id: value.id,
            education: ResumeEducation {
                institution: value.institution,
                degree: value.degree,
                field_of_study: value.field_of_study,
                location: value.location,
                graduation_date: value.graduation_date,
                gpa: value.gpa,
                honors: value.honors,
            },
        }
    }
}

impl Serialize for DraftEducation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        DraftEducationWire::from(self.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DraftEducation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        DraftEducationWire::deserialize(deserializer).map(Self::from)
    }
}

impl ResumeDraft {
    fn from_wire(wire: ResumeDraftWire) -> Self {
        let experience_ids = wire.experience.iter().map(|entry| entry.id).collect();
        let education_ids = wire.education.iter().map(|entry| entry.id).collect();
        Self {
            id: wire.id,
            resume: StructuredResume {
                personal: wire.contact,
                summary: Some(wire.summary),
                experience: wire
                    .experience
                    .into_iter()
                    .map(|entry| entry.experience)
                    .collect(),
                education: wire
                    .education
                    .into_iter()
                    .map(DraftEducation::from)
                    .map(|entry| entry.education)
                    .collect(),
                skills: group_skills(wire.skills),
                certifications: wire.certifications,
                projects: wire.projects,
                clearance: wire.clearance,
                military_info: wire.military_info,
            },
            experience_ids,
            education_ids,
            created_at: wire.created_at,
            updated_at: wire.updated_at,
        }
    }

    fn to_wire(&self) -> ResumeDraftWire {
        ResumeDraftWire {
            id: self.id,
            contact: self.resume.personal.clone(),
            summary: self.resume.summary.clone().unwrap_or_default(),
            experience: self
                .resume
                .experience
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, experience)| DraftExperience {
                    id: self.experience_ids.get(index).copied().unwrap_or_default(),
                    experience,
                })
                .collect(),
            education: self
                .resume
                .education
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, education)| DraftEducation {
                    id: self.education_ids.get(index).copied().unwrap_or_default(),
                    education,
                })
                .map(DraftEducationWire::from)
                .collect(),
            skills: flatten_skills(&self.resume.skills),
            certifications: self.resume.certifications.clone(),
            projects: self.resume.projects.clone(),
            clearance: self.resume.clearance.clone(),
            military_info: self.resume.military_info.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl Serialize for ResumeDraft {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_wire().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ResumeDraft {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ResumeDraftWire::deserialize(deserializer).map(Self::from_wire)
    }
}

pub(super) fn group_skills(skills: Vec<DraftSkill>) -> Vec<ResumeSkillCategory> {
    let mut categories: Vec<ResumeSkillCategory> = Vec::new();
    for mut entry in skills {
        entry.skill.proficiency = entry
            .skill
            .proficiency
            .map(|value| value.to_ascii_lowercase());
        if let Some(category) = categories
            .iter_mut()
            .find(|category| category.name == entry.category)
        {
            category.skills.push(entry.skill);
        } else {
            categories.push(ResumeSkillCategory {
                name: entry.category,
                skills: vec![entry.skill],
            });
        }
    }
    categories
}

fn flatten_skills(categories: &[ResumeSkillCategory]) -> Vec<DraftSkill> {
    categories
        .iter()
        .flat_map(|category| {
            category.skills.iter().cloned().map(|skill| DraftSkill {
                category: category.name.clone(),
                skill,
            })
        })
        .collect()
}
