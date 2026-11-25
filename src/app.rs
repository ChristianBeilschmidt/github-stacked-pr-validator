use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::github::GitHubClient;
use crate::input::StackInput;
use crate::ui::Card;
use crate::validation::{CheckStatus, Report, validate};

#[derive(Clone)]
enum Screen {
    Idle,
    Loading,
    Error(String),
    Complete(StackInput, Report),
}

#[component]
pub fn App() -> impl IntoView {
    let repository = RwSignal::new(String::new());
    let number = RwSignal::new(String::new());
    let screen = RwSignal::new(Screen::Idle);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if matches!(screen.get_untracked(), Screen::Loading) {
            return;
        }
        let input = match StackInput::parse(&repository.get_untracked(), &number.get_untracked()) {
            Ok(input) => input,
            Err(error) => {
                screen.set(Screen::Error(error.to_string()));
                return;
            }
        };
        screen.set(Screen::Loading);
        spawn_local(async move {
            let client = GitHubClient::new(&input);
            match client.stack(input.number).await {
                Ok(stack) => {
                    let report = validate(stack, &client).await;
                    screen.set(Screen::Complete(input, report));
                }
                Err(error) => screen.set(Screen::Error(format!("{error:#}"))),
            }
        });
    };

    view! {
        <div class="min-h-screen">
            <header class="border-b border-slate-200 bg-white">
                <div class="mx-auto flex max-w-5xl items-center justify-between px-5 py-5 sm:px-8">
                    <a href="./" class="flex items-center gap-3 font-semibold tracking-tight text-slate-900">
                        <span class="grid h-9 w-9 place-items-center rounded-lg bg-indigo-600 text-lg font-bold text-white" aria-hidden="true">"≋"</span>
                        <span>"Stack Check"</span>
                    </a>
                    <span class="text-xs font-medium text-slate-500 sm:text-sm">"GitHub stack validator"</span>
                </div>
            </header>
            <main class="mx-auto max-w-5xl px-5 py-12 sm:px-8 sm:py-16">
                <div class="max-w-2xl">
                    <p class="mb-3 text-xs font-bold tracking-widest text-indigo-600 uppercase">"Pull requests, in order"</p>
                    <h1 class="text-4xl font-semibold tracking-tight text-slate-900 sm:text-5xl">"Keep your stack on track."</h1>
                    <p class="mt-4 text-base leading-7 text-slate-600">"Find broken links and rebase issues in a public GitHub pull request stack, before they slow down your review."</p>
                </div>
                <Card class="mt-10 border-slate-200 bg-white p-6 sm:p-8">
                    <h2 class="text-lg font-semibold">"Check a stack"</h2>
                    <p class="mt-1 text-sm leading-6 text-slate-500">"Paste a GitHub stack URL, or enter a repository and stack ID."</p>
                    <form class="mt-6" on:submit=submit>
                        <div class="grid gap-5 sm:grid-cols-[minmax(0,1fr)_12rem]">
                            <div>
                                <label for="repository" class="mb-2 block text-sm font-medium text-slate-700">"Stack URL or repository"</label>
                                <input id="repository" name="repository" type="text" required
                                    placeholder="https://github.com/owner/repo/stack/123"
                                    bind:value=repository
                                    class="h-11 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm text-slate-900 outline-none placeholder:text-slate-400 focus:border-indigo-500 focus:ring-2 focus:ring-indigo-200"/>
                            </div>
                            <div>
                                <label for="stack-id" class="mb-2 block text-sm font-medium text-slate-700">"Stack ID" <span class="font-normal text-slate-400">"(if needed)"</span></label>
                                <input id="stack-id" name="stack-id" type="text" inputmode="numeric"
                                    placeholder="e.g. 123" bind:value=number
                                    class="h-11 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm text-slate-900 outline-none placeholder:text-slate-400 focus:border-indigo-500 focus:ring-2 focus:ring-indigo-200"/>
                            </div>
                        </div>
                        <div class="mt-6 flex flex-wrap items-center gap-4">
                            <button type="submit"
                                disabled=move || matches!(screen.get(), Screen::Loading)
                                class="inline-flex h-11 items-center justify-center rounded-lg bg-indigo-600 px-5 text-sm font-semibold text-white shadow-sm transition hover:bg-indigo-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 disabled:cursor-wait disabled:opacity-60">
                                {move || if matches!(screen.get(), Screen::Loading) { "Checking..." } else { "Check stack" }}
                            </button>
                            <span class="text-xs text-slate-500">"Public repositories only · No sign-in required"</span>
                        </div>
                    </form>
                </Card>
                <section class="mt-8" aria-live="polite" aria-atomic="true">
                    {move || match screen.get() {
                        Screen::Idle => view! { <p class="text-sm text-slate-500">"Results will appear here after you check a stack."</p> }.into_any(),
                        Screen::Loading => view! { <p role="status" class="text-sm font-medium text-slate-600">"Checking the stack with GitHub..."</p> }.into_any(),
                        Screen::Error(error) => view! {
                            <div role="alert" class="rounded-xl border border-red-200 bg-red-50 p-5 text-sm text-red-800">
                                <strong class="block font-semibold">"Could not check the stack"</strong>
                                <p class="mt-1">{error}</p>
                            </div>
                        }.into_any(),
                        Screen::Complete(input, report) => render_report(input, report).into_any(),
                    }}
                </section>
                <footer class="mt-16 border-t border-slate-200 pt-6 text-xs leading-5 text-slate-500">
                    "Checks are performed in your browser using GitHub’s public API. Rate limits apply; results reflect the data fetched during the check."
                    <p class="mt-2">
                        <a href="LICENSE" class="underline underline-offset-2">"License"</a>
                        " · "
                        <a href="THIRD_PARTY_NOTICES.md" class="underline underline-offset-2">"Third-party notices"</a>
                    </p>
                </footer>
            </main>
        </div>
    }
}

