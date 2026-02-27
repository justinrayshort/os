//! Shared contract types between the desktop window manager runtime and managed apps.
//!
//! v2 introduces a capability-scoped service injection model (`AppServices`) and
//! canonical string application identifiers (`ApplicationId`) while keeping stable
//! lifecycle semantics for runtime-managed windows.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use leptos::{Callable, Callback, ReadSignal, RwSignal, View};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    /// Theme/wallpaper/accessibility shell controls.
    Theme,
    /// Host notification APIs.
    Notifications,
    /// Inter-application pub/sub and request/reply channels.
    Ipc,
    /// Requests for opening external URLs.
    ExternalUrl,
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
    /// Set the active desktop wallpaper preset id.
    SetDesktopWallpaper {
        /// Stable wallpaper preset id.
        wallpaper_id: String,
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
}

impl ThemeService {
    /// Requests shell skin change by id.
    pub fn set_skin(&self, skin_id: impl Into<String>) {
        self.sender.call(AppCommand::SetDesktopSkin {
            skin_id: skin_id.into(),
        });
    }

    /// Requests wallpaper change by id.
    pub fn set_wallpaper(&self, wallpaper_id: impl Into<String>) {
        self.sender.call(AppCommand::SetDesktopWallpaper {
            wallpaper_id: wallpaper_id.into(),
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

#[derive(Clone, Copy)]
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
    /// Notification service.
    pub notifications: NotificationService,
    /// IPC service.
    pub ipc: IpcService,
    sender: Callback<AppCommand>,
}

impl AppServices {
    /// Creates service handles from the runtime command callback.
    pub fn new(sender: Callback<AppCommand>) -> Self {
        Self {
            window: WindowService { sender },
            state: StateService { sender },
            config: ConfigService { sender },
            theme: ThemeService { sender },
            notifications: NotificationService { sender },
            ipc: IpcService { sender },
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
