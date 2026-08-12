// Matches ATS keywords against plain-text resume evidence.

use chrono::{Datelike, Utc};
use jobsentinel_domain::{ResumeEvidenceCitation, ResumeEvidenceSnapshot};

use super::super::ats_types::{
    KeywordImportance, KeywordMatch, MissingKeyword, RegionalMatchingProfile,
};
use super::super::format_taxonomy::resume_format_taxonomy;
use super::super::structured_resume::ResumeExperience;
use super::{term_expansion, AtsAnalyzer};

mod structured;

pub(super) struct MatchedKeyword {
    pub keyword_match: KeywordMatch,
    pub evidence_citations: Vec<ResumeEvidenceCitation>,
}

impl AtsAnalyzer {
    pub(super) fn find_keyword_matches_in_text(
        resume_text: &str,
        skills: &[String],
        job_keywords: &[(String, KeywordImportance)],
        evidence_snapshot: Option<&ResumeEvidenceSnapshot>,
        region: Option<RegionalMatchingProfile>,
    ) -> (Vec<MatchedKeyword>, Vec<MissingKeyword>) {
        let mut matches = Vec::new();
        let mut missing = Vec::new();

        for (keyword, importance) in job_keywords {
            let keyword_lower = keyword.to_lowercase();
            let search_terms =
                term_expansion::conservative_keyword_search_terms(&keyword_lower, region);
            let (mut found_in, mut frequency, mut evidence_citations) =
                Self::plain_text_search_term_hits(resume_text, &search_terms, evidence_snapshot);

            for (index, skill) in skills.iter().enumerate() {
                let skill_lower = skill.to_lowercase();
                if search_terms.iter().any(|term| {
                    Self::keyword_appears_in_text(&skill_lower, term)
                        || Self::keyword_appears_in_text(term, &skill_lower)
                }) {
                    Self::add_evidence_section(&mut found_in, "skills");
                    if let Some(snapshot) = evidence_snapshot {
                        if let Some(citation) =
                            ResumeEvidenceCitation::for_field(snapshot, &format!("skills.{index}"))
                        {
                            evidence_citations.push(citation);
                        }
                    }
                    frequency += 1;
                }
            }

            record_keyword_result(
                keyword,
                *importance,
                found_in,
                frequency,
                evidence_citations,
                &mut matches,
                &mut missing,
            );
        }

        sort_keyword_matches(&mut matches);
        sort_missing_keywords(&mut missing);

        (matches, missing)
    }

    fn plain_text_search_term_hits(
        resume_text: &str,
        search_terms: &[String],
        evidence_snapshot: Option<&ResumeEvidenceSnapshot>,
    ) -> (Vec<String>, usize, Vec<ResumeEvidenceCitation>) {
        let mut found_in = Vec::new();
        let mut frequency = 0;
        let mut evidence_citations = Vec::new();
        let mut current_section = "resume text";
        let mut current_section_is_military = false;
        let mut current_experience_is_current = false;
        let mut current_experience_is_recent = false;

        for (line_index, line) in resume_text.lines().enumerate() {
            if let Some(section) = Self::plain_text_section_label(line) {
                current_section = section;
                current_section_is_military =
                    section == "experience" && line.to_ascii_lowercase().contains("military");
                current_experience_is_current = false;
                current_experience_is_recent = false;
            }

            let line_lower = line.to_lowercase();
            if current_section == "experience" {
                if Self::plain_text_current_experience_marker(&line_lower) {
                    current_experience_is_current = true;
                    current_experience_is_recent = false;
                } else if Self::plain_text_recent_experience_marker(&line_lower) {
                    current_experience_is_current = false;
                    current_experience_is_recent = true;
                } else if Self::plain_text_past_experience_marker(&line_lower) {
                    current_experience_is_current = false;
                    current_experience_is_recent = false;
                }
            }

            let count = Self::keyword_frequency_for_search_terms(&line_lower, search_terms);
            if count == 0 {
                continue;
            }

            let evidence_section =
                if current_section == "experience" && current_experience_is_current {
                    "current experience"
                } else if current_section == "experience" && current_experience_is_recent {
                    "recent experience"
                } else {
                    current_section
                };
            Self::add_evidence_section(&mut found_in, evidence_section);
            if current_section_is_military {
                Self::add_evidence_section(&mut found_in, "military service");
            }
            if let Some(snapshot) = evidence_snapshot {
                if let Some(citation) = ResumeEvidenceCitation::for_field(
                    snapshot,
                    &format!("resume_text.{line_index}"),
                ) {
                    evidence_citations.push(citation);
                }
            }
            frequency +=
                Self::evidence_strength_adjusted_count(count, &line_lower, evidence_section);
        }

        (found_in, frequency, evidence_citations)
    }

