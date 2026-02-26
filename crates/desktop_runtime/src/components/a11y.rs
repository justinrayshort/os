//! Internal DOM focus and menu-keyboard helpers for desktop shell widgets.

use wasm_bindgen::JsCast;

/// Returns the current active element as an [`web_sys::HtmlElement`] when possible.
pub(super) fn active_html_element() -> Option<web_sys::HtmlElement> {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.active_element())
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
}

/// Focuses an HTML element, ignoring browser focus errors.
pub(super) fn focus_html_element(element: &web_sys::HtmlElement) {
    let _ = element.focus();
}

/// Focuses an element by ID and reports whether a focusable HTML element was found.
pub(super) fn focus_element_by_id(id: &str) -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return false;
    };
    let Some(element) = document.get_element_by_id(id) else {
        return false;
    };
    let Ok(element) = element.dyn_into::<web_sys::HtmlElement>() else {
        return false;
    };
    focus_html_element(&element);
    true
}

/// Traps `Tab`/`Shift+Tab` within a fixed focus order list.
pub(super) fn trap_tab_focus(ev: &web_sys::KeyboardEvent, focus_order: &[&str]) {
    if ev.key() != "Tab" || ev.alt_key() || ev.ctrl_key() || ev.meta_key() {
        return;
    }
    if focus_order.is_empty() {
        return;
    }

    let active_id = active_html_element().map(|el| el.id()).unwrap_or_default();
    let current_index = focus_order
        .iter()
        .position(|id| *id == active_id)
        .unwrap_or(if ev.shift_key() {
            0
        } else {
            focus_order.len().saturating_sub(1)
        });
    let next_index = if ev.shift_key() {
        if current_index == 0 {
            focus_order.len().saturating_sub(1)
        } else {
            current_index.saturating_sub(1)
        }
    } else {
        (current_index + 1) % focus_order.len()
    };

    ev.prevent_default();
    let _ = focus_element_by_id(focus_order[next_index]);
}

fn menu_focusable_items(menu_id: &str) -> Vec<web_sys::HtmlElement> {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return Vec::new();
    };
    let Some(menu) = document.get_element_by_id(menu_id) else {
        return Vec::new();
    };
    let Ok(nodes) = menu.query_selector_all(
        r#"[role="menuitem"], [role="menuitemcheckbox"], [role="menuitemradio"]"#,
    ) else {
        return Vec::new();
    };

    let mut items = Vec::new();
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(item) = node.dyn_into::<web_sys::HtmlElement>() else {
            continue;
        };
        if item.get_attribute("disabled").is_some() {
            continue;
        }
        if item.get_attribute("aria-disabled").as_deref() == Some("true") {
            continue;
        }
        items.push(item);
    }

    items
}

/// Focuses the first enabled menu item inside a menu container.
pub(super) fn focus_first_menu_item(menu_id: &str) -> bool {
    let items = menu_focusable_items(menu_id);
    if let Some(first) = items.first() {
        focus_html_element(first);
        true
    } else {
        false
    }
}

fn focus_menu_item_relative(menu_id: &str, delta: i32) -> bool {
    let items = menu_focusable_items(menu_id);
    if items.is_empty() {
        return false;
    }

    let active_id = active_html_element().map(|el| el.id()).unwrap_or_default();
    let current_index = items
        .iter()
        .position(|item| !active_id.is_empty() && item.id() == active_id)
        .unwrap_or(0);
    let len = items.len() as i32;
    let next_index = (current_index as i32 + delta).rem_euclid(len) as usize;
    focus_html_element(&items[next_index]);
    true
}

fn focus_menu_item_edge(menu_id: &str, first: bool) -> bool {
    let items = menu_focusable_items(menu_id);
    if items.is_empty() {
        return false;
    }
    let index = if first {
        0
    } else {
        items.len().saturating_sub(1)
    };
    focus_html_element(&items[index]);
    true
}

/// Handles arrow/home/end menu navigation and prevents default when handled.
pub(super) fn handle_menu_roving_keydown(ev: &web_sys::KeyboardEvent, menu_id: &str) -> bool {
    let handled = match ev.key().as_str() {
        "ArrowDown" => focus_menu_item_relative(menu_id, 1),
        "ArrowUp" => focus_menu_item_relative(menu_id, -1),
        "Home" => focus_menu_item_edge(menu_id, true),
        "End" => focus_menu_item_edge(menu_id, false),
        _ => false,
    };

    if handled {
        ev.prevent_default();
        ev.stop_propagation();
    }
    handled
}
