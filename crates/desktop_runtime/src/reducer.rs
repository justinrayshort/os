use serde_json::{json, Value};
use thiserror::Error;

use crate::model::{
    AppId, DeepLinkOpenTarget, DeepLinkState, DesktopSnapshot, DesktopState, InteractionState,
    OpenWindowRequest, PointerPosition, ResizeEdge, ResizeSession, WindowId, WindowRecord,
    WindowRect, DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH,
};

const MIN_WINDOW_WIDTH: i32 = 220;
const MIN_WINDOW_HEIGHT: i32 = 140;

#[derive(Debug, Clone, PartialEq)]
pub enum DesktopAction {
    OpenWindow(OpenWindowRequest),
    CloseWindow {
        window_id: WindowId,
    },
    FocusWindow {
        window_id: WindowId,
    },
    MinimizeWindow {
        window_id: WindowId,
    },
    MaximizeWindow {
        window_id: WindowId,
        viewport: WindowRect,
    },
    RestoreWindow {
        window_id: WindowId,
    },
    ToggleTaskbarWindow {
        window_id: WindowId,
    },
    ToggleStartMenu,
    CloseStartMenu,
    BeginMove {
        window_id: WindowId,
        pointer: PointerPosition,
    },
    UpdateMove {
        pointer: PointerPosition,
    },
    EndMove,
    BeginResize {
        window_id: WindowId,
        edge: ResizeEdge,
        pointer: PointerPosition,
    },
    UpdateResize {
        pointer: PointerPosition,
    },
    EndResize,
    SetThemeName {
        theme_name: String,
    },
    SetWallpaper {
        wallpaper_id: String,
    },
    SetReducedMotion {
        enabled: bool,
    },
    PushTerminalHistory {
        command: String,
    },
    SetAppState {
        window_id: WindowId,
        app_state: Value,
    },
    HydrateSnapshot {
        snapshot: DesktopSnapshot,
    },
    ApplyDeepLink {
        deep_link: DeepLinkState,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeEffect {
    PersistLayout,
    PersistTheme,
    PersistTerminalHistory,
    FocusWindowInput(WindowId),
    ParseAndOpenDeepLink(DeepLinkState),
    OpenExternalUrl(String),
    PlaySound(&'static str),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReducerError {
    #[error("window not found")]
    WindowNotFound,
}

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
}