    fn evidence_strength_adjusted_count(
        count: usize,
        text_lower: &str,
        evidence_section: &str,
    ) -> usize {
        if count == 0 {
            return 0;
        }
        if evidence_section == "current experience" || evidence_section == "recent experience" {
            return count + 1;
        }
        let can_show_work_evidence = matches!(
            evidence_section,
            "experience" | "current experience" | "recent experience" | "projects"
        );
        if can_show_work_evidence
            && (Self::metric_backed_evidence_marker(text_lower)
                || Self::scope_backed_evidence_marker(text_lower)
                || Self::responsibility_backed_evidence_marker(text_lower)
                || Self::duty_backed_evidence_marker(text_lower))
        {
            count + 1
        } else {
            count
        }
    }

    fn metric_backed_evidence_marker(text_lower: &str) -> bool {
        regex::Regex::new(
            r"(?:\b\d+(?:\.\d+)?\s*(?:%|(?:percent|clients?|customers?|cases?|tickets?|orders?|projects?|reports?|days?|weeks?|months?)\b)|\$\s*\d)",
        )
        .unwrap()
        .is_match(text_lower)
    }

    fn scope_backed_evidence_marker(text_lower: &str) -> bool {
        regex::Regex::new(
            r"\bacross\s+(?:[a-z]+\s+){0,5}(?:teams?|departments?|locations?|sites?|regions?|markets?|service\s+lines?)\b",
        )
        .unwrap()
        .is_match(text_lower)
    }

    fn responsibility_backed_evidence_marker(text_lower: &str) -> bool {
        regex::Regex::new(
            r"\b(?:owned|managed|administered|developed|implemented|improved|operated)\b.+\b(?:workflows?|process(?:es)?|programs?|operations?|intake|cases?|systems?|tools?)\b",
        )
        .unwrap()
        .is_match(text_lower)
    }

    fn duty_backed_evidence_marker(text_lower: &str) -> bool {
        regex::Regex::new(
            r"\b(?:coordinated|processed|maintained|tracked|reviewed|prepared|scheduled|organized|documented|responded|resolved|updated|served|followed\s+up|followed-up)\b.+\b(?:requests?|appointments?|records?|orders?|cases?|tickets?|reports?|files?|forms?|calls?|emails?|inquiries|intake|follow[-\s]?ups?|tasks?|schedules?)\b",
        )
        .unwrap()
        .is_match(text_lower)
    }

