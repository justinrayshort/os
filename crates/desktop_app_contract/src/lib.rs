//! Shared contract types between the desktop window manager runtime and managed apps.
//!
//! v2 introduces a capability-scoped service injection model (`AppServices`) and
//! canonical string application identifiers (`ApplicationId`) while keeping stable
//! lifecycle semantics for runtime-managed windows.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use std::{cell::Cell, rc::Rc};

use futures::future::LocalBoxFuture;
use leptos::{Callable, Callback, ReadSignal, RwSignal, View};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use system_shell_contract::{
    CommandDescriptor, CompletionItem, CompletionRequest, ExecutionId, ShellError, ShellExit,
    ShellRequest, ShellStreamEvent,
};

/// Stable identifier for a runtime-managed window.
pub type WindowRuntimeId = u64;

/// Stable identifier for an app package/module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApplicationId(String);

impl ApplicationId {
    /// Returns an app identifier when `raw` conforms to the `segment.segment...` policy.
    pub fn new(raw: impl Into<String>) -> Result<Self, String> {
        let raw = raw.into();
        if is_valid_application_id(&raw) {
            Ok(Self(raw))
        } else {
            Err(format!(
                "invalid application id `{raw}`; expected namespaced dotted segments"
            ))
        }
    }

    /// Returns the string form of the identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Creates an id without validation for compile-time/runtime trusted constants.
    pub fn trusted(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }
}

impl std::fmt::Display for ApplicationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn is_valid_application_id(raw: &str) -> bool {
    if raw.is_empty() || raw.len() > 120 {
        return false;
    }

    let mut parts = raw.split('.');
    let mut count = 0usize;
    while let Some(part) = parts.next() {
        count += 1;
        if part.is_empty() || part.len() > 32 {
            return false;
        }
        let bytes = part.as_bytes();
        if !bytes[0].is_ascii_lowercase() {
            return false;
        }
        if !bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
        {
            return false;
        }
        if part.ends_with('-') {
            return false;
        }
    }

    count >= 2
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Declared app capability scopes enforced by runtime policy.
pub enum AppCapability {
    /// Window title/focus actions.
    Window,
    /// Window-scoped and app-shared state persistence APIs.
    State,
    /// Config key/value access.
    Config,
    /// Theme/accessibility shell controls.
    Theme,
    /// Wallpaper selection, preview, and library-management controls.
    Wallpaper,
    /// Host notification APIs.
    Notifications,
    /// Inter-application pub/sub and request/reply channels.
    Ipc,
    /// Requests for opening external URLs.
    ExternalUrl,
    /// Dynamic system terminal command registration.
    Commands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Lifecycle events emitted by the desktop window manager.
pub enum AppLifecycleEvent {
    /// App view has been mounted into a managed window.
    Mounted,
    /// Window became focused.
    Focused,
    /// Window lost focus.
    Blurred,
    /// Window was minimized.
    Minimized,
    /// Window was restored from minimized/maximized/suspended state.
    Restored,
    /// App is suspended by the manager.
    Suspended,
    /// App resumed from a suspended state.
    Resumed,
    /// Window close sequence started.
    Closing,
    /// Window close sequence completed.
    Closed,
}

impl AppLifecycleEvent {
    /// Returns a stable string token for persistence/debugging hooks.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Mounted => "mounted",
            Self::Focused => "focused",
            Self::Blurred => "blurred",
            Self::Minimized => "minimized",
            Self::Restored => "restored",
            Self::Suspended => "suspended",
            Self::Resumed => "resumed",
            Self::Closing => "closing",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Typed IPC envelope delivered through runtime-managed app inbox channels.
pub struct AppEvent {
    /// Envelope schema version.
    pub schema_version: u32,
    /// Topic identifier (`app.<app_id>.<channel>.v1`).
    pub topic: String,
    /// JSON payload for the event.
    pub payload: Value,
    /// Optional request/reply correlation id.
    pub correlation_id: Option<String>,
    /// Optional reply target topic.
    pub reply_to: Option<String>,
    /// Source app id when known.
    pub source_app_id: Option<String>,
    /// Source window id when known.
    pub source_window_id: Option<WindowRuntimeId>,
    /// Timestamp in unix milliseconds when known.
    pub timestamp_unix_ms: Option<u64>,
}

impl AppEvent {
    /// Creates a v1 app event from topic/payload/source window id.
    pub fn new(topic: impl Into<String>, payload: Value, source_window_id: Option<u64>) -> Self {
        Self {
            schema_version: 1,
            topic: topic.into(),
            payload,
            correlation_id: None,
            reply_to: None,
            source_app_id: None,
            source_window_id,
            timestamp_unix_ms: None,
        }
    }

