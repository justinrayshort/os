//! Reducer actions, side-effect intents, and transition logic for the desktop runtime.

use serde_json::{json, Value};
use thiserror::Error;

use crate::model::{
    AppId, DeepLinkOpenTarget, DeepLinkState, DesktopSnapshot, DesktopState, InteractionState,
    OpenWindowRequest, PointerPosition, ResizeEdge, ResizeSession, WindowId, WindowRecord,
    WindowRect, DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH,
};

const MIN_WINDOW_WIDTH: i32 = 220;
const MIN_WINDOW_HEIGHT: i32 = 140;
const SNAP_EDGE_THRESHOLD: i32 = 24;

#[derive(Debug, Clone, PartialEq)]
/// Actions accepted by [`reduce_desktop`] to mutate [`DesktopState`].
pub enum DesktopAction {
    /// Open a new window using the supplied request.
    OpenWindow(OpenWindowRequest),
    /// Close a window by id.
    CloseWindow {
        /// Window to close.
        window_id: WindowId,
    },
    /// Focus (and raise) a window by id.
    FocusWindow {
        /// Window to focus.
        window_id: WindowId,
    },
    /// Minimize a window.
    MinimizeWindow {
        /// Window to minimize.
        window_id: WindowId,
    },
    /// Maximize a window to the provided viewport.
    MaximizeWindow {
        /// Window to maximize.
        window_id: WindowId,
        /// Viewport rectangle to maximize into.
        viewport: WindowRect,
    },
    /// Restore a minimized or maximized window.
    RestoreWindow {
        /// Window to restore.
        window_id: WindowId,
    },
    /// Toggle taskbar behavior for a window (focus, minimize, or restore).
    ToggleTaskbarWindow {
        /// Window associated with the taskbar button.
        window_id: WindowId,
    },
    /// Toggle the start menu open/closed.
    ToggleStartMenu,
    /// Close the start menu if open.
    CloseStartMenu,
    /// Begin dragging a window.
    BeginMove {
        /// Window being dragged.
        window_id: WindowId,
        /// Pointer position at drag start.
        pointer: PointerPosition,
    },
    /// Update an in-progress window drag.
    UpdateMove {
        /// Current pointer position.
        pointer: PointerPosition,
    },
    /// End the active window drag.
    EndMove,
    /// End the active window drag and apply viewport-edge snapping.
    EndMoveWithViewport {
        /// Current desktop viewport rectangle.
        viewport: WindowRect,
    },
    /// Begin resizing a window.
    BeginResize {
        /// Window being resized.
        window_id: WindowId,
        /// Edge or corner being dragged.
        edge: ResizeEdge,
        /// Pointer position at resize start.
        pointer: PointerPosition,
    },
    /// Update an in-progress window resize.
    UpdateResize {
        /// Current pointer position.
        pointer: PointerPosition,
    },
    /// End the active window resize.
    EndResize,
    /// Set the desktop theme display name.
    SetThemeName {
        /// New theme name.
        theme_name: String,
    },
    /// Set the active wallpaper preset id.
    SetWallpaper {
        /// Wallpaper preset id.
        wallpaper_id: String,
    },
    /// Toggle reduced-motion rendering.
    SetReducedMotion {
        /// Whether reduced motion is enabled.
        enabled: bool,
    },
    /// Append a command to terminal history (subject to preferences and limits).
    PushTerminalHistory {
        /// Terminal command text.
        command: String,
    },
    /// Replace the app-specific state payload for a window.
    SetAppState {
        /// Window whose app state should be replaced.
        window_id: WindowId,
        /// New app state payload.
        app_state: Value,
    },
    /// Hydrate runtime state from a persisted snapshot.
    HydrateSnapshot {
        /// Snapshot payload to restore.
        snapshot: DesktopSnapshot,
    },
    /// Apply URL-derived deep-link instructions.
    ApplyDeepLink {
        /// Parsed deep-link payload.
        deep_link: DeepLinkState,
    },
}

