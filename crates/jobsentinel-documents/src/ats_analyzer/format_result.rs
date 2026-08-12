use super::super::ats_types::{AtsAnalysisResult, AtsSuggestion, FormatIssue, IssueSeverity};

pub(super) fn build_format_result(
    format_issues: Vec<FormatIssue>,
    suggestions: Vec<AtsSuggestion>,
    completeness_score: f64,
) -> AtsAnalysisResult {
    let critical_count = format_issues
        .iter()
        .filter(|issue| issue.severity == IssueSeverity::Critical)
        .count();
    let warning_count = format_issues
        .iter()
        .filter(|issue| issue.severity == IssueSeverity::Warning)
        .count();
    let format_score =
        (100.0 - (critical_count as f64 * 20.0) - (warning_count as f64 * 5.0)).max(0.0);

    AtsAnalysisResult {
        overall_score: (format_score * 0.5) + (completeness_score * 0.5),
        keyword_score: 0.0,
        format_score,
        completeness_score,
        keyword_matches: Vec::new(),
        missing_keywords: Vec::new(),
        missing_keyword_details: Vec::new(),
        format_issues,
        requirement_reviews: Vec::new(),
        hard_constraint_risks: Vec::new(),
        suggestions,
        matching_profile: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ats_types::SuggestionCategory;

    #[test]
    fn builds_format_scores_and_empty_job_metadata() {
        let format_issues = vec![
            FormatIssue {
                severity: IssueSeverity::Critical,
                issue: "critical".to_string(),
                fix: "fix".to_string(),
            },
            FormatIssue {
                severity: IssueSeverity::Warning,
                issue: "warning".to_string(),
                fix: "fix".to_string(),
            },
            FormatIssue {
                severity: IssueSeverity::Info,
                issue: "info".to_string(),
                fix: "fix".to_string(),
            },
        ];
        let suggestions = vec![AtsSuggestion {
            category: SuggestionCategory::FormatFix,
            suggestion: "suggestion".to_string(),
            impact: "impact".to_string(),
        }];

        let result = build_format_result(format_issues, suggestions, 80.0);

        assert_eq!(result.format_score, 75.0);
        assert_eq!(result.overall_score, 77.5);
        assert_eq!(result.format_issues.len(), 3);
        assert_eq!(result.suggestions.len(), 1);
        assert!(result.keyword_matches.is_empty());
        assert!(result.missing_keywords.is_empty());
        assert!(result.missing_keyword_details.is_empty());
        assert!(result.requirement_reviews.is_empty());
        assert!(result.hard_constraint_risks.is_empty());
    }
}
