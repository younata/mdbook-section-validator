mod link_formatter;
pub mod issue_validator;

use regex::{Regex, Captures};

use mdbook::book::{Book, BookItem};
use mdbook::errors::Error;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use url::Url;
use crate::link_formatter::LinkFormatter;
use crate::issue_validator::{IssueValidator, issue_from_url, ValidationResult};
use futures::executor::block_on;
use futures::stream::{self, StreamExt};

pub struct ValidatorProcessorOptions {
    hide_invalid: bool,
    invalid_message: String
}

#[derive(Debug, Eq, PartialEq)]
enum ValidationSection {
    NonValidationSection(String),
    ValidationSection(Vec<Url>, String),
}

pub struct ValidatorProcessor {
    pub validator: Box<dyn IssueValidator>
}

impl Preprocessor for ValidatorProcessor {
    fn name(&self) -> &str { "section-validator" }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        let options = self.build_options(ctx);

        book.for_each_mut(|item| {
            if let BookItem::Chapter(chapter) = item {
                chapter.content =
                    self.process_chapter(&chapter.content, &options)
            }
        });
        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool { renderer == "html" }
}

impl ValidatorProcessor {
    fn build_options(&self, ctx: &PreprocessorContext) -> ValidatorProcessorOptions {
        let mut options = ValidatorProcessorOptions {
            hide_invalid: true,
            invalid_message: "🚨 Warning, this content is out of date and is included for historical reasons. 🚨".to_string()
        };

        if let Some(config) = ctx.config.get_preprocessor("section-validator") {
            if let Some(toml::value::Value::Boolean(hide_closed)) = config.get("hide_invalid") {
                options.hide_invalid = *hide_closed;
            }
            if let Some(toml::value::Value::String(message)) = config.get("invalid_message") {
                options.invalid_message = message.to_string();
            }
        }

        options
    }

    fn process_chapter(
        &self,
        raw_content: &str,
        options: &ValidatorProcessorOptions
    ) -> String {
        let mut content = String::new();
        for section in ValidatorProcessor::validation_sections(raw_content) {
            match section {
                ValidationSection::NonValidationSection(text) => {
                    content.push_str(&text);
                },
                ValidationSection::ValidationSection(links, text) => {
                    let validation_result = self.is_section_valid(links.clone());
                    if options.hide_invalid && validation_result == ValidationResult::NoLongerValid {
                        continue;
                    }
                    content.push_str(&*format!("<div class=\"validated-content\" links=\"{}\">\n\n", ValidatorProcessor::links_joined(&links)));
                    if validation_result == ValidationResult::NoLongerValid {
                        content.push_str(&*options.invalid_message);
                    } else {
                        let mut is_or_are = "is";
                        if links.len() != 1 {
                            is_or_are = "are";
                        }
                        content.push_str(&*format!("⚠️ This is only valid while {} {} open", LinkFormatter::markdown_many(&links), is_or_are));
                    }
                    content.push_str(&text);
                    content.push_str("\n</div>");
                }
            }
        }
        content
    }

    fn validation_sections(raw_content: &str) -> Vec<ValidationSection> {
        let section_regex = Regex::new(r"(?m)^!!!(.+)$(?s)(.+?)(?-s)^!!!$").unwrap();

        let captures: Vec<Captures> = section_regex.captures_iter(&raw_content).collect();
        let mut sections: Vec<ValidationSection> = Vec::new();

        if captures.is_empty() {
            return vec!(ValidationSection::NonValidationSection(raw_content.to_string()));
        }

        let mut last_endpoint: usize = 0;
        for capture in captures {
            let mat = capture.get(0).unwrap();
            let start = mat.start();

            if start - last_endpoint != 0 {
                sections.push(ValidationSection::NonValidationSection(raw_content[last_endpoint..start].to_string()));
            }

            last_endpoint = mat.end();

            sections.push(ValidationSection::ValidationSection(
                ValidatorProcessor::links_to_check(capture.get(1).unwrap().as_str()),
                capture.get(2).unwrap().as_str().to_string()
            ))
        }


        if raw_content.len() > last_endpoint {
            sections.push(ValidationSection::NonValidationSection(raw_content[last_endpoint..raw_content.len()].to_string()));
        }

        return sections;
    }

    fn links_to_check(links: &str) -> Vec<Url> {
        links.split(",").map(|text| Url::parse(text).unwrap()).collect()
    }

    fn links_joined(links: &Vec<Url>) -> String {
        let links_strs: Vec<String> = links.into_iter().map(|url| url.as_str().to_string()).collect();
        links_strs.join(",")
    }

