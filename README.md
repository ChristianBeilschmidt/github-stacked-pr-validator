# Stack Check

A client-side GitHub stack validator built with Rust 2024, Leptos, WebAssembly
and adapted [Rust/UI](https://leptos.rust-ui.com/) components. It checks
public GitHub pull request stacks for gaps and rebase problems. It runs entirely
in the browser; there is no backend, sign-in or token storage.

## What it checks

Enter a GitHub stack URL such as
`https://github.com/geo-engine/geoengine/stack/1531`, or enter
`geo-engine/geoengine` and stack ID `1531` separately. A stack ID is not a PR
number.

The app loads the native GitHub stack and checks its PRs from bottom to top:

- The first PR targets the stack's reported base branch; each next PR targets
  the preceding PR's head branch.
- PRs are open, not repeated, and use branches in the stack repository. Fork
  branches are not supported.
- GitHub's comparison of each PR's base and head reports `ahead` with
  `behind_by` equal to zero.

The first problem stops the check. Already checked PRs, the problem, and
unchecked PRs are distinguished in the result. The app does not search for
PRs missing from GitHub's stack list, and cannot discover arbitrary unlisted
branches. GitHub API requests are not an atomic snapshot, so a stack that
changes during a check can produce a transient result.

Requests go directly from your browser to `api.github.com`. GitHub's anonymous
rate limit is shared by requests from the same public IP, including users on
the same network. Rate-limit and other API errors are shown in the app.

## Development

Requirements: a current stable Rust toolchain, the `wasm32-unknown-unknown`
target, [Trunk](https://trunkrs.dev/) and Node.js 24 with npm.
The deployment workflow uses Trunk 0.22.2.

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.2 --locked
npm ci
npm run css
trunk serve
```

Open the address printed by Trunk. After changing Rust/UI classes, run
`npm run css` again or run `npm run css:watch` in another terminal. To run
the core tests and build for a GitHub Pages project site:

```sh
cargo test --locked
npx playwright install chromium
npm run test:e2e
npm run css
trunk build --release --locked --public-url /github-stacked-pr-validator/
```

## GitHub Pages

In the repository settings, choose **Pages → Build and deployment → GitHub
Actions**. The included workflow builds and deploys on pushes to `main` and
can also be started manually. It sets the asset base path to the repository
name for project sites and `/` for `*.github.io` user or organization sites.
No server or repository secrets are required.

## Licensing

This project is licensed under Apache-2.0; see [LICENSE](LICENSE). The adapted
Rust/UI component is MIT-licensed and its copyright and license are reproduced
in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
