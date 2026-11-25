use anyhow::{Context, Result, anyhow, bail};
use futures::future::{Either, select};
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use serde::de::DeserializeOwned;

use crate::input::StackInput;
use crate::validation::{CompareClient, Comparison, Stack};

const API: &str = "https://api.github.com";
const TIMEOUT_MS: u32 = 20_000;

pub struct GitHubClient {
    repository_url: String,
}

impl GitHubClient {
    pub fn new(input: &StackInput) -> Self {
        Self {
            repository_url: format!("{API}/repos/{}/{}", input.owner, input.repo),
        }
    }

    pub async fn stack(&self, number: u64) -> Result<Stack> {
        let stack: Stack = self
            .get(&format!("{}/stacks/{number}", self.repository_url))
            .await?;
        if stack.number != number {
            bail!("GitHub returned a different stack ID than requested.");
        }
        Ok(stack)
    }

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let controller = web_sys::AbortController::new()
            .map_err(|_| anyhow!("Could not initialize the GitHub request."))?;
        let request = async {
            let response = Request::get(url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2026-03-10")
                .abort_signal(Some(&controller.signal()))
                .send()
                .await
                .map_err(|_| anyhow!("Network or browser CORS error while contacting GitHub."))?;
            if !response.ok() {
                let status = response.status();
                if status == 404 {
                    bail!(
                        "GitHub could not find the requested resource (HTTP 404). Check that the stack and repository are public."
                    );
                }
                if (status == 403 || status == 429)
                    && (status == 429
                        || response.headers().get("x-ratelimit-remaining").as_deref() == Some("0"))
                {
                    let reset = response.headers().get("x-ratelimit-reset");
                    let detail = reset
                        .map(|s| format!(" Reset time (Unix seconds): {s}."))
                        .unwrap_or_default();
                    bail!("GitHub's anonymous API rate limit was exceeded.{detail}");
                }
                bail!("GitHub returned HTTP {status}.");
            }
            response.json::<T>().await.map_err(|error| {
                anyhow!("GitHub returned an invalid or incomplete response: {error}")
            })
        };
        let timeout = TimeoutFuture::new(TIMEOUT_MS);
        futures::pin_mut!(request, timeout);
        match select(request, timeout).await {
            Either::Left((result, _)) => result,
            Either::Right((_, _)) => {
                controller.abort();
                Err(anyhow!("The GitHub request timed out. Please try again."))
            }
        }
    }
}

impl CompareClient for GitHubClient {
    async fn compare(&self, base: &str, head: &str) -> Result<Comparison> {
        self.get(&format!("{}/compare/{base}...{head}", self.repository_url))
            .await
            .context("GitHub comparison failed")
    }
}
