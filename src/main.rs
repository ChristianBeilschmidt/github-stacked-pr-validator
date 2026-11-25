#[cfg(target_arch = "wasm32")]
fn main() {
    leptos::mount::mount_to_body(github_stacked_pr_validator::app::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("This application runs in the browser. Use `trunk serve`.");
}