    fn is_section_valid(&self, links: Vec<Url>) -> ValidationResult {
        let stream = stream::unfold(links.into_iter(), |mut links| async {
            let url = links.next()?;
            let issue = issue_from_url(&url);
            let response = self.validator.validate(&issue).await;
            Some((response, links))
        });
        let result = block_on(async { stream.collect::<Vec<ValidationResult>>().await });
        result.into_iter().reduce(|a, b| {
            if a == ValidationResult::StillValid && b == ValidationResult::StillValid { a } else { ValidationResult::NoLongerValid }
        }).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::IssueValidator;
    use super::ValidatorProcessor;
    use super::ValidationSection;
    use url::Url;
    use crate::issue_validator::{Issue, ValidationResult};
    use crate::ValidatorProcessorOptions;
    use async_trait::async_trait;

    #[test]
    fn test_validation_sections_single_link() {
        let content = "whatever
!!!https://github.com/example/example/issues/1

some content to be conditionally included.

!!!

other content";

        let sections: Vec<ValidationSection> = ValidatorProcessor::validation_sections(&content);

        assert_eq!(sections.len(), 3);
        assert_eq!(sections.get(0).unwrap(), &ValidationSection::NonValidationSection("whatever\n".to_string()));
        assert_eq!(
            sections.get(1).unwrap(),
            &ValidationSection::ValidationSection(
                vec![Url::parse("https://github.com/example/example/issues/1").unwrap()],
                "\n\nsome content to be conditionally included.\n\n".to_string()
            )
        );
        assert_eq!(sections.get(2).unwrap(), &ValidationSection::NonValidationSection("\n\nother content".to_string()));
    }

    #[test]
    fn test_validation_sections_multiple() {
        let content = "!!!https://github.com/example/example/issues/1

some content to be conditionally included.

!!!

other content

!!!https://github.com/example/example/issues/1,https://github.com/example/example/issues/2

other content to be conditionally included.

!!!";

        let sections: Vec<ValidationSection> = ValidatorProcessor::validation_sections(&content);

        assert_eq!(sections.len(), 3);
        assert_eq!(
            sections.get(0).unwrap(),
            &ValidationSection::ValidationSection(
                vec![Url::parse("https://github.com/example/example/issues/1").unwrap()],
                "\n\nsome content to be conditionally included.\n\n".to_string()
            )
        );
        assert_eq!(sections.get(1).unwrap(), &ValidationSection::NonValidationSection("\n\nother content\n\n".to_string()));
        assert_eq!(
            sections.get(2).unwrap(),
            &ValidationSection::ValidationSection(
                vec![
                    Url::parse("https://github.com/example/example/issues/1").unwrap(),
                    Url::parse("https://github.com/example/example/issues/2").unwrap()
                ],
                "\n\nother content to be conditionally included.\n\n".to_string()
            )
        );
    }

    #[test]
    fn test_content_all_valid_still_included_with_warning() {
        let content = "whatever
!!!https://github.com/example/example/issues/1

some content to be conditionally included.

!!!

other content
        ";

        let validator = FakeIssueValidator { validate_behavior: ValidateBehavior::AllValid };

        let processor = ValidatorProcessor { validator: Box::new(validator) };

        let options = ValidatorProcessorOptions { hide_invalid: true, invalid_message: "".to_string() };

        let received_chapter = processor.process_chapter(
            content,
            &options
        );

        let expected_chapter = "whatever
<div class=\"validated-content\" links=\"https://github.com/example/example/issues/1\">

⚠️ This is only valid while [`example/example#1`](https://github.com/example/example/issues/1) is open

some content to be conditionally included.


</div>

other content
        ";
        assert_eq!(received_chapter, expected_chapter.to_string());
    }

    #[test]
    fn tset_content_none_valid_content_not_included() {
        let content = "whatever
!!!https://github.com/example/example/issues/1

some content to be conditionally included.

!!!

other content
        ";

        let validator = FakeIssueValidator { validate_behavior: ValidateBehavior::NoneValid };

        let processor = ValidatorProcessor { validator: Box::new(validator) };

        let received_chapter = processor.process_chapter(
            content,
            &ValidatorProcessorOptions {
                hide_invalid: true,
                invalid_message: "🚨 Warning, this content is out of date and is included for historical reasons. 🚨".to_string()
            }
        );

        let expected_chapter = "whatever


other content
        ";
        assert_eq!(received_chapter, expected_chapter.to_string());
    }

    #[test]
    fn test_content_none_valid_content_still_included_with_warning() {
        let content = "whatever
!!!https://github.com/example/example/issues/1

some content to be conditionally included.

!!!

other content
        ";

        let validator = FakeIssueValidator { validate_behavior: ValidateBehavior::NoneValid };

        let processor = ValidatorProcessor { validator: Box::new(validator) };

        let received_chapter = processor.process_chapter(
            content,
            &ValidatorProcessorOptions {
                hide_invalid: false,
                invalid_message: "🚨 Warning, this content is out of date and is included for historical reasons. 🚨".to_string()
            }
        );

        let expected_chapter = "whatever
<div class=\"validated-content\" links=\"https://github.com/example/example/issues/1\">

🚨 Warning, this content is out of date and is included for historical reasons. 🚨

some content to be conditionally included.


</div>

other content
        ";
        assert_eq!(received_chapter, expected_chapter.to_string());
    }

    enum ValidateBehavior {
        AllValid,
        NoneValid
    }

    struct FakeIssueValidator {
        validate_behavior: ValidateBehavior
    }

    #[async_trait]
    impl IssueValidator for FakeIssueValidator {
        async fn validate(&self, _link: &Issue) -> ValidationResult {
            async {
                match &self.validate_behavior {
                    ValidateBehavior::NoneValid => ValidationResult::NoLongerValid,
                    ValidateBehavior::AllValid => ValidationResult::StillValid
                }
            }.await
        }
    }
}