#[derive(Debug, Clone, PartialEq)]
/// Side-effect intents emitted by [`reduce_desktop`] for the shell runtime to execute.
pub enum RuntimeEffect {
    /// Persist the current desktop layout snapshot.
    PersistLayout,
    /// Persist theme changes.
    PersistTheme,
    /// Persist terminal history changes.
    PersistTerminalHistory,
    /// Move focus into the newly focused window's primary input.
    FocusWindowInput(WindowId),
    /// Parse and open deep-link targets in the UI layer.
    ParseAndOpenDeepLink(DeepLinkState),
    /// Open an external URL (for app actions that leave the shell).
    OpenExternalUrl(String),
    /// Play a named UI sound effect.
    PlaySound(&'static str),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
/// Reducer errors for invalid actions (for example, referencing a missing window).
pub enum ReducerError {
    /// The target window id was not found in the current state.
    #[error("window not found")]
    WindowNotFound,
}

/// Applies a [`DesktopAction`] to the desktop runtime state and collects resulting side effects.
///
/// This function is the authoritative state transition engine for desktop window management and
/// shell-level preferences.
///
/// # Errors
///
/// Returns [`ReducerError::WindowNotFound`] when an action references a window that is not present.
pub fn reduce_desktop(
    state: &mut DesktopState,
    interaction: &mut InteractionState,
    action: DesktopAction,
) -> Result<Vec<RuntimeEffect>, ReducerError> {
    let mut effects = Vec::new();
    match action {
        DesktopAction::OpenWindow(req) => {
            let window_id = next_window_id(state);
            let default_offset = ((window_id.0 as i32) - 1) % 8 * 20;
            let rect = req
                .rect
                .unwrap_or(WindowRect {
                    x: 40 + default_offset,
                    y: 48 + default_offset,
                    w: DEFAULT_WINDOW_WIDTH,
                    h: DEFAULT_WINDOW_HEIGHT,
                })
                .clamped_min(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT);
            let record = WindowRecord {
                id: window_id,
                app_id: req.app_id,
                title: req.title.unwrap_or_else(|| req.app_id.title().to_string()),
                icon_id: req
                    .icon_id
                    .unwrap_or_else(|| req.app_id.icon_id().to_string()),
                rect,
                restore_rect: None,
                z_index: 0,
                is_focused: false,
                minimized: false,
                maximized: false,
                flags: req.flags,
                persist_key: req.persist_key,
                app_state: req.app_state,
                launch_params: req.launch_params,
            };
            state.windows.push(record);
            focus_window_internal(state, window_id)?;
            state.start_menu_open = false;
            effects.push(RuntimeEffect::PersistLayout);
            effects.push(RuntimeEffect::FocusWindowInput(window_id));
            if matches!(req.app_id, AppId::Dialup) && state.theme.audio_enabled {
                effects.push(RuntimeEffect::PlaySound("dialup-open"));
            }
        }
        DesktopAction::CloseWindow { window_id } => {
            let before_len = state.windows.len();
            state.windows.retain(|w| w.id != window_id);
            if state.windows.len() == before_len {
                return Err(ReducerError::WindowNotFound);
            }
            if state.active_modal == Some(window_id) {
                state.active_modal = None;
            }
            normalize_window_stack(state);
            effects.push(RuntimeEffect::PersistLayout);
        }
        DesktopAction::FocusWindow { window_id } => {
            focus_window_internal(state, window_id)?;
            state.start_menu_open = false;
            effects.push(RuntimeEffect::FocusWindowInput(window_id));
        }
        DesktopAction::MinimizeWindow { window_id } => {
            let window = find_window_mut(state, window_id)?;
            window.minimized = true;
            window.is_focused = false;
            normalize_window_stack(state);
            effects.push(RuntimeEffect::PersistLayout);
        }
        DesktopAction::MaximizeWindow {
            window_id,
            viewport,
        } => {
            let window = find_window_mut(state, window_id)?;
            if !window.maximized {
                window.restore_rect = Some(window.rect);
            }
            window.rect = viewport.clamped_min(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT);
            window.maximized = true;
            window.minimized = false;
            focus_window_internal(state, window_id)?;
            effects.push(RuntimeEffect::PersistLayout);
        }
        DesktopAction::RestoreWindow { window_id } => {
            let window = find_window_mut(state, window_id)?;
            if window.maximized {
                if let Some(restore_rect) = window.restore_rect {
                    window.rect = restore_rect;
                }
                window.maximized = false;
            }
            window.minimized = false;
            focus_window_internal(state, window_id)?;
            effects.push(RuntimeEffect::PersistLayout);
        }
        DesktopAction::ToggleTaskbarWindow { window_id } => {
            let focused = state.focused_window_id() == Some(window_id);
            let minimized = state
                .windows
                .iter()
                .find(|w| w.id == window_id)
                .map(|w| w.minimized)
                .ok_or(ReducerError::WindowNotFound)?;
            if minimized {
                let _ = reduce_desktop(
                    state,
                    interaction,
                    DesktopAction::RestoreWindow { window_id },
                )?;
                effects.push(RuntimeEffect::PersistLayout);
            } else if focused {
                let _ = reduce_desktop(
                    state,
                    interaction,
                    DesktopAction::MinimizeWindow { window_id },
                )?;
                effects.push(RuntimeEffect::PersistLayout);
            } else {
                let _ =
                    reduce_desktop(state, interaction, DesktopAction::FocusWindow { window_id })?;
            }
        }
        DesktopAction::ToggleStartMenu => {
            state.start_menu_open = !state.start_menu_open;
        }
        DesktopAction::CloseStartMenu => {
            state.start_menu_open = false;
        }
        DesktopAction::BeginMove { window_id, pointer } => {
            let rect_start = find_window_mut(state, window_id)?.rect;
            focus_window_internal(state, window_id)?;
            interaction.dragging = Some(crate::model::DragSession {
                window_id,
                pointer_start: pointer,
                rect_start,
            });
        }
        DesktopAction::UpdateMove { pointer } => {
            if let Some(session) = interaction.dragging.as_ref() {
                let dx = pointer.x - session.pointer_start.x;
                let dy = pointer.y - session.pointer_start.y;
                let window = find_window_mut(state, session.window_id)?;
                if !window.maximized {
                    window.rect = session.rect_start.offset(dx, dy);
                }
            }
        }
        DesktopAction::EndMove => {
            interaction.dragging = None;
            effects.push(RuntimeEffect::PersistLayout);
        }
        DesktopAction::EndMoveWithViewport { viewport } => {
            let dragged_window_id = interaction
                .dragging
                .as_ref()
                .map(|session| session.window_id);
            interaction.dragging = None;

            if let Some(window_id) = dragged_window_id {
                snap_window_to_viewport_edge(state, window_id, viewport);
            }

            effects.push(RuntimeEffect::PersistLayout);
        }
        DesktopAction::BeginResize {
            window_id,
            edge,
            pointer,
        } => {
            let rect_start = find_window_mut(state, window_id)?.rect;
            focus_window_internal(state, window_id)?;
            interaction.resizing = Some(ResizeSession {
                window_id,
                edge,
                pointer_start: pointer,
                rect_start,
            });
        }
        DesktopAction::UpdateResize { pointer } => {
            if let Some(session) = interaction.resizing.as_ref() {
                let dx = pointer.x - session.pointer_start.x;
                let dy = pointer.y - session.pointer_start.y;
                let window = find_window_mut(state, session.window_id)?;
                if !window.maximized && window.flags.resizable {
                    window.rect = resize_rect(session.rect_start, session.edge, dx, dy)
                        .clamped_min(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT);
                }
            }
        }
        DesktopAction::EndResize => {
            interaction.resizing = None;
            effects.push(RuntimeEffect::PersistLayout);
        }
        DesktopAction::SetThemeName { theme_name } => {
            state.theme.name = theme_name;
            effects.push(RuntimeEffect::PersistTheme);
        }
        DesktopAction::SetWallpaper { wallpaper_id } => {
            state.theme.wallpaper_id = wallpaper_id;
            effects.push(RuntimeEffect::PersistTheme);
        }
        DesktopAction::SetReducedMotion { enabled } => {
            state.theme.reduced_motion = enabled;
            effects.push(RuntimeEffect::PersistTheme);
        }
        DesktopAction::PushTerminalHistory { command } => {
            if state.preferences.terminal_history_enabled && !command.trim().is_empty() {
                state.terminal_history.push(command);
                if state.terminal_history.len() > 100 {
                    let overflow = state.terminal_history.len() - 100;
                    state.terminal_history.drain(0..overflow);
                }
                effects.push(RuntimeEffect::PersistTerminalHistory);
            }
        }
        DesktopAction::SetAppState {
            window_id,
            app_state,
        } => {
            let window = find_window_mut(state, window_id)?;
            window.app_state = app_state;
            effects.push(RuntimeEffect::PersistLayout);
        }
        DesktopAction::HydrateSnapshot { snapshot } => {
            let max_restore = state.preferences.max_restore_windows;
            *state = DesktopState::from_snapshot(snapshot);
            if state.windows.len() > max_restore {
                state.windows.truncate(max_restore);
            }
            normalize_window_stack(state);
        }
        DesktopAction::ApplyDeepLink { deep_link } => {
            effects.push(RuntimeEffect::ParseAndOpenDeepLink(deep_link));
        }
    }

    normalize_window_stack(state);
    Ok(effects)
}

/// Converts a parsed deep-link target into an [`OpenWindowRequest`].
pub fn build_open_request_from_deeplink(target: DeepLinkOpenTarget) -> OpenWindowRequest {
    match target {
        DeepLinkOpenTarget::App(app_id) => OpenWindowRequest::new(app_id),
        DeepLinkOpenTarget::NotesSlug(slug) => {
            let mut req = OpenWindowRequest::new(AppId::Notepad);
            req.title = Some(format!("Note - {slug}"));
            req.persist_key = Some(format!("notes:{slug}"));
            req.launch_params = json!({ "slug": slug });
            req
        }
        DeepLinkOpenTarget::ProjectSlug(slug) => {
            let mut req = OpenWindowRequest::new(AppId::Explorer);
            req.title = Some(format!("Project - {slug}"));
            req.persist_key = Some(format!("projects:{slug}"));
            req.launch_params = json!({ "project_slug": slug });
            req
        }
    }
}

fn next_window_id(state: &mut DesktopState) -> WindowId {
    let id = WindowId(state.next_window_id);
    state.next_window_id = state.next_window_id.saturating_add(1);
    id
}

fn find_window_mut(
    state: &mut DesktopState,
    window_id: WindowId,
) -> Result<&mut WindowRecord, ReducerError> {
    state
        .windows
        .iter_mut()
        .find(|w| w.id == window_id)
        .ok_or(ReducerError::WindowNotFound)
}

fn focus_window_internal(
    state: &mut DesktopState,
    window_id: WindowId,
) -> Result<(), ReducerError> {
    let index = state
        .windows
        .iter()
        .position(|w| w.id == window_id)
        .ok_or(ReducerError::WindowNotFound)?;
    let already_focused_top = index + 1 == state.windows.len()
        && state
            .windows
            .get(index)
            .map(|w| w.is_focused && !w.minimized)
            .unwrap_or(false);
    if already_focused_top {
        return Ok(());
    }
    for window in &mut state.windows {
        window.is_focused = false;
    }
    let mut window = state.windows.remove(index);
    window.is_focused = true;
    window.minimized = false;
    state.windows.push(window);
    normalize_window_stack(state);
    Ok(())
}

fn normalize_window_stack(state: &mut DesktopState) {
    let mut has_focused = false;
    for (idx, window) in state.windows.iter_mut().enumerate() {
        window.z_index = (idx + 1) as u32;
        if window.minimized {
            window.is_focused = false;
        }
        if window.is_focused {
            if has_focused {
                window.is_focused = false;
            } else {
                has_focused = true;
            }
        }
    }

    if !has_focused {
        if let Some(last_non_minimized) = state.windows.iter_mut().rev().find(|w| !w.minimized) {
            last_non_minimized.is_focused = true;
        }
    }
}

fn resize_rect(start: WindowRect, edge: ResizeEdge, dx: i32, dy: i32) -> WindowRect {
    match edge {
        ResizeEdge::East => WindowRect {
            w: start.w + dx,
            ..start
        },
        ResizeEdge::West => WindowRect {
            x: start.x + dx,
            w: start.w - dx,
            ..start
        },
        ResizeEdge::South => WindowRect {
            h: start.h + dy,
            ..start
        },
        ResizeEdge::North => WindowRect {
            y: start.y + dy,
            h: start.h - dy,
            ..start
        },
        ResizeEdge::NorthEast => WindowRect {
            y: start.y + dy,
            h: start.h - dy,
            w: start.w + dx,
            ..start
        },
        ResizeEdge::NorthWest => WindowRect {
            x: start.x + dx,
            y: start.y + dy,
            w: start.w - dx,
            h: start.h - dy,
        },
        ResizeEdge::SouthEast => WindowRect {
            w: start.w + dx,
            h: start.h + dy,
            ..start
        },
        ResizeEdge::SouthWest => WindowRect {
            x: start.x + dx,
            w: start.w - dx,
            h: start.h + dy,
            ..start
        },
    }
}

fn snap_window_to_viewport_edge(
    state: &mut DesktopState,
    window_id: WindowId,
    viewport: WindowRect,
) {
    let Some(window) = state.windows.iter_mut().find(|w| w.id == window_id) else {
        return;
    };

    if window.minimized {
        return;
    }

    let near_left = window.rect.x <= viewport.x + SNAP_EDGE_THRESHOLD;
    let near_right = window.rect.x + window.rect.w >= viewport.x + viewport.w - SNAP_EDGE_THRESHOLD;
    let near_top = window.rect.y <= viewport.y + SNAP_EDGE_THRESHOLD;

    if near_top && window.flags.maximizable {
        if !window.maximized {
            window.restore_rect = Some(window.rect);
        }
        window.rect = viewport.clamped_min(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT);
        window.maximized = true;
        window.minimized = false;
        return;
    }

    if !(near_left || near_right) || !window.flags.resizable {
        return;
    }

    let half_width = (viewport.w / 2).max(MIN_WINDOW_WIDTH);
    let snapped = WindowRect {
        x: if near_right {
            viewport.x + viewport.w - half_width
        } else {
            viewport.x
        },
        y: viewport.y,
        w: half_width,
        h: viewport.h.max(MIN_WINDOW_HEIGHT),
    };

    window.restore_rect = Some(window.rect);
    window.rect = snapped;
    window.maximized = false;
    window.minimized = false;
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::model::{AppId, InteractionState, OpenWindowRequest};

    fn open(
        state: &mut DesktopState,
        interaction: &mut InteractionState,
        app_id: AppId,
    ) -> WindowId {
        let _ = reduce_desktop(
            state,
            interaction,
            DesktopAction::OpenWindow(OpenWindowRequest::new(app_id)),
        )
        .expect("open window");
        state.windows.last().expect("window").id
    }

    #[test]
    fn open_window_focuses_new_window_and_updates_stack() {
        let mut state = DesktopState::default();
        let mut interaction = InteractionState::default();

        let first = open(&mut state, &mut interaction, AppId::Explorer);
        let second = open(&mut state, &mut interaction, AppId::Notepad);

        assert_eq!(state.focused_window_id(), Some(second));
        assert_eq!(state.windows.len(), 2);
        assert_eq!(state.windows[0].id, first);
        assert_eq!(state.windows[1].id, second);
        assert_eq!(state.windows[1].z_index, 2);
    }

    #[test]
    fn taskbar_toggle_minimizes_if_focused_and_restores_if_minimized() {
        let mut state = DesktopState::default();
        let mut interaction = InteractionState::default();

        let win = open(&mut state, &mut interaction, AppId::Explorer);
        reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::ToggleTaskbarWindow { window_id: win },
        )
        .expect("minimize");

        let record = state.windows.iter().find(|w| w.id == win).unwrap();
        assert!(record.minimized);
        assert!(!record.is_focused);

        reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::ToggleTaskbarWindow { window_id: win },
        )
        .expect("restore");
        let record = state.windows.iter().find(|w| w.id == win).unwrap();
        assert!(!record.minimized);
        assert!(record.is_focused);
    }

