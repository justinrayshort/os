mod web_app;

pub use web_app::{DesktopEntry, SiteApp};

#[cfg(all(feature = "csr", target_arch = "wasm32"))]
pub fn mount() {
    leptos::mount_to_body(|| leptos::view! { <SiteApp /> })
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
pub fn hydrate() {
    leptos::mount_to_body(|| leptos::view! { <SiteApp /> })
}

#[cfg(feature = "ssr")]
pub fn render_app() -> impl leptos::IntoView {
    leptos::view! { <SiteApp /> }
}