    fn plain_text_current_experience_marker(line_lower: &str) -> bool {
        line_lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == "present")
    }

    fn structured_experience_evidence_section(exp: &ResumeExperience) -> &'static str {
        if exp.is_current
            || exp
                .end_date
                .as_deref()
                .is_some_and(|date| date.trim().eq_ignore_ascii_case("present"))
        {
            return "current experience";
        }
        if exp
            .end_date
            .as_deref()
            .is_some_and(Self::recent_end_year_marker)
        {
            return "recent experience";
        }
        "experience"
    }

    fn plain_text_recent_experience_marker(line_lower: &str) -> bool {
        !Self::plain_text_current_experience_marker(line_lower)
            && Self::recent_end_year_marker(line_lower)
    }

    fn plain_text_past_experience_marker(line_lower: &str) -> bool {
        !Self::plain_text_current_experience_marker(line_lower)
            && regex::Regex::new(r"\b(?:19|20)\d{2}\s*(?:-|to)\s*(?:19|20)\d{2}\b")
                .unwrap()
                .is_match(line_lower)
    }

    fn recent_end_year_marker(text: &str) -> bool {
        let Some(end_year) = regex::Regex::new(r"\b(?:19|20)\d{2}\b")
            .unwrap()
            .find_iter(text)
            .last()
            .and_then(|year| year.as_str().parse::<i32>().ok())
        else {
            return false;
        };

        let current_year = Utc::now().year();
        end_year >= current_year - 1 && end_year <= current_year
    }

    fn keyword_frequency_for_search_terms(text: &str, search_terms: &[String]) -> usize {
        search_terms
            .iter()
            .map(|term| Self::keyword_frequency(text, term))
            .max()
            .unwrap_or(0)
    }

    fn add_evidence_section(found_in: &mut Vec<String>, section: &str) {
        if !found_in.iter().any(|existing| existing == section) {
            found_in.push(section.to_string());
        }
    }

    pub(super) fn plain_text_section_label(line: &str) -> Option<&'static str> {
        let normalized = line
            .trim()
            .trim_start_matches(|c: char| c == '-' || c == '*' || c == '•')
            .trim_start()
            .trim_end_matches(':')
            .to_lowercase()
            .replace('/', " ")
            .replace('&', " and ");
        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

        resume_format_taxonomy()
            .plain_text_section_aliases
            .iter()
            .find_map(|alias| {
                alias
                    .headings
                    .iter()
                    .any(|heading| Self::line_starts_with_heading(&normalized, heading))
                    .then_some(alias.section.as_str())
            })
    }

    fn keyword_frequency(text: &str, keyword: &str) -> usize {
        if keyword.trim().is_empty() {
            return 0;
        }

        text.match_indices(keyword)
            .filter(|(start, _)| Self::keyword_match_has_boundaries(text, keyword, *start))
            .count()
    }

    pub(super) fn keyword_appears_in_text(text: &str, keyword: &str) -> bool {
        Self::keyword_frequency(text, keyword) > 0
    }

    fn keyword_match_has_boundaries(text: &str, keyword: &str, start: usize) -> bool {
        let end = start + keyword.len();
        let before_is_term = text[..start]
            .chars()
            .next_back()
            .is_some_and(Self::is_keyword_term_char);
        let after_is_term = text[end..]
            .chars()
            .next()
            .is_some_and(Self::is_keyword_term_char);

        !before_is_term && !after_is_term
    }

    fn is_keyword_term_char(ch: char) -> bool {
        ch.is_alphanumeric() || matches!(ch, '#' | '+')
    }

    pub(super) fn extract_section(text: &str, headers: &[&str]) -> String {
        for header in headers {
            if let Some(start) = text.find(header) {
                let after = &text[start..];
                let blank_line_end = after.find("\n\n").map(|i| i + start).unwrap_or(text.len());
                let heading_end = Self::find_next_section_heading(after, headers)
                    .map(|i| i + start)
                    .unwrap_or(text.len());
                let end = blank_line_end.min(heading_end);
                return text[start..end].to_string();
            }
        }
        String::new()
    }

    fn find_next_section_heading(section_text: &str, current_headers: &[&str]) -> Option<usize> {
        for (offset, _) in section_text.match_indices('\n').skip(1) {
            let line = &section_text[offset + 1..];
            let trimmed = line.trim_start_matches(|c: char| {
                c.is_whitespace() || c == '-' || c == '*' || c == '•'
            });

            if Self::SECTION_BOUNDARY_HEADERS
                .iter()
                .filter(|boundary| !current_headers.contains(boundary))
                .any(|boundary| Self::line_starts_with_heading(trimmed, boundary))
            {
                return Some(offset);
            }
        }

        None
    }

    fn line_starts_with_heading(line: &str, heading: &str) -> bool {
        let Some(rest) = line.strip_prefix(heading) else {
            return false;
        };

        rest.is_empty()
            || rest.starts_with(':')
            || rest.starts_with('-')
            || rest.starts_with(' ')
            || rest.starts_with('\t')
    }
}

fn keyword_importance_order(importance: KeywordImportance) -> usize {
    match importance {
        KeywordImportance::Required => 0,
        KeywordImportance::Preferred => 1,
        KeywordImportance::Industry => 2,
    }
}

fn record_keyword_result(
    keyword: &str,
    importance: KeywordImportance,
    found_in: Vec<String>,
    frequency: usize,
    evidence_citations: Vec<ResumeEvidenceCitation>,
    matches: &mut Vec<MatchedKeyword>,
    missing: &mut Vec<MissingKeyword>,
) {
    if frequency > 0 {
        matches.push(MatchedKeyword {
            keyword_match: KeywordMatch {
                keyword: keyword.to_string(),
                found_in,
                frequency,
                importance,
            },
            evidence_citations,
        });
    } else {
        missing.push(MissingKeyword {
            keyword: keyword.to_string(),
            importance,
        });
    }
}

fn sort_keyword_matches(matches: &mut [MatchedKeyword]) {
    matches.sort_by(|a, b| {
        keyword_importance_order(a.keyword_match.importance)
            .cmp(&keyword_importance_order(b.keyword_match.importance))
            .then(b.keyword_match.frequency.cmp(&a.keyword_match.frequency))
    });
}

fn sort_missing_keywords(missing: &mut [MissingKeyword]) {
    missing.sort_by(|a, b| {
        keyword_importance_order(a.importance)
            .cmp(&keyword_importance_order(b.importance))
            .then(a.keyword.cmp(&b.keyword))
    });
}
