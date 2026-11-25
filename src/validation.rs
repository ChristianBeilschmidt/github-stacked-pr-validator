use std::collections::HashSet;

use anyhow::Result;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Stack {
    pub number: u64,
    pub base: StackBase,
    pub pull_requests: Vec<PullRequest>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StackBase {
    #[serde(rename = "ref")]
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub base: Branch,
    pub head: Branch,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Branch {
    #[serde(rename = "ref")]
    pub name: String,
    pub sha: String,
    pub repo: Option<Repository>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Repository {
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct Comparison {
    pub status: String,
    pub behind_by: u64,
}

pub trait CompareClient {
    fn compare(&self, base: &str, head: &str) -> impl Future<Output = Result<Comparison>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckStatus {
    Passed,
    Failed(String),
    Unchecked,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub stack: Stack,
    pub checks: Vec<CheckStatus>,
    pub error: Option<String>,
}

impl Report {
    pub fn is_valid(&self) -> bool {
        self.error.is_none() && self.checks.iter().all(|c| *c == CheckStatus::Passed)
    }
}

pub async fn validate(stack: Stack, client: &impl CompareClient) -> Report {
    let mut report = Report {
        checks: vec![CheckStatus::Unchecked; stack.pull_requests.len()],
        stack,
        error: None,
    };
    if report.stack.pull_requests.is_empty() {
        report.error = Some("This stack does not contain any pull requests.".into());
        return report;
    }
    if report.stack.base.name.is_empty() {
        report.error = Some("GitHub did not provide a valid stack base branch.".into());
        return report;
    }
    let Some(repo_id) = report.stack.pull_requests[0]
        .base
        .repo
        .as_ref()
        .map(|r| r.id)
    else {
        report.error = Some("GitHub did not provide the base repository for this stack.".into());
        return report;
    };

    let mut seen = HashSet::new();
    let mut seen_branches = HashSet::new();
    seen_branches.insert(report.stack.base.name.clone());
    for (index, pr) in report.stack.pull_requests.iter().enumerate() {
        let expected_base = if index == 0 {
            &report.stack.base.name
        } else {
            &report.stack.pull_requests[index - 1].head.name
        };
        let problem = if !seen.insert(pr.number) {
            Some(format!(
                "PR #{} appears more than once in the stack.",
                pr.number
            ))
        } else if pr.state != "open" {
            Some(format!("PR #{} is {}, not open.", pr.number, pr.state))
        } else if pr.base.repo.as_ref().map(|r| r.id) != Some(repo_id)
            || pr.head.repo.as_ref().map(|r| r.id) != Some(repo_id)
        {
            Some("The PR uses a fork or a missing/different repository; only branches in the stack repository are supported.".into())
        } else if pr.base.name.is_empty() || pr.head.name.is_empty() {
            Some("GitHub did not provide both branch names.".into())
        } else if pr.base.name != *expected_base {
            Some(format!(
                "Gap before PR #{}: expected base branch '{}', but found '{}'.",
                pr.number, expected_base, pr.base.name
            ))
        } else if !seen_branches.insert(pr.head.name.clone()) {
            Some(format!(
                "PR #{} repeats branch '{}', creating a cycle in the stack.",
                pr.number, pr.head.name
            ))
        } else if !valid_sha(&pr.base.sha) || !valid_sha(&pr.head.sha) {
            Some("GitHub did not provide valid commit SHAs for this PR.".into())
        } else {
            match client.compare(&pr.base.sha, &pr.head.sha).await {
                Ok(comparison) if comparison.status == "ahead" && comparison.behind_by == 0 => None,
                Ok(comparison) => Some(format!(
                    "PR #{} is not cleanly rebased: comparison is '{}' and behind by {} commit(s).",
                    pr.number, comparison.status, comparison.behind_by
                )),
                Err(error) => Some(format!("Could not compare PR #{}: {error:#}", pr.number)),
            }
        };
        if let Some(problem) = problem {
            report.checks[index] = CheckStatus::Failed(problem.clone());
            report.error = Some(problem);
            return report;
        }
        report.checks[index] = CheckStatus::Passed;
    }
    report
}

fn valid_sha(value: &str) -> bool {
    (40..=64).contains(&value.len()) && value.bytes().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct MockClient {
        calls: Cell<usize>,
        status: &'static str,
        behind: u64,
        fail_on: Option<usize>,
    }

    impl CompareClient for MockClient {
        async fn compare(&self, _: &str, _: &str) -> Result<Comparison> {
            let calls = self.calls.get() + 1;
            self.calls.set(calls);
            if self.fail_on == Some(calls) {
                anyhow::bail!("API rate limit exceeded");
            }
            Ok(Comparison {
                status: self.status.into(),
                behind_by: self.behind,
            })
        }
    }

    fn client() -> MockClient {
        MockClient {
            calls: Cell::new(0),
            status: "ahead",
            behind: 0,
            fail_on: None,
        }
    }

    fn stack() -> Stack {
        let pr = |number, base: &str, head: &str| PullRequest {
            number,
            title: format!("PR {number}"),
            state: "open".into(),
            base: Branch {
                name: base.into(),
                sha: "a".repeat(40),
                repo: Some(Repository { id: 1 }),
            },
            head: Branch {
                name: head.into(),
                sha: "b".repeat(40),
                repo: Some(Repository { id: 1 }),
            },
        };
        Stack {
            number: 42,
            base: StackBase {
                name: "develop".into(),
            },
            pull_requests: vec![
                pr(1, "develop", "one"),
                pr(2, "one", "two"),
                pr(3, "two", "three"),
            ],
        }
    }

    fn run(stack: Stack, client: &MockClient) -> Report {
        futures::executor::block_on(validate(stack, client))
    }

    #[test]
    fn accepts_linear_stacks_and_single_pr() {
        let c = client();
        assert!(run(stack(), &c).is_valid());
        assert_eq!(c.calls.get(), 3);
        let mut single = stack();
        single.pull_requests.truncate(1);
        assert!(run(single, &client()).is_valid());
    }

    #[test]
    fn reports_gap_and_stops_before_compare() {
        let mut s = stack();
        s.pull_requests[1].base.name = "missing".into();
        let c = client();
        let report = run(s, &c);
        assert!(report.error.unwrap().contains("expected base branch 'one'"));
        assert_eq!(report.checks[0], CheckStatus::Passed);
        assert!(matches!(report.checks[1], CheckStatus::Failed(_)));
        assert_eq!(report.checks[2], CheckStatus::Unchecked);
        assert_eq!(c.calls.get(), 1);
    }

    #[test]
    fn rejects_bad_stack_metadata() {
        let mut s = stack();
        s.pull_requests.clear();
        assert!(!run(s, &client()).is_valid());
        let mut s = stack();
        s.pull_requests[0].base.name = "main".into();
        assert!(run(s, &client()).error.unwrap().contains("Gap"));
        let mut s = stack();
        s.pull_requests[1].number = 1;
        assert!(run(s, &client()).error.unwrap().contains("more than once"));
        let mut s = stack();
        s.pull_requests[0].state = "closed".into();
        assert!(run(s, &client()).error.unwrap().contains("closed"));
        let mut s = stack();
        s.pull_requests[0].head.repo = Some(Repository { id: 2 });
        assert!(run(s, &client()).error.unwrap().contains("fork"));
        let mut s = stack();
        s.pull_requests[0].head.sha = "not-a-sha".into();
        assert!(run(s, &client()).error.unwrap().contains("SHAs"));
        let mut s = stack();
        s.pull_requests[1].head.name = "develop".into();
        assert!(run(s, &client()).error.unwrap().contains("cycle"));
    }

    #[test]
    fn rejects_all_unrebased_comparisons() {
        for (status, behind) in [
            ("behind", 1),
            ("diverged", 2),
            ("identical", 0),
            ("ahead", 1),
        ] {
            let c = MockClient {
                status,
                behind,
                ..client()
            };
            let report = run(stack(), &c);
            assert!(!report.is_valid());
            assert_eq!(c.calls.get(), 1);
            assert_eq!(report.checks[1], CheckStatus::Unchecked);
        }
    }

    #[test]
    fn surfaces_api_errors_and_stops() {
        let c = MockClient {
            fail_on: Some(2),
            ..client()
        };
        let report = run(stack(), &c);
        assert!(report.error.unwrap().contains("rate limit"));
        assert_eq!(report.checks[2], CheckStatus::Unchecked);
        assert_eq!(c.calls.get(), 2);
    }

    #[test]
    fn incomplete_compare_responses_are_rejected() {
        assert!(serde_json::from_str::<Comparison>(r#"{"status":"ahead"}"#).is_err());
        assert!(serde_json::from_str::<Comparison>(r#"{"behind_by":0}"#).is_err());
    }
}
