use leptos::*;
use serde_json::Value;

#[derive(Clone, Copy)]
struct ExplorerItem {
    name: &'static str,
    kind: &'static str,
    location: &'static str,
    modified: &'static str,
    route_hint: &'static str,
}

const EXPLORER_ITEMS: [ExplorerItem; 8] = [
    ExplorerItem {
        name: "Projects",
        kind: "Folder",
        location: "/projects",
        modified: "2026-02-25",
        route_hint: "open explorer",
    },
    ExplorerItem {
        name: "Notes",
        kind: "Folder",
        location: "/notes",
        modified: "2026-02-25",
        route_hint: "open notepad",
    },
    ExplorerItem {
        name: "Documentation",
        kind: "Folder",
        location: "/docs",
        modified: "2026-02-25",
        route_hint: "open notepad docs-index",
    },
    ExplorerItem {
        name: "retro-desktop-runtime",
        kind: "Project",
        location: "/projects/retro-desktop-runtime",
        modified: "2026-02-25",
        route_hint: "open projects:retro-desktop-runtime",
    },
    ExplorerItem {
        name: "windows-shell-notes",
        kind: "Note",
        location: "/notes/windows-shell-notes",
        modified: "2026-02-25",
        route_hint: "open notes:windows-shell-notes",
    },
    ExplorerItem {
        name: "about",
        kind: "Page",
        location: "/about",
        modified: "2026-02-25",
        route_hint: "open notes:about",
    },
    ExplorerItem {
        name: "dialup-easter-egg",
        kind: "Tool",
        location: "/#/open/dialup",
        modified: "2026-02-25",
        route_hint: "dial",
    },
    ExplorerItem {
        name: "terminal-cheatsheet",
        kind: "Note",
        location: "/notes/terminal-cheatsheet",
        modified: "2026-02-25",
        route_hint: "open notes:terminal-cheatsheet",
    },
];

#[component]
pub fn ExplorerApp(launch_params: Value) -> impl IntoView {
    let initial_target = launch_params
        .get("project_slug")
        .and_then(Value::as_str)
        .map(|slug| format!("projects/{slug}"))
        .unwrap_or_else(|| "Desktop".to_string());

    let selected_tree = create_rw_signal(initial_target);
    let selected_row = create_rw_signal::<Option<usize>>(None);
    let show_details = create_rw_signal(true);

    view! {
        <div class="app-shell app-explorer-shell">
            <div class="app-menubar">
                <button type="button">"File"</button>
                <button type="button">"Edit"</button>
                <button type="button">"View"</button>
                <button type="button">"Tools"</button>
                <button type="button">"Help"</button>
            </div>

            <div class="app-toolbar">
                <button type="button" on:click=move |_| selected_tree.set("Desktop".to_string())>"Desktop"</button>
                <button type="button" on:click=move |_| selected_tree.set("Projects".to_string())>"Projects"</button>
                <button type="button" on:click=move |_| selected_tree.set("Notes".to_string())>"Notes"</button>
                <button type="button" on:click=move |_| show_details.update(|v| *v = !*v)>
                    {move || if show_details.get() { "Details On" } else { "Details Off" }}
                </button>
            </div>

            <div class="explorer-workspace">
                <aside class="explorer-tree" aria-label="Folder tree">
                    <div class="tree-header">"Folders"</div>
                    <ul class="tree-list">
                        <TreeNode label="Desktop" selected_tree=selected_tree />
                        <TreeNode label="Projects" selected_tree=selected_tree />
                        <TreeNode label="Notes" selected_tree=selected_tree />
                        <TreeNode label="Documentation" selected_tree=selected_tree />
                        <TreeNode label="Archive" selected_tree=selected_tree />
                    </ul>
                </aside>

                <section class="explorer-pane">
                    <div class="pane-header">
                        <div class="pane-title">"Contents"</div>
                        <div class="pane-path">{move || format!("Path: {}", selected_tree.get())}</div>
                    </div>

                    <div class="explorer-listwrap">
                        <table class="explorer-list" role="grid" aria-label="Explorer list view">
                            <thead>
                                <tr>
                                    <th>"Name"</th>
                                    <th>"Type"</th>
                                    <th>"Modified"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For each=move || 0..EXPLORER_ITEMS.len() key=|idx| *idx let:idx>
                                    <ExplorerRow
                                        idx=idx
                                        item=EXPLORER_ITEMS[idx]
                                        selected_row=selected_row
                                    />
                                </For>
                            </tbody>
                        </table>
                    </div>

                    <Show when=move || show_details.get() fallback=|| ()>
                        <div class="explorer-details">
                            {move || {
                                if let Some(idx) = selected_row.get() {
                                    let item = EXPLORER_ITEMS[idx];
                                    view! {
                                        <div class="details-grid">
                                            <div>"Name"</div><div>{item.name}</div>
                                            <div>"Type"</div><div>{item.kind}</div>
                                            <div>"Route"</div><div>{item.route_hint}</div>
                                            <div>"Location"</div><div>{item.location}</div>
                                        </div>
                                    }.into_view()
                                } else {
                                    view! { <div class="details-empty">"Select an item to view properties."</div> }.into_view()
                                }
                            }}
                        </div>
                    </Show>
                </section>
            </div>

            <div class="app-statusbar">
                <span>{move || format!("{} item(s)", EXPLORER_ITEMS.len())}</span>
                <span>{move || {
                    selected_row
                        .get()
                        .map(|idx| format!("Selected: {}", EXPLORER_ITEMS[idx].name))
                        .unwrap_or_else(|| "Ready".to_string())
                }}</span>
            </div>
        </div>
    }
}

#[component]
fn TreeNode(label: &'static str, selected_tree: RwSignal<String>) -> impl IntoView {
    let is_selected = Signal::derive(move || selected_tree.get() == label);
    view! {
        <li>
            <button
                type="button"
                class=move || if is_selected.get() { "tree-node selected" } else { "tree-node" }
                on:click=move |_| selected_tree.set(label.to_string())
            >
                <span class="tree-glyph">"[+]"</span>
                <span>{label}</span>
            </button>
        </li>
    }
}

#[component]
fn ExplorerRow(
    idx: usize,
    item: ExplorerItem,
    selected_row: RwSignal<Option<usize>>,
) -> impl IntoView {
    let is_selected = Signal::derive(move || selected_row.get() == Some(idx));
    view! {
        <tr
            class=move || if is_selected.get() { "selected" } else { "" }
            on:mousedown=move |_| selected_row.set(Some(idx))
        >
            <td>{item.name}</td>
            <td>{item.kind}</td>
            <td>{item.modified}</td>
        </tr>
    }
}
