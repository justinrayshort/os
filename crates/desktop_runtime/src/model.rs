use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DESKTOP_LAYOUT_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_WINDOW_WIDTH: i32 = 420;
pub const DEFAULT_WINDOW_HEIGHT: i32 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WindowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppId {
    Calculator,
    Explorer,
    Notepad,
    Paint,
    Terminal,
    Dialup,
}

impl AppId {
    pub fn title(self) -> &'static str {
        match self {
            Self::Calculator => "Calculator",
            Self::Explorer => "Explorer",
            Self::Notepad => "Notepad",
            Self::Paint => "Paint",
            Self::Terminal => "Terminal",
            Self::Dialup => "Dial-up",
        }
    }

    pub fn icon_id(self) -> &'static str {
        match self {
            Self::Calculator => "calculator",
            Self::Explorer => "folder",
            Self::Notepad => "notepad",
            Self::Paint => "paint",
            Self::Terminal => "terminal",
            Self::Dialup => "modem",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl WindowRect {
    pub fn offset(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }

    pub fn clamped_min(self, min_w: i32, min_h: i32) -> Self {
        Self {
            w: self.w.max(min_w),
            h: self.h.max(min_h),
            ..self
        }
    }
}

impl Default for WindowRect {
    fn default() -> Self {
        Self {
            x: 48,
            y: 48,
            w: DEFAULT_WINDOW_WIDTH,
            h: DEFAULT_WINDOW_HEIGHT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowFlags {
    pub resizable: bool,
    pub minimizable: bool,
    pub maximizable: bool,
    pub modal_parent: Option<WindowId>,
}

impl Default for WindowFlags {
    fn default() -> Self {
        Self {
            resizable: true,
            minimizable: true,
            maximizable: true,
            modal_parent: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowRecord {
    pub id: WindowId,
    pub app_id: AppId,
    pub title: String,
    pub icon_id: String,
    pub rect: WindowRect,
    pub restore_rect: Option<WindowRect>,
    pub z_index: u32,
    pub is_focused: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub flags: WindowFlags,
    pub persist_key: Option<String>,
    pub app_state: Value,
    pub launch_params: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopTheme {
    pub name: String,
    pub wallpaper_id: String,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub audio_enabled: bool,
}

impl Default for DesktopTheme {
    fn default() -> Self {
        Self {
            name: "Retro Classic".to_string(),
            wallpaper_id: "teal-solid".to_string(),
            high_contrast: false,
            reduced_motion: false,
            audio_enabled: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopPreferences {
    pub restore_on_boot: bool,
    pub max_restore_windows: usize,
    pub terminal_history_enabled: bool,
}

impl Default for DesktopPreferences {
    fn default() -> Self {
        Self {
            restore_on_boot: true,
            max_restore_windows: 5,
            terminal_history_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopState {
    pub next_window_id: u64,
    pub windows: Vec<WindowRecord>,
    pub start_menu_open: bool,
    pub active_modal: Option<WindowId>,
    pub theme: DesktopTheme,
    pub preferences: DesktopPreferences,
    pub last_explorer_path: Option<String>,
    pub last_notepad_slug: Option<String>,
    pub terminal_history: Vec<String>,
}

impl Default for DesktopState {
    fn default() -> Self {
        Self {
            next_window_id: 1,
            windows: Vec::new(),
            start_menu_open: false,
            active_modal: None,
            theme: DesktopTheme::default(),
            preferences: DesktopPreferences::default(),
            last_explorer_path: None,
            last_notepad_slug: None,
            terminal_history: Vec::new(),
        }
    }
}

impl DesktopState {
    pub fn focused_window_id(&self) -> Option<WindowId> {
        self.windows.iter().find(|w| w.is_focused).map(|w| w.id)
    }

    pub fn snapshot(&self) -> DesktopSnapshot {
        DesktopSnapshot {
            schema_version: DESKTOP_LAYOUT_SCHEMA_VERSION,
            theme: self.theme.clone(),
            preferences: self.preferences.clone(),
            windows: self.windows.clone(),
            last_explorer_path: self.last_explorer_path.clone(),
            last_notepad_slug: self.last_notepad_slug.clone(),
            terminal_history: self.terminal_history.clone(),
        }
    }

    pub fn from_snapshot(snapshot: DesktopSnapshot) -> Self {
        let mut state = Self::default();
        state.theme = snapshot.theme;
        state.preferences = snapshot.preferences;
        state.windows = snapshot.windows;
        state.last_explorer_path = snapshot.last_explorer_path;
        state.last_notepad_slug = snapshot.last_notepad_slug;
        state.terminal_history = snapshot.terminal_history;
        state.next_window_id = state
            .windows
            .iter()
            .map(|w| w.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        state
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopSnapshot {
    pub schema_version: u32,
    pub theme: DesktopTheme,
    pub preferences: DesktopPreferences,
    pub windows: Vec<WindowRecord>,
    pub last_explorer_path: Option<String>,
    pub last_notepad_slug: Option<String>,
    pub terminal_history: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenWindowRequest {
    pub app_id: AppId,
    pub title: Option<String>,
    pub icon_id: Option<String>,
    pub rect: Option<WindowRect>,
    pub persist_key: Option<String>,
    pub launch_params: Value,
    pub app_state: Value,
    pub flags: WindowFlags,
}

impl OpenWindowRequest {
    pub fn new(app_id: AppId) -> Self {
        Self {
            app_id,
            title: None,
            icon_id: None,
            rect: None,
            persist_key: None,
            launch_params: Value::Null,
            app_state: Value::Null,
            flags: WindowFlags::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointerPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResizeEdge {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragSession {
    pub window_id: WindowId,
    pub pointer_start: PointerPosition,
    pub rect_start: WindowRect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResizeSession {
    pub window_id: WindowId,
    pub edge: ResizeEdge,
    pub pointer_start: PointerPosition,
    pub rect_start: WindowRect,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InteractionState {
    pub dragging: Option<DragSession>,
    pub resizing: Option<ResizeSession>,
    pub desktop_selection_origin: Option<PointerPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeepLinkOpenTarget {
    App(AppId),
    NotesSlug(String),
    ProjectSlug(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DeepLinkState {
    pub open: Vec<DeepLinkOpenTarget>,
}
