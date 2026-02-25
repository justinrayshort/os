use desktop_runtime::{DesktopProvider, DesktopShell};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn SiteApp() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="Justin Short" />
        <Meta name="description" content="A retro desktop-style personal website shell." />

        <Router>
            <main class="site-root">
                <Routes>
                    <Route path="" view=DesktopEntry />
                    <Route path="/notes/:slug" view=CanonicalNoteRoute />
                    <Route path="/projects/:slug" view=CanonicalProjectRoute />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn DesktopEntry() -> impl IntoView {
    view! {
        <DesktopProvider>
            <DesktopShell />
        </DesktopProvider>
    }
}

#[component]
fn CanonicalNoteRoute() -> impl IntoView {
    let params = use_params_map();
    let slug = move || {
        params
            .with(|map| map.get("slug").cloned())
            .unwrap_or_else(|| "unknown".to_string())
    };

    view! {
        <section class="canonical-content canonical-note">
            <h1>"Note"</h1>
            <p>{move || format!("Slug: {}", slug())}</p>
            <p>"Canonical SSR route placeholder. Final version renders prebuilt HTML here."</p>
            <A href=move || format!("/?open=notes:{}", slug())>"Open in Desktop"</A>
        </section>
    }
}

#[component]
fn CanonicalProjectRoute() -> impl IntoView {
    let params = use_params_map();
    let slug = move || {
        params
            .with(|map| map.get("slug").cloned())
            .unwrap_or_else(|| "unknown".to_string())
    };

    view! {
        <section class="canonical-content canonical-project">
            <h1>"Project"</h1>
            <p>{move || format!("Slug: {}", slug())}</p>
            <p>"Canonical SSR route placeholder. Final version renders project metadata/details."</p>
            <A href=move || format!("/?open=projects:{}", slug())>"Open in Desktop"</A>
        </section>
    }
}