    #[test]
    fn focusing_already_focused_top_window_is_noop_for_stack_order() {
        let mut state = DesktopState::default();
        let mut interaction = InteractionState::default();

        let first = open(&mut state, &mut interaction, AppId::Explorer);
        let second = open(&mut state, &mut interaction, AppId::Calculator);
        let before = state.windows.clone();

        let effects = reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::FocusWindow { window_id: second },
        )
        .expect("focus focused window");

        assert_eq!(state.windows, before);
        assert_eq!(state.focused_window_id(), Some(second));
        assert_ne!(state.focused_window_id(), Some(first));
        assert!(effects.contains(&RuntimeEffect::FocusWindowInput(second)));
    }

    #[test]
    fn moving_window_updates_rect_during_drag_and_persists_on_end() {
        let mut state = DesktopState::default();
        let mut interaction = InteractionState::default();

        let win = open(&mut state, &mut interaction, AppId::Terminal);
        let original = state.windows.iter().find(|w| w.id == win).unwrap().rect;

        reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::BeginMove {
                window_id: win,
                pointer: PointerPosition { x: 10, y: 10 },
            },
        )
        .unwrap();
        reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::UpdateMove {
                pointer: PointerPosition { x: 35, y: 50 },
            },
        )
        .unwrap();

        let moved = state.windows.iter().find(|w| w.id == win).unwrap().rect;
        assert_eq!(moved.x, original.x + 25);
        assert_eq!(moved.y, original.y + 40);
        let effects = reduce_desktop(&mut state, &mut interaction, DesktopAction::EndMove).unwrap();
        assert!(effects.contains(&RuntimeEffect::PersistLayout));
    }

    #[test]
    fn end_move_with_viewport_snaps_window_to_left_half() {
        let mut state = DesktopState::default();
        let mut interaction = InteractionState::default();
        let viewport = WindowRect {
            x: 0,
            y: 0,
            w: 1000,
            h: 700,
        };

        let win = open(&mut state, &mut interaction, AppId::Explorer);
        reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::BeginMove {
                window_id: win,
                pointer: PointerPosition { x: 0, y: 0 },
            },
        )
        .unwrap();
        reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::UpdateMove {
                pointer: PointerPosition { x: -35, y: 80 },
            },
        )
        .unwrap();

        reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::EndMoveWithViewport { viewport },
        )
        .unwrap();

        let record = state.windows.iter().find(|w| w.id == win).unwrap();
        assert_eq!(record.rect.x, 0);
        assert_eq!(record.rect.y, 0);
        assert_eq!(record.rect.w, 500);
        assert_eq!(record.rect.h, 700);
        assert!(!record.maximized);
    }

    #[test]
    fn end_move_with_viewport_snaps_window_to_top_maximize() {
        let mut state = DesktopState::default();
        let mut interaction = InteractionState::default();
        let viewport = WindowRect {
            x: 0,
            y: 0,
            w: 1200,
            h: 760,
        };

        let win = open(&mut state, &mut interaction, AppId::Terminal);
        reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::BeginMove {
                window_id: win,
                pointer: PointerPosition { x: 0, y: 0 },
            },
        )
        .unwrap();
        reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::UpdateMove {
                pointer: PointerPosition { x: 150, y: -40 },
            },
        )
        .unwrap();

        reduce_desktop(
            &mut state,
            &mut interaction,
            DesktopAction::EndMoveWithViewport { viewport },
        )
        .unwrap();

        let record = state.windows.iter().find(|w| w.id == win).unwrap();
        assert_eq!(record.rect, viewport);
        assert!(record.maximized);
        assert!(record.restore_rect.is_some());
    }
}
