//! Desktop runtime state model, reducer, persistence hooks, and shell components.
//!
//! `desktop_runtime` is the main API surface for the retro desktop shell. It exposes:
//!
//! - data types in [`model`]
//! - state transitions in [`reduce_desktop`]
//! - persistence helpers in [`persistence`]
//! - Leptos UI primitives in [`components`]
//!
//! # Example
//!
//! ```rust
//! use desktop_runtime::{
//!     reduce_desktop, AppId, DesktopAction, DesktopState, InteractionState, OpenWindowRequest,
//! };
//!
//! let mut state = DesktopState::default();
//! let mut interaction = InteractionState::default();
//!
//! let effects = reduce_desktop(
//!     &mut state,
//!     &mut interaction,
//!     DesktopAction::OpenWindow(OpenWindowRequest::new(AppId::Calculator)),
//! )
//! .expect("reducer should open a window");
//!
//! assert_eq!(state.windows.len(), 1);
//! assert!(effects.iter().any(|effect| matches!(effect, desktop_runtime::RuntimeEffect::PersistLayout)));
//! ```

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

/// Application registry metadata and app view renderers.
pub mod apps;
/// Leptos provider/context and desktop shell UI components.
pub mod components;
/// Core runtime state model and serializable snapshot types.
pub mod model;
/// Browser/local persistence helpers for desktop runtime state.
pub mod persistence;
/// Reducer actions and effect generation for desktop state transitions.
pub mod reducer;

/// Re-exported runtime provider and shell UI entrypoints.
pub use components::{use_desktop_runtime, DesktopProvider, DesktopRuntimeContext, DesktopShell};
/// Re-exported runtime state model types.
pub use model::*;
/// Re-exported persistence entrypoints used by the shell runtime.
pub use persistence::{
    load_boot_snapshot, persist_layout_snapshot, persist_terminal_history, persist_theme,
};
/// Re-exported reducer entrypoint and core action/effect enums.
pub use reducer::{reduce_desktop, DesktopAction, RuntimeEffect};
