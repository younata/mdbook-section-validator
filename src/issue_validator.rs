use url::Url;
use regex::Regex;
use serde::Deserialize;
use reqwest::{Result, StatusCode};
use reqwest::blocking::{Client, Response};

#[derive(Debug, Eq, PartialEq)]
pub enum GithubIssueType {
    Issue,
    PullRequest,
}

fn issue_type_from_string(str: &str) -> GithubIssueType {
    if str == "issues" {
        GithubIssueType::Issue
    } else {
        GithubIssueType::PullRequest
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Issue<'u> {
    Github(&'u str, &'u str, &'u str, GithubIssueType, &'u Url),
    Link(&'u Url),
}

pub fn issue_from_url(url: &Url) -> Issue {
    let github_regex = Regex::new(r"(?i)github.com/(.+?)/(.+?)/(issues|pull)/(\d+)$").unwrap();
    return if let Some(capture) = github_regex.captures(url.as_str()) {
        let issue_type_string = capture.get(3).unwrap().as_str();
        let issue_type = issue_type_from_string(issue_type_string);

        Issue::Github(
            capture.get(1).unwrap().as_str(),
            capture.get(2).unwrap().as_str(),
            capture.get(4).unwrap().as_str(),
            issue_type,
            url,
        )
    } else {
        Issue::Link(url)
    };
}

#[derive(Debug, Eq, PartialEq)]
pub enum ValidationResult {
    NoLongerValid,
    StillValid,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct IssueResult {
    state: String
}

pub trait IssueValidator {
    fn validate(&self, issue: &Issue) -> ValidationResult;
}

pub struct DefaultIssueValidator;

impl IssueValidator for DefaultIssueValidator {
    fn validate(&self, issue: &Issue) -> ValidationResult {
        match issue {
            Issue::Github(owner, repo, number, issue_type, _url) => self.github_validation_result(owner, repo, number, issue_type),
            Issue::Link(url) => self.arbitrary_url_validation_result(url)
        }
    }
}

impl DefaultIssueValidator {
    fn github_validation_result(&self, owner: &str, repo: &str, number: &str, issue_type: &GithubIssueType) -> ValidationResult {
        let issue_kind = match issue_type {
            GithubIssueType::Issue => "issues",
            GithubIssueType::PullRequest => "pulls"
        };

        let request_url = format!(
            "https://api.github.com/repos/{owner}/{repo}/{issue_kind}/{number}",
            owner = owner,
            repo = repo,
            issue_kind = issue_kind,
            number = number
        );
        let client = Client::new();
        let request = client.get(&request_url)
            .header("User-Agent", "younata/mdbook-section-validator");
        let send_result: Result<Response> = request.send();
        if let Result::Ok(response) = send_result {
            let json_result: Result<IssueResult> = response.json();
            if let Result::Ok(issue) = json_result {
                if issue.state.as_str() == "open" {
                    return ValidationResult::StillValid;
                }
            } else {
                eprintln!("Unable to unwrap json: {}", json_result.unwrap_err());
            }
        } else {
            eprintln!("bad response: {}", send_result.unwrap_err());
        }
        return ValidationResult::NoLongerValid;
    }

    fn arbitrary_url_validation_result(&self, url: &Url) -> ValidationResult {
        let client = Client::new();
        let request = client.head(url.as_str())
            .header("User-Agent", "younata/mdbook-section-validator");
        let result: Result<Response> = request.send();

        if let Result::Ok(response) = result {
            if response.status() == StatusCode::OK {
                return ValidationResult::StillValid;
            }
        }
        return ValidationResult::NoLongerValid;
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Url;
    use crate::issue_validator::{GithubIssueType, Issue, issue_from_url};

    #[test]
    fn issue_from_url_github_pr() {
        let url = Url::parse("https://github.com/rust-lang/mdBook/pull/1539").unwrap();

        assert_eq!(issue_from_url(&url), Issue::Github("rust-lang", "mdBook", "1539", GithubIssueType::PullRequest, &url));
    }

    #[test]
    fn issue_from_url_github_issue() {
        let url = Url::parse("https://github.com/rust-lang/mdBook/issues/1538").unwrap();

        assert_eq!(issue_from_url(&url), Issue::Github("rust-lang", "mdBook", "1538", GithubIssueType::Issue, &url));
    }

    #[test]
    fn issue_from_url_arbitrary_link() {
        let url = Url::parse("https://example.com").unwrap();

        assert_eq!(issue_from_url(&url), Issue::Link(&url));
    }
}