fn render_report(input: StackInput, report: Report) -> impl IntoView {
    let valid = report.is_valid();
    let title = if valid {
        "Stack is linear"
    } else {
        "Stack check stopped"
    };
    let banner_class = if valid {
        "rounded-xl border border-emerald-200 bg-emerald-50 p-5 text-emerald-900"
    } else {
        "rounded-xl border border-amber-200 bg-amber-50 p-5 text-amber-950"
    };
    let error = report.error.clone();
    let base = report.stack.base.name.clone();
    let rows = report.stack.pull_requests.iter().zip(report.checks.iter()).enumerate().map(|(index, (pr, check))| {
        let (label, badge, detail) = match check {
            CheckStatus::Passed => ("Passed", "bg-emerald-100 text-emerald-800", None),
            CheckStatus::Failed(message) => ("Problem", "bg-amber-100 text-amber-900", Some(message.clone())),
            CheckStatus::Unchecked => ("Not checked", "bg-slate-100 text-slate-600", None),
        };
        let url = format!("https://github.com/{}/{}/pull/{}", input.owner, input.repo, pr.number);
        let number = pr.number;
        let title = pr.title.clone();
        let branches = format!("{} → {}", pr.head.name, pr.base.name);
        view! {
            <li class="relative border-t border-slate-100 px-5 py-5 sm:px-6">
                <div class="flex items-start gap-4">
                    <span class="grid h-8 w-8 shrink-0 place-items-center rounded-full border border-slate-200 bg-white text-xs font-semibold text-slate-500">{index + 1}</span>
                    <div class="min-w-0 flex-1">
                        <div class="flex flex-wrap items-center gap-2">
                            <a href=url target="_blank" rel="noopener noreferrer" class="font-semibold text-slate-900 underline-offset-4 hover:text-indigo-700 hover:underline">{format!("#{number} {title}")}</a>
                            <span class=format!("rounded-full px-2.5 py-1 text-xs font-semibold {badge}")>{label}</span>
                        </div>
                        <p class="mt-2 break-all font-mono text-xs leading-5 text-slate-500">{branches}</p>
                        {detail.map(|message| view! { <p class="mt-3 text-sm leading-6 text-amber-900">{message}</p> })}
                    </div>
                </div>
            </li>
        }
    }).collect_view();
    view! {
        <div class=banner_class>
            <h2 class="font-semibold">{title}</h2>
            {if let Some(error) = error {
                view! { <p class="mt-1 text-sm leading-6">{error}</p> }.into_any()
            } else {
                view! { <p class="mt-1 text-sm leading-6">"All pull requests are connected and cleanly rebased."</p> }.into_any()
            }}
        </div>
        <Card class="mt-5 overflow-hidden border-slate-200 bg-white">
            <div class="flex flex-wrap items-center justify-between gap-3 px-5 py-5 sm:px-6">
                <div>
                    <h2 class="font-semibold text-slate-900">"Pull requests"</h2>
                    <p class="mt-1 text-sm text-slate-500">{format!("Base: {base} · bottom to top")}</p>
                </div>
                <a href=input.stack_url() target="_blank" rel="noopener noreferrer" class="text-sm font-semibold text-indigo-700 underline-offset-4 hover:underline">"Open stack on GitHub ↗"</a>
            </div>
            <ol>{rows}</ol>
        </Card>
    }
}
