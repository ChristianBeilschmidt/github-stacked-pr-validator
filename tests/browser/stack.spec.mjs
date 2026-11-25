import { test, expect } from "@playwright/test";

const sha = (letter) => letter.repeat(40);
const branch = (ref, letter, repo = 1) => ({
  ref,
  sha: sha(letter),
  repo: { id: repo },
});
const stack = () => ({
  number: 42,
  base: { ref: "develop" },
  pull_requests: [
    { number: 1, title: "First", state: "open", base: branch("develop", "a"), head: branch("one", "b") },
    { number: 2, title: "Second", state: "open", base: branch("one", "b"), head: branch("two", "c") },
    { number: 3, title: "Third", state: "open", base: branch("two", "c"), head: branch("three", "d") },
  ],
});

async function mockGitHub(page, data, options = {}) {
  const calls = [];
  await page.route("https://api.github.com/**", async (route) => {
    const request = route.request();
    const url = request.url();
    if (request.method() === "OPTIONS") {
      await route.fulfill({ status: 204, headers: {
        "access-control-allow-origin": "*",
        "access-control-allow-headers": "x-github-api-version",
        "access-control-allow-methods": "GET",
      } });
      return;
    }
    calls.push(url);
    const isStack = url.includes("/stacks/");
    const response = isStack ? (options.stackResponse ?? { status: 200, json: data }) :
      (options.compareResponse ?? { status: 200, json: { status: "ahead", behind_by: 0 } });
    await route.fulfill({
      ...response,
      headers: { "access-control-allow-origin": "*", ...response.headers },
    });
  });
  return calls;
}

test("valid stack URL checks all PRs in order", async ({ page }) => {
  const calls = await mockGitHub(page, stack());
  await page.goto("/");
  await page.getByLabel("Stack URL or repository").fill("https://github.com/acme/demo/stack/42");
  await page.getByRole("button", { name: "Check stack" }).click();
  await expect(page.getByRole("heading", { name: "Stack is linear" })).toBeVisible();
  await expect(page.getByText("Passed", { exact: true })).toHaveCount(3);
  expect(calls).toHaveLength(4);
  expect(calls[0]).toContain("/repos/acme/demo/stacks/42");
  expect(calls[1]).toContain(`/compare/${sha("a")}...${sha("b")}`);
  expect(calls[3]).toContain(`/compare/${sha("c")}...${sha("d")}`);
});

test("gap stops the check and leaves later PRs unchecked", async ({ page }) => {
  const data = stack();
  data.pull_requests[1].base.ref = "missing";
  const calls = await mockGitHub(page, data);
  await page.goto("/");
  await page.getByLabel("Stack URL or repository").fill("acme/demo");
  await page.getByLabel("Stack ID").fill("42");
  await page.getByRole("button", { name: "Check stack" }).click();
  await expect(page.getByRole("heading", { name: "Stack check stopped" })).toBeVisible();
  await expect(page.getByText(/expected base branch 'one'/).first()).toBeVisible();
  await expect(page.getByText("Passed", { exact: true })).toHaveCount(1);
  await expect(page.getByText("Problem", { exact: true })).toHaveCount(1);
  await expect(page.getByText("Not checked", { exact: true })).toHaveCount(1);
  expect(calls).toHaveLength(2);
});

test("rejects foreign URLs without making API requests", async ({ page }) => {
  const calls = await mockGitHub(page, stack());
  await page.goto("/");
  await page.getByLabel("Stack URL or repository").fill("https://example.com/acme/demo/stack/42");
  await page.getByRole("button", { name: "Check stack" }).click();
  await expect(page.getByRole("alert")).toContainText("GitHub stack URL");
  expect(calls).toHaveLength(0);
});

test("shows GitHub rate-limit errors", async ({ page }) => {
  await mockGitHub(page, stack(), {
    stackResponse: { status: 403, json: { message: "rate limit" }, headers: { "x-ratelimit-remaining": "0", "access-control-expose-headers": "x-ratelimit-remaining" } },
  });
  await page.goto("/");
  await page.getByLabel("Stack URL or repository").fill("acme/demo");
  await page.getByLabel("Stack ID").fill("42");
  await page.getByRole("button", { name: "Check stack" }).click();
  await expect(page.getByRole("alert")).toContainText("rate limit");
});

test("shows a missing stack error", async ({ page }) => {
  await mockGitHub(page, stack(), {
    stackResponse: { status: 404, json: { message: "Not Found" } },
  });
  await page.goto("/");
  await page.getByLabel("Stack URL or repository").fill("acme/demo");
  await page.getByLabel("Stack ID").fill("42");
  await page.getByRole("button", { name: "Check stack" }).click();
  await expect(page.getByRole("alert")).toContainText("HTTP 404");
});

test("shows network errors", async ({ page }) => {
  await page.route("https://api.github.com/**", (route) => route.abort("failed"));
  await page.goto("/");
  await page.getByLabel("Stack URL or repository").fill("acme/demo");
  await page.getByLabel("Stack ID").fill("42");
  await page.getByRole("button", { name: "Check stack" }).click();
  await expect(page.getByRole("alert")).toContainText("Network or browser CORS error");
});

test("stops a request that times out", async ({ page }) => {
  await page.clock.install();
  await page.route("https://api.github.com/**", async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 2000));
    await route.abort().catch(() => {});
  });
  await page.goto("/");
  await page.getByLabel("Stack URL or repository").fill("acme/demo");
  await page.getByLabel("Stack ID").fill("42");
  const request = page.waitForRequest("https://api.github.com/**");
  await page.getByRole("button", { name: "Check stack" }).click();
  await request;
  await page.clock.fastForward(21_000);
  await expect(page.getByRole("alert")).toContainText("timed out");
});

test("shows a rebase problem without checking later PRs", async ({ page }) => {
  const calls = await mockGitHub(page, stack(), {
    compareResponse: { status: 200, json: { status: "diverged", behind_by: 2 } },
  });
  await page.goto("/");
  await page.getByLabel("Stack URL or repository").fill("acme/demo");
  await page.getByLabel("Stack ID").fill("42");
  await page.getByRole("button", { name: "Check stack" }).click();
  await expect(page.getByText(/not cleanly rebased/).first()).toBeVisible();
  await expect(page.getByText("Not checked", { exact: true })).toHaveCount(2);
  expect(calls).toHaveLength(2);
});

test("reports invalid comparison responses and stops", async ({ page }) => {
  const calls = await mockGitHub(page, stack(), {
    compareResponse: { status: 200, json: { status: "ahead" } },
  });
  await page.goto("/");
  await page.getByLabel("Stack URL or repository").fill("acme/demo");
  await page.getByLabel("Stack ID").fill("42");
  await page.getByRole("button", { name: "Check stack" }).click();
  await expect(page.getByRole("heading", { name: "Stack check stopped" })).toBeVisible();
  await expect(page.getByText(/invalid or incomplete response/).first()).toBeVisible();
  await expect(page.getByText("Not checked", { exact: true })).toHaveCount(2);
  expect(calls).toHaveLength(2);
});
