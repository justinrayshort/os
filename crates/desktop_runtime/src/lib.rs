pub mod apps;
pub mod components;
pub mod model;
pub mod persistence;
pub mod reducer;

pub use components::{DesktopProvider, DesktopRuntimeContext, DesktopShell};
pub use model::*;
pub use persistence::{
    load_boot_snapshot, persist_layout_snapshot, persist_terminal_history, persist_theme,
};
pub use reducer::{reduce_desktop, DesktopAction, RuntimeEffect};
