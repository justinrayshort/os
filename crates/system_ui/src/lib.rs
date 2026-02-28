//! Shared UI primitive library for shell and built-in system applications.
//!
//! The crate owns reusable Leptos primitives, a centralized icon API, and the
//! stable `data-ui-*` DOM contract consumed by the desktop shell CSS layers.
//! Apps should compose these primitives instead of emitting ad hoc control
//! markup or reusing legacy `.app-*` class contracts directly.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

mod icon;
mod primitives;

pub use icon::{Icon, IconName, IconSize};
pub use primitives::{
    AppShell, Badge, Button, ButtonSize, ButtonVariant, ClockButton, Cluster, ColorField,
    CompletionItem, CompletionList, DataTable, DesktopBackdrop, DesktopIconButton, DesktopIconGrid,
    DesktopRoot, DesktopWindowLayer, DisclosurePanel, Elevation, ElevationLayer, EmptyState,
    FieldVariant, Grid, Heading, InspectorGrid, LauncherMenu, LayoutAlign, LayoutGap,
    LayoutJustify, LayoutPadding, ListSurface, MenuBar, MenuItem, MenuSeparator, MenuSurface,
    OptionCard, Pane, PaneHeader, Panel, PreviewFrame, ProgressBar, ProgressVariant, RangeField,
    ResizeHandle, SelectField, SplitLayout, Stack, StatusBar, StepFlow, StepFlowActions,
    StepFlowHeader, StepFlowStep, StepStatus, Surface, SurfaceVariant, Tab, TabList, Taskbar,
    TaskbarButton, TaskbarOverflowButton, TaskbarSection, TerminalLine, TerminalPrompt,
    TerminalSurface, TerminalTranscript, Text, TextArea, TextField, TextRole, TextTone, ToggleRow,
    ToolBar, TrayButton, TrayList, WindowBody, WindowControlButton, WindowControls, WindowFrame,
    WindowTitle, WindowTitleBar,
};

/// Convenience imports for application crates consuming the shared primitive set.
pub mod prelude {
    pub use crate::{
        AppShell, Badge, Button, ButtonSize, ButtonVariant, ClockButton, Cluster, ColorField,
        CompletionItem, CompletionList, DataTable, DesktopBackdrop, DesktopIconButton,
        DesktopIconGrid, DesktopRoot, DesktopWindowLayer, DisclosurePanel, Elevation,
        ElevationLayer, EmptyState, FieldVariant, Grid, Heading, Icon, IconName, IconSize,
        InspectorGrid, LauncherMenu, LayoutAlign, LayoutGap, LayoutJustify, LayoutPadding,
        ListSurface, MenuBar, MenuItem, MenuSeparator, MenuSurface, OptionCard, Pane, PaneHeader,
        Panel, PreviewFrame, ProgressBar, ProgressVariant, RangeField, ResizeHandle, SelectField,
        SplitLayout, Stack, StatusBar, StepFlow, StepFlowActions, StepFlowHeader, StepFlowStep,
        StepStatus, Surface, SurfaceVariant, Tab, TabList, Taskbar, TaskbarButton,
        TaskbarOverflowButton, TaskbarSection, TerminalLine, TerminalPrompt, TerminalSurface,
        TerminalTranscript, Text, TextArea, TextField, TextRole, TextTone, ToggleRow, ToolBar,
        TrayButton, TrayList, WindowBody, WindowControlButton, WindowControls, WindowFrame,
        WindowTitle, WindowTitleBar,
    };
}
