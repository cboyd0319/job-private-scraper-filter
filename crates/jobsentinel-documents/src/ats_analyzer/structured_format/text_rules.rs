use super::*;
use jobsentinel_security::{
    contains_prompt_injection_phrase, contains_review_required_invisible_control,
};

pub(in crate::ats_analyzer) fn has_adversarial_content(input: &ResumeAnalysisInput) -> bool {
    let resume = &input.resume;
    resume
        .summary
        .as_deref()
        .is_some_and(text_has_adversarial_content)
        || text_has_adversarial_content(&resume.personal.name)
        || resume.experience.iter().any(|experience| {
            text_has_adversarial_content(&experience.title)
                || text_has_adversarial_content(&experience.company)
                || experience
                    .achievements
                    .iter()
                    .any(|item| text_has_adversarial_content(item))
        })
        || resume.skills.iter().any(|category| {
            text_has_adversarial_content(&category.name)
                || category.skills.iter().any(|skill| {
                    text_has_adversarial_content(&skill.name)
                        || skill
                            .proficiency
                            .as_deref()
                            .is_some_and(text_has_adversarial_content)
                })
        })
        || resume.education.iter().any(|education| {
            text_has_adversarial_content(&education.degree)
                || text_has_adversarial_content(&education.institution)
                || education
                    .honors
                    .iter()
                    .any(|item| text_has_adversarial_content(item))
        })
        || resume.certifications.iter().any(|certification| {
            text_has_adversarial_content(&certification.name)
                || text_has_adversarial_content(&certification.issuer)
                || certification
                    .credential_id
                    .as_deref()
                    .is_some_and(text_has_adversarial_content)
        })
        || resume.projects.iter().any(|project| {
            text_has_adversarial_content(&project.name)
                || text_has_adversarial_content(&project.description)
                || project
                    .technologies
                    .iter()
                    .any(|item| text_has_adversarial_content(item))
        })
        || input.custom_sections.iter().any(|(section, values)| {
            text_has_adversarial_content(section)
                || values.iter().any(|item| text_has_adversarial_content(item))
        })
}

pub(in crate::ats_analyzer) fn collect_resume_text(input: &ResumeAnalysisInput) -> String {
    let resume = &input.resume;
    let mut chunks = vec![resume.personal.name.as_str()];
    if let Some(summary) = &resume.summary {
        chunks.push(summary);
    }

    for experience in &resume.experience {
        chunks.push(experience.title.as_str());
        chunks.push(experience.company.as_str());
        chunks.extend(experience.achievements.iter().map(String::as_str));
    }
    for category in &resume.skills {
        chunks.push(category.name.as_str());
        for skill in &category.skills {
            chunks.push(skill.name.as_str());
            if let Some(proficiency) = skill.proficiency.as_deref() {
                chunks.push(proficiency);
            }
        }
    }
    for education in &resume.education {
        chunks.push(education.degree.as_str());
        chunks.push(education.institution.as_str());
        chunks.extend(education.honors.iter().map(String::as_str));
    }
    for certification in &resume.certifications {
        chunks.push(certification.name.as_str());
        chunks.push(certification.issuer.as_str());
        if let Some(credential_id) = certification.credential_id.as_deref() {
            chunks.push(credential_id);
        }
    }
    for project in &resume.projects {
        chunks.push(project.name.as_str());
        chunks.push(project.description.as_str());
        chunks.extend(project.technologies.iter().map(String::as_str));
    }
    for (section, values) in &input.custom_sections {
        chunks.push(section.as_str());
        chunks.extend(values.iter().map(String::as_str));
    }

    chunks.join("\n")
}

