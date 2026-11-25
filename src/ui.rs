use leptos::prelude::*;

// Adapted from Rust/UI's MIT-licensed Card component; see THIRD_PARTY_NOTICES.md.
#[component]
pub fn Card(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let classes = format!("flex flex-col rounded-xl border shadow-sm {class}");
    view! { <div class=classes data-name="Card">{children()}</div> }
}
