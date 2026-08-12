use super::*;

impl FormFiller {
    /// Fill screening questions using stored answer patterns
    ///
    /// Finds question labels on the page, matches them against stored patterns,
    /// and fills the corresponding inputs with configured answers.
    pub(super) async fn fill_screening_questions(
        &self,
        page: &AutomationPage,
        result: &mut FillResult,
    ) {
        // Common screening question selectors across ATS platforms
        let question_selectors = [
            // Greenhouse
            "div.field label",
            "div.application-question label",
            // Lever
            "div.question-field label",
            "[data-qa='question-label']",
            // Workday
            "[data-automation-id='questionLabel']",
            "label.WGAE",
            // Generic
            "fieldset legend",
            "div.form-group label",
            ".question label",
            "label[for]",
        ];

        // Try to find and fill questions
        for selector in question_selectors {
            if let Ok(questions) = self.find_questions_with_selector(page, selector).await {
                for (question_text, input_selector) in questions {
                    if requires_user_answer(&question_text) {
                        result.add_manual_review_topic();
                        continue;
                    }

                    if let Some(answer) = self.find_screening_answer_for_question(&question_text) {
                        let answer_value = answer.answer.clone();
                        let review_topic = screening_answer_review_topic(&answer.question_pattern);
                        let question_chars = question_text.chars().count();

                        // Try to fill the associated input
                        if let Ok(true) = page.fill(&input_selector, &answer_value).await {
                            result.filled_fields.push(SCREENING_FIELD_LABEL.to_string());
                            result.add_screening_answer_topic(review_topic);
                            tracing::debug!(
                                question_chars,
                                "Filled screening question with answer"
                            );
                        } else if let Ok(true) = page.select(&input_selector, &answer_value).await {
                            result.filled_fields.push(SCREENING_FIELD_LABEL.to_string());
                            result.add_screening_answer_topic(review_topic);
                            tracing::debug!(question_chars, "Selected screening answer");
                        }
                    }
                }
            }
        }
    }

    /// Find question elements and their associated input selectors
    async fn find_questions_with_selector(
        &self,
        page: &AutomationPage,
        _selector: &str,
    ) -> Result<Vec<(String, String)>> {
        // Use JavaScript to find all question labels and their associated inputs
        let script = r#"
            (function() {
                const results = [];
                const labels = document.querySelectorAll('label[for], fieldset legend, .question label, [data-automation-id*="question"]');

                for (const label of labels) {
                    const text = label.textContent?.trim();
                    if (!text || text.length < 5) continue;

                    // Find associated input
                    let input = null;

                    // Try 'for' attribute
                    if (label.htmlFor) {
                        input = document.getElementById(label.htmlFor);
                    }

                    // Try next sibling
                    if (!input) {
                        input = label.nextElementSibling;
                        if (input && !['INPUT', 'SELECT', 'TEXTAREA'].includes(input.tagName)) {
                            input = input.querySelector('input, select, textarea');
                        }
                    }

                    // Try parent container
                    if (!input) {
                        const container = label.closest('.field, .form-group, .question, fieldset');
                        if (container) {
                            input = container.querySelector('input:not([type="hidden"]), select, textarea');
                        }
                    }

                    if (input && (input.tagName === 'INPUT' || input.tagName === 'SELECT' || input.tagName === 'TEXTAREA')) {
                        // Generate a unique selector for the input
                        let selector = '';
                        if (input.id) {
                            selector = '#' + input.id;
                        } else if (input.name) {
                            selector = `[name="${input.name}"]`;
                        } else {
                            // Use data attributes or class
                            const attrs = Array.from(input.attributes)
                                .filter(a => a.name.startsWith('data-'))
                                .map(a => `[${a.name}="${a.value}"]`)
                                .join('');
                            if (attrs) selector = input.tagName.toLowerCase() + attrs;
                        }

                        if (selector) {
                            results.push([text, selector]);
                        }
                    }
                }

                return results;
            })()
        "#;

        let value = page
            .inner()
            .evaluate(script)
            .await
            .map_err(|_| question_discovery_error())?;

        // Parse the result - expecting array of [question, selector] pairs
        let result_value: serde_json::Value =
            value.into_value().map_err(|_| question_discovery_error())?;

        let pairs: Vec<(String, String)> = match result_value {
            serde_json::Value::Array(arr) => arr
                .into_iter()
                .filter_map(|item| {
                    if let serde_json::Value::Array(pair) = item {
                        if pair.len() == 2 {
                            let q = pair[0].as_str()?.to_string();
                            let s = pair[1].as_str()?.to_string();
                            return Some((q, s));
                        }
                    }
                    None
                })
                .collect(),
            _ => Vec::new(),
        };

        Ok(pairs)
    }

    /// Find matching answer for a question text
    #[cfg(test)]
    pub(super) fn find_answer_for_question(&self, question: &str) -> Option<String> {
        self.find_screening_answer_for_question(question)
            .map(|answer| answer.answer.clone())
    }

    fn find_screening_answer_for_question(&self, question: &str) -> Option<&ScreeningAnswer> {
        if requires_user_answer(question) {
            return None;
        }

        for answer in &self.screening_answers {
            if requires_user_answer(&answer.question_pattern) {
                continue;
            }

            if screening_question_matches(&answer.question_pattern, question) {
                tracing::debug!(
                    pattern_chars = answer.question_pattern.chars().count(),
                    question_chars = question.chars().count(),
                    "Matched saved screening answer"
                );
                return Some(answer);
            }
        }

        None
    }
}

pub(super) fn question_discovery_error() -> anyhow::Error {
    anyhow::anyhow!(QUESTION_DISCOVERY_ERROR)
}
