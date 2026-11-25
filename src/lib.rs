pub mod input;
pub mod validation;

#[cfg(target_arch = "wasm32")]
pub mod app;
#[cfg(target_arch = "wasm32")]
pub mod github;
#[cfg(target_arch = "wasm32")]
mod ui;
