//! Shared contract types between the desktop window manager runtime and internal apps.
//!
//! This crate defines the common interface for mounting an app into a managed window,
//! receiving lifecycle/bus events, and sending commands back to the desktop runtime.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use leptos::{Callable, Callback, ReadSignal, RwSignal, View};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Stable identifier for a runtime-managed window.
pub type WindowRuntimeId = u64;

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
/// Event payload delivered to apps through the runtime pub/sub bus.
pub struct AppEvent {
    /// Logical topic identifier.
    pub topic: String,
    /// JSON payload for the event.
    pub payload: Value,
    /// Source window id when known.
    pub source_window_id: Option<WindowRuntimeId>,
}

impl AppEvent {
    /// Creates a new app event.
    pub fn new(
        topic: impl Into<String>,
        payload: Value,
        source_window_id: Option<WindowRuntimeId>,
    ) -> Self {
        Self {
            topic: topic.into(),
            payload,
            source_window_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Commands that apps can send to the desktop runtime.
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
/// Runtime callbacks exposed to apps mounted in managed windows.
pub struct AppHost {
    command_sender: Callback<AppCommand>,
}

impl AppHost {
    /// Creates an app-host callback wrapper.
    pub fn new(command_sender: Callback<AppCommand>) -> Self {
        Self { command_sender }
    }

    /// Sends a command to the desktop runtime.
    pub fn send(&self, command: AppCommand) {
        self.command_sender.call(command);
    }

    /// Convenience helper to request a window title update.
    pub fn set_window_title(&self, title: impl Into<String>) {
        self.send(AppCommand::SetWindowTitle {
            title: title.into(),
        });
    }

    /// Convenience helper to persist app state through the manager.
    pub fn persist_state(&self, state: Value) {
        self.send(AppCommand::PersistState { state });
    }

    /// Convenience helper to publish an app-bus event.
    pub fn publish_event(&self, topic: impl Into<String>, payload: Value) {
        self.send(AppCommand::PublishEvent {
            topic: topic.into(),
            payload,
        });
    }
}

#[derive(Clone)]
/// App mount context injected by the desktop runtime per window instance.
pub struct AppMountContext {
    /// Stable runtime window id.
    pub window_id: WindowRuntimeId,
    /// Launch params supplied at window-open time.
    pub launch_params: Value,
    /// Manager-restored app state payload.
    pub restored_state: Value,
    /// Reactive lifecycle signal for this window/app.
    pub lifecycle: ReadSignal<AppLifecycleEvent>,
    /// Reactive inbox signal populated by the app-bus.
    pub inbox: RwSignal<Vec<AppEvent>>,
    /// Runtime command host.
    pub host: AppHost,
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