    /// Adds request/reply metadata to the envelope.
    pub fn with_correlation(
        mut self,
        correlation_id: Option<String>,
        reply_to: Option<String>,
    ) -> Self {
        self.correlation_id = correlation_id;
        self.reply_to = reply_to;
        self
    }
}

/// Alias for v2 naming in runtime/app APIs.
pub type IpcEnvelope = AppEvent;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Transport commands emitted by app services to the desktop runtime.
pub enum AppCommand {
    /// Request a title update for the current window.
    SetWindowTitle {
        /// New title text.
        title: String,
    },
    /// Persist manager-owned app state for the current window.
    PersistState {
        /// Serialized app state payload.
        state: Value,
    },
    /// Persist app-shared state scoped by key.
    PersistSharedState {
        /// Shared state key.
        key: String,
        /// Shared state payload.
        state: Value,
    },
    /// Save a config value under a namespace/key pair.
    SaveConfig {
        /// Config namespace.
        namespace: String,
        /// Config key.
        key: String,
        /// Config payload.
        value: Value,
    },
    /// Request opening a URL through the host boundary.
    OpenExternalUrl {
        /// Target URL.
        url: String,
    },
    /// Subscribe current window to an app-bus topic.
    Subscribe {
        /// Topic name.
        topic: String,
    },
    /// Remove current window subscription for an app-bus topic.
    Unsubscribe {
        /// Topic name.
        topic: String,
    },
    /// Publish an event to all topic subscribers.
    PublishEvent {
        /// Topic name.
        topic: String,
        /// Event payload.
        payload: Value,
        /// Optional correlation id.
        correlation_id: Option<String>,
        /// Optional reply target.
        reply_to: Option<String>,
    },
    /// Set the active desktop skin preset.
    SetDesktopSkin {
        /// Stable desktop skin id (for example `modern-adaptive`).
        skin_id: String,
    },
    /// Preview a wallpaper configuration without committing it.
    PreviewWallpaper {
        /// Wallpaper preview configuration.
        config: WallpaperConfig,
    },
    /// Commit the active wallpaper preview as the current wallpaper.
    ApplyWallpaperPreview,
    /// Set the active wallpaper configuration immediately.
    SetCurrentWallpaper {
        /// Wallpaper configuration to apply.
        config: WallpaperConfig,
    },
    /// Clear the active wallpaper preview.
    ClearWallpaperPreview,
    /// Import a wallpaper asset through the host picker flow.
    ImportWallpaperFromPicker {
        /// Import policy and defaults for the new asset.
        request: WallpaperImportRequest,
    },
    /// Rename a managed wallpaper asset.
    RenameWallpaperAsset {
        /// Managed asset identifier.
        asset_id: String,
        /// New human-readable label.
        display_name: String,
    },
    /// Toggle the favorite flag for a managed wallpaper asset.
    SetWallpaperFavorite {
        /// Managed asset identifier.
        asset_id: String,
        /// Updated favorite state.
        favorite: bool,
    },
    /// Replace tags for a managed wallpaper asset.
    SetWallpaperTags {
        /// Managed asset identifier.
        asset_id: String,
        /// Tags associated with the asset.
        tags: Vec<String>,
    },
    /// Replace collection memberships for a managed wallpaper asset.
    SetWallpaperCollections {
        /// Managed asset identifier.
        asset_id: String,
        /// Collection identifiers.
        collection_ids: Vec<String>,
    },
    /// Create a new wallpaper collection.
    CreateWallpaperCollection {
        /// New collection label.
        display_name: String,
    },
    /// Rename an existing wallpaper collection.
    RenameWallpaperCollection {
        /// Collection identifier.
        collection_id: String,
        /// Updated collection label.
        display_name: String,
    },
    /// Delete a wallpaper collection and remove memberships.
    DeleteWallpaperCollection {
        /// Collection identifier.
        collection_id: String,
    },
    /// Delete a managed wallpaper asset.
    DeleteWallpaperAsset {
        /// Managed asset identifier.
        asset_id: String,
    },
    /// Toggle desktop high-contrast rendering.
    SetDesktopHighContrast {
        /// Whether high contrast should be enabled.
        enabled: bool,
    },
    /// Toggle desktop reduced-motion rendering.
    SetDesktopReducedMotion {
        /// Whether reduced motion should be enabled.
        enabled: bool,
    },
    /// Emit a host notification.
    Notify {
        /// Notification title.
        title: String,
        /// Notification body.
        body: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Manager policy controlling app suspension behavior.
pub enum SuspendPolicy {
    /// Minimized windows are suspended until restored.
    #[default]
    OnMinimize,
    /// Windows are never manager-suspended.
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
/// Identifies either a built-in wallpaper or an imported managed asset.
pub enum WallpaperSelection {
    /// Built-in wallpaper from the generated runtime catalog.
    BuiltIn {
        /// Stable built-in wallpaper identifier.
        wallpaper_id: String,
    },
    /// Imported managed wallpaper asset.
    Imported {
        /// Stable managed asset identifier.
        asset_id: String,
    },
}

impl Default for WallpaperSelection {
    fn default() -> Self {
        Self::BuiltIn {
            wallpaper_id: "cloud-bands".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
/// Traditional desktop wallpaper display modes.
pub enum WallpaperDisplayMode {
    /// Preserve aspect ratio while covering the viewport.
    #[default]
    Fill,
    /// Preserve aspect ratio while containing the image inside the viewport.
    Fit,
    /// Stretch the wallpaper to match the viewport exactly.
    Stretch,
    /// Repeat the wallpaper at intrinsic size from the top-left origin.
    Tile,
    /// Render the wallpaper once at intrinsic size.
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
/// Anchor position used for non-tiled wallpaper placement.
pub enum WallpaperPosition {
    /// Center the wallpaper.
    #[default]
    Center,
    /// Place the wallpaper in the top-left corner.
    TopLeft,
    /// Align the wallpaper to the top edge.
    Top,
    /// Place the wallpaper in the top-right corner.
    TopRight,
    /// Align the wallpaper to the left edge.
    Left,
    /// Align the wallpaper to the right edge.
    Right,
    /// Place the wallpaper in the bottom-left corner.
    BottomLeft,
    /// Align the wallpaper to the bottom edge.
    Bottom,
    /// Place the wallpaper in the bottom-right corner.
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
/// Persisted animation intent for wallpapers that can move.
pub enum WallpaperAnimationPolicy {
    /// Render the wallpaper in a static form.
    #[default]
    None,
    /// Loop animated media with muted playback.
    LoopMuted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
/// Media kind used by the shell renderer.
pub enum WallpaperMediaKind {
    /// Static bitmap image.
    #[default]
    StaticImage,
    /// Animated image such as GIF or animated SVG.
    AnimatedImage,
    /// Video wallpaper.
    Video,
    /// Static or animated SVG wallpaper.
    Svg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
/// Source origin for wallpaper assets in the user library.
pub enum WallpaperSourceKind {
    /// Shell-provided built-in wallpaper.
    BuiltIn,
    /// User-imported managed asset.
    #[default]
    Imported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Current or previewed wallpaper configuration.
pub struct WallpaperConfig {
    /// Wallpaper asset selection.
    pub selection: WallpaperSelection,
    /// Viewport rendering mode.
    #[serde(default)]
    pub display_mode: WallpaperDisplayMode,
    /// Anchor position used by placement modes.
    #[serde(default)]
    pub position: WallpaperPosition,
    /// Animation intent for moving wallpaper media.
    #[serde(default)]
    pub animation: WallpaperAnimationPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Managed wallpaper asset metadata known to the runtime library.
pub struct WallpaperAssetRecord {
    /// Stable asset identifier.
    pub asset_id: String,
    /// User-facing label.
    pub display_name: String,
    /// Built-in vs imported source classification.
    pub source_kind: WallpaperSourceKind,
    /// Media kind used by the shell renderer.
    pub media_kind: WallpaperMediaKind,
    /// MIME type for the primary asset payload.
    pub mime_type: String,
    /// Asset size in bytes.
    pub byte_len: u64,
    /// Natural width in pixels when known.
    pub natural_width: Option<u32>,
    /// Natural height in pixels when known.
    pub natural_height: Option<u32>,
    /// Duration in milliseconds for animated media when known.
    pub duration_ms: Option<u64>,
    /// Favorite flag shown in the library UI.
    #[serde(default)]
    pub favorite: bool,
    /// User-defined tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// User-defined collection memberships.
    #[serde(default)]
    pub collection_ids: Vec<String>,
    /// Managed primary URL or data URL.
    pub primary_url: String,
    /// Optional poster URL or data URL.
    pub poster_url: Option<String>,
    /// Creation timestamp when known.
    pub created_at_unix_ms: Option<u64>,
    /// Last-used timestamp when known.
    pub last_used_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// User-defined grouping for wallpaper library browsing.
pub struct WallpaperCollection {
    /// Stable collection identifier.
    pub collection_id: String,
    /// User-facing label.
    pub display_name: String,
    /// Stable ordering key.
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Full wallpaper library snapshot exposed to apps and persisted by the runtime.
pub struct WallpaperLibrarySnapshot {
    /// Known built-in and imported assets.
    pub assets: Vec<WallpaperAssetRecord>,
    /// User-defined collections.
    pub collections: Vec<WallpaperCollection>,
    /// Soft storage limit enforced by the host/runtime policy.
    pub soft_limit_bytes: u64,
    /// Current managed library usage in bytes.
    pub used_bytes: u64,
}

impl Default for WallpaperLibrarySnapshot {
    fn default() -> Self {
        Self {
            assets: Vec::new(),
            collections: Vec::new(),
            soft_limit_bytes: 512 * 1024 * 1024,
            used_bytes: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Import request describing how a newly selected wallpaper should enter the library.
pub struct WallpaperImportRequest {
    /// Default display name to use when host metadata is missing.
    pub display_name: Option<String>,
    /// Wallpaper configuration to apply after import when provided.
    pub default_config: Option<WallpaperConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Resolved wallpaper source information used by the renderer.
pub struct ResolvedWallpaperSource {
    /// Resolved URL for the primary asset.
    pub primary_url: String,
    /// Optional poster URL for animated media.
    pub poster_url: Option<String>,
    /// Media kind used by the renderer.
    pub media_kind: WallpaperMediaKind,
    /// Natural width in pixels when known.
    pub natural_width: Option<u32>,
    /// Natural height in pixels when known.
    pub natural_height: Option<u32>,
    /// Duration in milliseconds when known.
    pub duration_ms: Option<u64>,
}

#[derive(Clone, Copy)]
/// Window-scoped app service for shell window integration APIs.
pub struct WindowService {
    sender: Callback<AppCommand>,
}

impl WindowService {
    /// Requests a title change for the current window.
    pub fn set_title(&self, title: impl Into<String>) {
        self.sender.call(AppCommand::SetWindowTitle {
            title: title.into(),
        });
    }
}

#[derive(Clone, Copy)]
/// State persistence service for window and app-shared state channels.
pub struct StateService {
    sender: Callback<AppCommand>,
}

impl StateService {
    /// Persists manager-owned state for this window instance.
    pub fn persist_window_state(&self, state: Value) {
        self.sender.call(AppCommand::PersistState { state });
    }

    /// Persists app-shared state under `key`.
    pub fn persist_shared_state(&self, key: impl Into<String>, state: Value) {
        self.sender.call(AppCommand::PersistSharedState {
            key: key.into(),
            state,
        });
    }
}

#[derive(Clone, Copy)]
/// Namespaced app config service.
pub struct ConfigService {
    sender: Callback<AppCommand>,
}

impl ConfigService {
    /// Saves a namespaced config key/value payload.
    pub fn save(&self, namespace: impl Into<String>, key: impl Into<String>, value: Value) {
        self.sender.call(AppCommand::SaveConfig {
            namespace: namespace.into(),
            key: key.into(),
            value,
        });
    }
}

#[derive(Clone, Copy)]
/// Theme service for shell appearance/accessibility actions.
pub struct ThemeService {
    sender: Callback<AppCommand>,
    /// Current shell skin id.
    pub skin_id: ReadSignal<String>,
    /// Current high-contrast flag.
    pub high_contrast: ReadSignal<bool>,
    /// Current reduced-motion flag.
    pub reduced_motion: ReadSignal<bool>,
}

impl ThemeService {
    /// Requests shell skin change by id.
    pub fn set_skin(&self, skin_id: impl Into<String>) {
        self.sender.call(AppCommand::SetDesktopSkin {
            skin_id: skin_id.into(),
        });
    }

    /// Requests high contrast toggle.
    pub fn set_high_contrast(&self, enabled: bool) {
        self.sender
            .call(AppCommand::SetDesktopHighContrast { enabled });
    }

    /// Requests reduced motion toggle.
    pub fn set_reduced_motion(&self, enabled: bool) {
        self.sender
            .call(AppCommand::SetDesktopReducedMotion { enabled });
    }
}

#[derive(Clone, Copy)]
/// Wallpaper service for desktop background query, preview, and library operations.
pub struct WallpaperService {
    sender: Callback<AppCommand>,
    /// Current committed wallpaper configuration.
    pub current: ReadSignal<WallpaperConfig>,
    /// Current wallpaper preview when one exists.
    pub preview: ReadSignal<Option<WallpaperConfig>>,
    /// Current wallpaper library snapshot.
    pub library: ReadSignal<WallpaperLibrarySnapshot>,
}

impl WallpaperService {
    /// Starts a wallpaper preview.
    pub fn preview(&self, config: WallpaperConfig) {
        self.sender.call(AppCommand::PreviewWallpaper { config });
    }

    /// Commits the active wallpaper preview.
    pub fn apply_preview(&self) {
        self.sender.call(AppCommand::ApplyWallpaperPreview);
    }

    /// Replaces the current wallpaper immediately.
    pub fn set_current(&self, config: WallpaperConfig) {
        self.sender.call(AppCommand::SetCurrentWallpaper { config });
    }

    /// Clears the active wallpaper preview.
    pub fn clear_preview(&self) {
        self.sender.call(AppCommand::ClearWallpaperPreview);
    }

    /// Starts host import flow for a new wallpaper asset.
    pub fn import_from_picker(&self, request: WallpaperImportRequest) {
        self.sender
            .call(AppCommand::ImportWallpaperFromPicker { request });
    }

    /// Renames a managed wallpaper asset.
    pub fn rename_asset(&self, asset_id: impl Into<String>, display_name: impl Into<String>) {
        self.sender.call(AppCommand::RenameWallpaperAsset {
            asset_id: asset_id.into(),
            display_name: display_name.into(),
        });
    }

    /// Updates the favorite flag for a managed wallpaper asset.
    pub fn set_favorite(&self, asset_id: impl Into<String>, favorite: bool) {
        self.sender.call(AppCommand::SetWallpaperFavorite {
            asset_id: asset_id.into(),
            favorite,
        });
    }

    /// Replaces tags for a managed wallpaper asset.
    pub fn set_tags(&self, asset_id: impl Into<String>, tags: Vec<String>) {
        self.sender.call(AppCommand::SetWallpaperTags {
            asset_id: asset_id.into(),
            tags,
        });
    }

    /// Replaces collection memberships for a managed wallpaper asset.
    pub fn set_collections(&self, asset_id: impl Into<String>, collection_ids: Vec<String>) {
        self.sender.call(AppCommand::SetWallpaperCollections {
            asset_id: asset_id.into(),
            collection_ids,
        });
    }

    /// Creates a new wallpaper collection.
    pub fn create_collection(&self, display_name: impl Into<String>) {
        self.sender.call(AppCommand::CreateWallpaperCollection {
            display_name: display_name.into(),
        });
    }

    /// Renames an existing wallpaper collection.
    pub fn rename_collection(
        &self,
        collection_id: impl Into<String>,
        display_name: impl Into<String>,
    ) {
        self.sender.call(AppCommand::RenameWallpaperCollection {
            collection_id: collection_id.into(),
            display_name: display_name.into(),
        });
    }

    /// Deletes a wallpaper collection.
    pub fn delete_collection(&self, collection_id: impl Into<String>) {
        self.sender.call(AppCommand::DeleteWallpaperCollection {
            collection_id: collection_id.into(),
        });
    }

    /// Deletes a managed wallpaper asset.
    pub fn delete_asset(&self, asset_id: impl Into<String>) {
        self.sender.call(AppCommand::DeleteWallpaperAsset {
            asset_id: asset_id.into(),
        });
    }
}

#[derive(Clone, Copy)]
/// Notification service routed through host capabilities.
pub struct NotificationService {
    sender: Callback<AppCommand>,
}

impl NotificationService {
    /// Emits a host notification request.
    pub fn notify(&self, title: impl Into<String>, body: impl Into<String>) {
        self.sender.call(AppCommand::Notify {
            title: title.into(),
            body: body.into(),
        });
    }
}

#[derive(Clone, Copy)]
/// Inter-app IPC service for topic subscriptions and pub/sub request-reply envelopes.
pub struct IpcService {
    sender: Callback<AppCommand>,
}

impl IpcService {
    /// Subscribes this window to a topic.
    pub fn subscribe(&self, topic: impl Into<String>) {
        self.sender.call(AppCommand::Subscribe {
            topic: topic.into(),
        });
    }

    /// Unsubscribes this window from a topic.
    pub fn unsubscribe(&self, topic: impl Into<String>) {
        self.sender.call(AppCommand::Unsubscribe {
            topic: topic.into(),
        });
    }

    /// Publishes a one-way event payload.
    pub fn publish(&self, topic: impl Into<String>, payload: Value) {
        self.sender.call(AppCommand::PublishEvent {
            topic: topic.into(),
            payload,
            correlation_id: None,
            reply_to: None,
        });
    }

    /// Publishes a request payload carrying correlation metadata.
    pub fn request(
        &self,
        topic: impl Into<String>,
        payload: Value,
        correlation_id: impl Into<String>,
        reply_to: impl Into<String>,
    ) {
        self.sender.call(AppCommand::PublishEvent {
            topic: topic.into(),
            payload,
            correlation_id: Some(correlation_id.into()),
            reply_to: Some(reply_to.into()),
        });
    }
}

/// Async completion provider used by command registrations.
pub type AppCommandCompletion = Rc<
    dyn Fn(CompletionRequest) -> LocalBoxFuture<'static, Result<Vec<CompletionItem>, ShellError>>,
>;

/// Async command handler used by app command registrations.
pub type AppCommandHandler =
    Rc<dyn Fn(AppCommandContext) -> LocalBoxFuture<'static, Result<ShellExit, ShellError>>>;

/// Execution context supplied to app-registered command handlers.
#[derive(Clone)]
pub struct AppCommandContext {
    /// Execution identifier for the current command.
    pub execution_id: ExecutionId,
    /// Full parsed argv payload.
    pub argv: Vec<String>,
    /// Current logical cwd.
    pub cwd: String,
    /// Optional source window identifier.
    pub source_window_id: Option<WindowRuntimeId>,
    emit: Rc<dyn Fn(ShellStreamEvent)>,
    set_cwd: Rc<dyn Fn(String)>,
    is_cancelled: Rc<dyn Fn() -> bool>,
}

impl AppCommandContext {
    /// Emits a stdout chunk for the current execution.
    pub fn stdout(&self, text: impl Into<String>) {
        self.emit(ShellStreamEvent::StdoutChunk {
            execution_id: self.execution_id,
            text: text.into(),
        });
    }

    /// Emits a stderr chunk for the current execution.
    pub fn stderr(&self, text: impl Into<String>) {
        self.emit(ShellStreamEvent::StderrChunk {
            execution_id: self.execution_id,
            text: text.into(),
        });
    }

    /// Emits a status update for the current execution.
    pub fn status(&self, text: impl Into<String>) {
        self.emit(ShellStreamEvent::Status {
            execution_id: self.execution_id,
            text: text.into(),
        });
    }

    /// Emits a structured JSON payload for the current execution.
    pub fn json(&self, value: Value) {
        self.emit(ShellStreamEvent::Json {
            execution_id: self.execution_id,
            value,
        });
    }

    /// Emits an incremental shell stream event.
    pub fn emit(&self, event: ShellStreamEvent) {
        (self.emit)(event);
    }

    /// Updates the logical cwd for the current session.
    pub fn set_cwd(&self, cwd: impl Into<String>) {
        (self.set_cwd)(cwd.into());
    }

    /// Returns whether the active execution has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        (self.is_cancelled)()
    }

    /// Creates a new command context from runtime-provided callbacks.
    pub fn new(
        execution_id: ExecutionId,
        argv: Vec<String>,
        cwd: String,
        source_window_id: Option<WindowRuntimeId>,
        emit: Rc<dyn Fn(ShellStreamEvent)>,
        set_cwd: Rc<dyn Fn(String)>,
        is_cancelled: Rc<dyn Fn() -> bool>,
    ) -> Self {
        Self {
            execution_id,
            argv,
            cwd,
            source_window_id,
            emit,
            set_cwd,
            is_cancelled,
        }
    }
}

/// One command registration payload exposed by an app/provider.
#[derive(Clone)]
pub struct AppCommandRegistration {
    /// Static command metadata.
    pub descriptor: CommandDescriptor,
    /// Optional completion provider.
    pub completion: Option<AppCommandCompletion>,
    /// Async command handler.
    pub handler: AppCommandHandler,
}

/// Dynamic command provider implemented by apps that expose multiple commands.
pub trait AppCommandProvider {
    /// Returns all command registrations owned by this provider.
    fn commands(&self) -> Vec<AppCommandRegistration>;
}

/// Drop-based registration handle for dynamically registered commands.
#[derive(Clone)]
pub struct CommandRegistrationHandle {
    unregister: Rc<dyn Fn()>,
    active: Rc<Cell<bool>>,
}

impl CommandRegistrationHandle {
    /// Creates a new registration handle from an unregister callback.
    pub fn new(unregister: Rc<dyn Fn()>) -> Self {
        Self {
            unregister,
            active: Rc::new(Cell::new(true)),
        }
    }

    /// Creates a no-op registration handle.
    pub fn noop() -> Self {
        Self::new(Rc::new(|| {}))
    }

    /// Unregisters the command(s) if still active.
    pub fn unregister(&self) {
        if self.active.replace(false) {
            (self.unregister)();
        }
    }
}

impl Drop for CommandRegistrationHandle {
    fn drop(&mut self) {
        self.unregister();
    }
}

/// Live shell session bridge exposed to the terminal UI.
#[derive(Clone)]
pub struct ShellSessionHandle {
    /// Reactive shell event stream for this session.
    pub events: ReadSignal<Vec<ShellStreamEvent>>,
    /// Reactive active execution id when one exists.
    pub active_execution: ReadSignal<Option<ExecutionId>>,
    /// Reactive current cwd value.
    pub cwd: ReadSignal<String>,
    submit: Rc<dyn Fn(ShellRequest)>,
    cancel: Rc<dyn Fn()>,
    complete: AppCommandCompletion,
}

impl ShellSessionHandle {
    /// Creates a new shell session handle.
    pub fn new(
        events: ReadSignal<Vec<ShellStreamEvent>>,
        active_execution: ReadSignal<Option<ExecutionId>>,
        cwd: ReadSignal<String>,
        submit: Rc<dyn Fn(ShellRequest)>,
        cancel: Rc<dyn Fn()>,
        complete: AppCommandCompletion,
    ) -> Self {
        Self {
            events,
            active_execution,
            cwd,
            submit,
            cancel,
            complete,
        }
    }

    /// Submits a shell request to the active session.
    pub fn submit(&self, request: ShellRequest) {
        (self.submit)(request);
    }

    /// Cancels the active foreground execution.
    pub fn cancel(&self) {
        (self.cancel)();
    }

    /// Resolves completion candidates for the current request.
    pub async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<Vec<CompletionItem>, ShellError> {
        (self.complete)(request).await
    }
}

/// Command service bridging shell sessions and dynamic registration.
#[derive(Clone)]
pub struct CommandService {
    /// Reactive global terminal history maintained by the desktop runtime.
    pub history: ReadSignal<Vec<String>>,
    create_session: Rc<dyn Fn(String) -> Result<ShellSessionHandle, String>>,
    register_command: Rc<
        dyn Fn(AppCommandRegistration) -> Result<CommandRegistrationHandle, String>,
    >,
    register_provider:
        Rc<dyn Fn(Rc<dyn AppCommandProvider>) -> Result<CommandRegistrationHandle, String>>,
}

impl CommandService {
    /// Creates a command service from runtime-provided callbacks.
    pub fn new(
        history: ReadSignal<Vec<String>>,
        create_session: Rc<dyn Fn(String) -> Result<ShellSessionHandle, String>>,
        register_command: Rc<
            dyn Fn(AppCommandRegistration) -> Result<CommandRegistrationHandle, String>,
        >,
        register_provider: Rc<
            dyn Fn(Rc<dyn AppCommandProvider>) -> Result<CommandRegistrationHandle, String>,
        >,
    ) -> Self {
        Self {
            history,
            create_session,
            register_command,
            register_provider,
        }
    }

    /// Creates a disabled command service that rejects all requests deterministically.
    pub fn disabled() -> Self {
        Self::new(
            leptos::create_rw_signal(Vec::new()).read_only(),
            Rc::new(|_| Err("command sessions are unavailable".to_string())),
            Rc::new(|_| Err("command registration is unavailable".to_string())),
            Rc::new(|_| Err("command registration is unavailable".to_string())),
        )
    }

    /// Creates a new shell session for the current app window.
    pub fn create_session(&self, cwd: impl Into<String>) -> Result<ShellSessionHandle, String> {
        (self.create_session)(cwd.into())
    }

    /// Registers one command dynamically.
    pub fn register_command(
        &self,
        registration: AppCommandRegistration,
    ) -> Result<CommandRegistrationHandle, String> {
        (self.register_command)(registration)
    }

    /// Registers a multi-command provider dynamically.
    pub fn register_provider(
        &self,
        provider: Rc<dyn AppCommandProvider>,
    ) -> Result<CommandRegistrationHandle, String> {
        (self.register_provider)(provider)
    }
}

#[derive(Clone)]
/// Injected app services bundle.
pub struct AppServices {
    /// Window integration service.
    pub window: WindowService,
    /// State persistence service.
    pub state: StateService,
    /// Namespaced config service.
    pub config: ConfigService,
    /// Theme/accessibility service.
    pub theme: ThemeService,
    /// Wallpaper query/preview/library service.
    pub wallpaper: WallpaperService,
    /// Notification service.
    pub notifications: NotificationService,
    /// IPC service.
    pub ipc: IpcService,
    /// Shell command registration and session service.
    pub commands: CommandService,
    sender: Callback<AppCommand>,
}

impl AppServices {
    /// Creates service handles from the runtime command callback.
    pub fn new(
        sender: Callback<AppCommand>,
        theme_skin_id: ReadSignal<String>,
        theme_high_contrast: ReadSignal<bool>,
        theme_reduced_motion: ReadSignal<bool>,
        wallpaper_current: ReadSignal<WallpaperConfig>,
        wallpaper_preview: ReadSignal<Option<WallpaperConfig>>,
        wallpaper_library: ReadSignal<WallpaperLibrarySnapshot>,
        commands: CommandService,
    ) -> Self {
        Self {
            window: WindowService { sender },
            state: StateService { sender },
            config: ConfigService { sender },
            theme: ThemeService {
                sender,
                skin_id: theme_skin_id,
                high_contrast: theme_high_contrast,
                reduced_motion: theme_reduced_motion,
            },
            wallpaper: WallpaperService {
                sender,
                current: wallpaper_current,
                preview: wallpaper_preview,
                library: wallpaper_library,
            },
            notifications: NotificationService { sender },
            ipc: IpcService { sender },
            commands,
            sender,
        }
    }

    /// Low-level transport send for exceptional app/runtime flows.
    pub fn send(&self, command: AppCommand) {
        self.sender.call(command);
    }
}

#[derive(Clone)]
/// App mount context injected by the desktop runtime per window instance.
pub struct AppMountContext {
    /// Stable app id from the runtime catalog.
    pub app_id: ApplicationId,
    /// Stable runtime window id.
    pub window_id: WindowRuntimeId,
    /// Launch params supplied at window-open time.
    pub launch_params: Value,
    /// Manager-restored app state payload.
    pub restored_state: Value,
    /// Reactive lifecycle signal for this window/app.
    pub lifecycle: ReadSignal<AppLifecycleEvent>,
    /// Reactive inbox signal populated by the app-bus.
    pub inbox: RwSignal<Vec<IpcEnvelope>>,
    /// Runtime service bundle.
    pub services: AppServices,
}

/// Static app mount function used by the runtime registry.
pub type AppMountFn = fn(AppMountContext) -> View;

#[derive(Debug, Clone, Copy)]
/// Mounted app module descriptor used by the runtime app registry.
pub struct AppModule {
    mount_fn: AppMountFn,
}

impl AppModule {
    /// Creates a module from a mount function.
    pub const fn new(mount_fn: AppMountFn) -> Self {
        Self { mount_fn }
    }

    /// Mounts the app view with a runtime-provided context.
    pub fn mount(self, context: AppMountContext) -> View {
        (self.mount_fn)(context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Manifest-backed registration metadata for a runtime app entry.
pub struct AppRegistration {
    /// Canonical app id.
    pub app_id: ApplicationId,
    /// Human-readable display name.
    pub display_name: String,
    /// Package semantic version.
    pub version: String,
    /// Runtime contract version string.
    pub runtime_contract_version: String,
    /// Declared requested capabilities.
    pub requested_capabilities: Vec<AppCapability>,
    /// Whether only one instance should be active.
    pub single_instance: bool,
    /// Suspend policy for minimized windows.
    pub suspend_policy: SuspendPolicy,
    /// Launcher visibility flag.
    pub show_in_launcher: bool,
    /// Desktop icon visibility flag.
    pub show_on_desktop: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_id_requires_dotted_namespaces() {
        assert!(ApplicationId::new("system.calculator").is_ok());
        assert!(ApplicationId::new("system.settings").is_ok());
        assert!(ApplicationId::new("calculator").is_err());
        assert!(ApplicationId::new("System.calc").is_err());
        assert!(ApplicationId::new("system..calc").is_err());
    }

    #[test]
    fn publish_event_request_metadata_is_attached() {
        let envelope = AppEvent::new("app.system.calc.events.v1", Value::Null, Some(3))
            .with_correlation(
                Some("req-1".to_string()),
                Some("app.system.calc.reply.v1".to_string()),
            );
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(envelope.correlation_id.as_deref(), Some("req-1"));
        assert_eq!(
            envelope.reply_to.as_deref(),
            Some("app.system.calc.reply.v1")
        );
    }
}
