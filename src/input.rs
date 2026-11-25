use anyhow::{Result, bail};
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackInput {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

impl StackInput {
    pub fn parse(repository_or_url: &str, number: &str) -> Result<Self> {
        let value = repository_or_url.trim();
        let (owner, repo, stack_number) = if value.starts_with("https://") {
            let url = Url::parse(value)?;
            if url.scheme() != "https"
                || url.host_str() != Some("github.com")
                || !url.username().is_empty()
                || url.password().is_some()
                || url.port().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || !number.trim().is_empty()
            {
                bail!(
                    "Enter a GitHub stack URL without an additional stack ID, query or fragment."
                );
            }
            let parts: Vec<_> = url.path().trim_end_matches('/').split('/').collect();
            match parts.as_slice() {
                ["", owner, repo, "stack", stack_number] => (
                    owner.to_string(),
                    repo.to_string(),
                    stack_number.to_string(),
                ),
                _ => bail!("Use a URL like https://github.com/owner/repo/stack/123."),
            }
        } else {
            let parts: Vec<_> = value.split('/').collect();
            match parts.as_slice() {
                [owner, repo] => (
                    owner.to_string(),
                    repo.to_string(),
                    number.trim().to_string(),
                ),
                _ => bail!("Enter owner/repository or a GitHub stack URL."),
            }
        };
        if !valid_owner(&owner) || !valid_repo(&repo) {
            bail!("Enter a valid GitHub owner and repository name.");
        }
        let number = stack_number
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| anyhow::anyhow!("Enter a positive numeric stack ID."))?;
        Ok(Self {
            owner,
            repo,
            number,
        })
    }

    pub fn stack_url(&self) -> String {
        format!(
            "https://github.com/{}/{}/stack/{}",
            self.owner, self.repo, self.number
        )
    }
}

fn valid_owner(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 39
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
}

fn valid_repo(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_inputs_select_the_same_stack() {
        assert_eq!(
            StackInput::parse("geo-engine/geoengine", "1531").unwrap(),
            StackInput::parse("https://github.com/geo-engine/geoengine/stack/1531/", "").unwrap()
        );
    }

    #[test]
    fn rejects_invalid_inputs() {
        for (repo, id) in [
            ("https://evil.example/a/b/stack/1", ""),
            ("http://github.com/a/b/stack/1", ""),
            ("https://github.com/a/b/stack/1?x=y", ""),
            ("https://user@github.com/a/b/stack/1", ""),
            ("https://github.com/a/b/stack/1", "2"),
            ("a/../stack/1", ""),
            ("a/b", "0"),
            ("a/b", "-1"),
            ("a/b", ""),
            ("a/b/c", "1"),
            ("a/%2F", "1"),
        ] {
            assert!(StackInput::parse(repo, id).is_err(), "{repo} {id}");
        }
    }
}
