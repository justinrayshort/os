mod web_app;

pub use web_app::{DesktopEntry, SiteApp};

#[cfg(all(feature = "csr", target_arch = "wasm32"))]
pub fn mount() {
    leptos::mount_to_body(|| leptos::view! { <SiteApp /> })
}
