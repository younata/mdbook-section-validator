use url::Url;
use crate::issue_validator::{Issue, issue_from_url};

pub struct LinkFormatter;

impl LinkFormatter {
    pub fn markdown_many(links: &Vec<Url>) -> String {
        let markdown_links: Vec<String> = links.into_iter().map(|l| LinkFormatter::markdown_single(&l)).collect();
        if markdown_links.len() == 1 {
            markdown_links.last().unwrap().to_string()
        } else if markdown_links.len() == 2 {
            markdown_links.join(", and ")
        } else {
            // ew.
            vec![
                markdown_links[0..(markdown_links.len() - 1)].join(", "),
                markdown_links.last().unwrap().to_string()
            ].join(", and ").to_string()
        }
    }

    fn markdown_single(link: &Url) -> String {
        match issue_from_url(link) {
            Issue::Github(owner, repo, number, _, url) => format!("[`{}/{}#{}`]({})", owner, repo, number, url.as_str()),
            Issue::Link(url) => format!("[`{}`]({})", url.as_str(), url.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::link_formatter::LinkFormatter;
    use url::Url;

    #[test]
    fn markdown_single_github_link() {
        assert_eq!(
            LinkFormatter::markdown_single(&Url::parse("https://github.com/foo/bar/issues/1").unwrap()),
            "[`foo/bar#1`](https://github.com/foo/bar/issues/1)".to_string()
        )
    }

    #[test]
    fn markdown_single_non_github_link() {
        assert_eq!(
            LinkFormatter::markdown_single(&Url::parse("https://www.example.com/foo/bar/issues/1").unwrap()),
            "[`https://www.example.com/foo/bar/issues/1`](https://www.example.com/foo/bar/issues/1)".to_string()
        )
    }

    #[test]
    fn markdown_multiple_1_link() {
        assert_eq!(
            LinkFormatter::markdown_many(&vec![
                Url::parse("https://github.com/foo/bar/issues/1").unwrap()
            ]),
            "[`foo/bar#1`](https://github.com/foo/bar/issues/1)".to_string()
        )
    }

    #[test]
    fn markdown_multiple_2_links() {
        assert_eq!(
            LinkFormatter::markdown_many(&vec![
                Url::parse("https://github.com/foo/bar/issues/1").unwrap(),
                Url::parse("https://www.example.com/foo/bar/issues/1").unwrap()
            ]),
            "[`foo/bar#1`](https://github.com/foo/bar/issues/1), and [`https://www.example.com/foo/bar/issues/1`](https://www.example.com/foo/bar/issues/1)".to_string()
        )
    }

    #[test]
    fn markdown_multiple_many_links() {
        assert_eq!(
            LinkFormatter::markdown_many(&vec![
                Url::parse("https://github.com/foo/bar/issues/1").unwrap(),
                Url::parse("https://www.example.com/foo/bar/issues/1").unwrap(),
                Url::parse("https://github.com/bar/foo/issues/3").unwrap()
            ]),
            "[`foo/bar#1`](https://github.com/foo/bar/issues/1), [`https://www.example.com/foo/bar/issues/1`](https://www.example.com/foo/bar/issues/1), and [`bar/foo#3`](https://github.com/bar/foo/issues/3)".to_string()
        )
    }
}