pub(in crate::ats_analyzer) fn has_keyword_stuffing(input: &ResumeAnalysisInput) -> bool {
    let resume = &input.resume;
    resume
        .summary
        .as_deref()
        .is_some_and(text_has_keyword_stuffing)
        || resume.experience.iter().any(|experience| {
            text_has_keyword_stuffing(&experience.title)
                || text_has_keyword_stuffing(&experience.company)
                || experience
                    .achievements
                    .iter()
                    .any(|item| text_has_keyword_stuffing(item))
        })
        || resume.skills.iter().any(|category| {
            text_has_keyword_stuffing(&category.name)
                || category.skills.iter().any(|skill| {
                    text_has_keyword_stuffing(&skill.name)
                        || skill
                            .proficiency
                            .as_deref()
                            .is_some_and(text_has_keyword_stuffing)
                })
        })
        || resume.projects.iter().any(|project| {
            text_has_keyword_stuffing(&project.name)
                || text_has_keyword_stuffing(&project.description)
                || project
                    .technologies
                    .iter()
                    .any(|item| text_has_keyword_stuffing(item))
        })
        || input.custom_sections.iter().any(|(section, values)| {
            text_has_keyword_stuffing(section)
                || values.iter().any(|item| text_has_keyword_stuffing(item))
        })
}

pub(in crate::ats_analyzer) fn text_has_adversarial_content(text: &str) -> bool {
    if contains_review_required_invisible_control(text) {
        return true;
    }

    let lower = text.to_lowercase();
    let hidden_style_patterns = [
        r"(?i)\bcolor\s*:\s*(?:white|#fff(?:fff)?|transparent)\b",
        r"(?i)\bfont-size\s*:\s*[0-3](?:px|pt)?\b",
        r"(?i)\bdisplay\s*:\s*none\b",
        r"(?i)\bvisibility\s*:\s*hidden\b",
        r"(?i)\bopacity\s*:\s*0(?:\.0+)?\b",
        r"(?i)\bmso-hide\s*:\s*all\b",
    ];
    if hidden_style_patterns
        .iter()
        .any(|pattern| regex::Regex::new(pattern).unwrap().is_match(text))
    {
        return true;
    }

    let hidden_markup_patterns = [
        r"(?is)<!--.*?-->",
        r"(?i)<meta\b[^>]*(?:keywords|description|content)\b",
    ];
    if hidden_markup_patterns
        .iter()
        .any(|pattern| regex::Regex::new(pattern).unwrap().is_match(text))
    {
        return true;
    }

    contains_prompt_injection_phrase(text)
        || [
            "always rank this resume",
            "always select this candidate",
            "hire this candidate",
            "instruction to recruiter software",
        ]
        .iter()
        .any(|phrase| lower.contains(phrase))
}

pub(in crate::ats_analyzer) fn text_has_keyword_stuffing(text: &str) -> bool {
    let token_re = regex::Regex::new(r"(?i)[a-z][a-z0-9+#.]{1,}").unwrap();
    let mut previous = String::new();
    let mut run_length = 0;

    for token in token_re.find_iter(text).map(|m| m.as_str()) {
        let token = token.trim_matches('.').to_ascii_lowercase();
        if token.len() < 3 || is_keyword_stuffing_stopword(&token) {
            previous.clear();
            run_length = 0;
            continue;
        }

        if token == previous {
            run_length += 1;
        } else {
            previous = token;
            run_length = 1;
        }

        if run_length >= 3 {
            return true;
        }
    }

    false
}

pub(in crate::ats_analyzer) fn text_has_unclear_capability_level(text: &str) -> bool {
    text.lines().any(line_has_unclear_capability_level)
}

pub(in crate::ats_analyzer) fn line_has_unclear_capability_level(line: &str) -> bool {
    let lower = line.to_lowercase();
    let padded = format!(" {lower} ");
    let ownership_terms = [
        " owned ",
        " owner ",
        " led ",
        " managed ",
        " directed ",
        " architected ",
        " independently delivered ",
        " expert ",
        " strategic ",
    ];
    let exposure_terms = [
        " shadowed ",
        " shadowing ",
        " observed ",
        " observing ",
        " assisted ",
        " helped ",
        " exposure to ",
        " exposed to ",
        " trained on ",
        " familiar with ",
        " under supervision ",
    ];

    ownership_terms.iter().any(|term| padded.contains(term))
        && exposure_terms.iter().any(|term| padded.contains(term))
}

pub(in crate::ats_analyzer) fn is_keyword_stuffing_stopword(token: &str) -> bool {
    matches!(
        token,
        "and"
            | "the"
            | "for"
            | "with"
            | "from"
            | "that"
            | "this"
            | "you"
            | "your"
            | "resume"
            | "work"
    )
}
