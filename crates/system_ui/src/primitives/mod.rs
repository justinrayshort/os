//! Shared structural, shell, overlay, data-display, control, and layout primitives.

use leptos::ev::{FocusEvent, KeyboardEvent, MouseEvent};
use leptos::*;

use crate::{Icon, IconName, IconSize};

mod controls;
mod data_display;
mod layout;
mod navigation;
mod overlays;
mod shell;

pub use controls::{
    Button, CheckboxField, ColorField, CompletionItem, CompletionList, FieldGroup, ProgressBar,
    RangeField, SelectField, TextArea, TextField, ToggleRow,
};
pub use data_display::{
    Badge, Card, DataTable, ElevationLayer, EmptyState, Heading, InspectorGrid, ListSurface,
    OptionCard, Pane, PaneHeader, Panel, PreviewFrame, StatusBarItem, Surface, TerminalLine,
    TerminalPrompt, TerminalSurface, TerminalTranscript, Text, Tree, TreeItem,
};
pub use layout::{Cluster, Grid, SplitLayout, Stack};
pub use navigation::{
    DisclosurePanel, LauncherMenu, MenuBar, StatusBar, StepFlow, StepFlowActions, StepFlowHeader,
    StepFlowStep, Tab, TabList, ToolBar,
};
pub use overlays::{MenuItem, MenuSeparator, MenuSurface, Modal};
pub use shell::{
    AppShell, ClockButton, DesktopBackdrop, DesktopIconButton, DesktopIconGrid, DesktopRoot,
    DesktopWindowLayer, ResizeHandle, Taskbar, TaskbarButton, TaskbarOverflowButton,
    TaskbarSection, TrayButton, TrayList, WindowBody, WindowControlButton, WindowControls,
    WindowFrame, WindowTitle, WindowTitleBar,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Semantic surface variants for structural primitives.
pub enum SurfaceVariant {
    /// Primary surface.
    Standard,
    /// Secondary or muted surface.
    Muted,
    /// Inset surface.
    Inset,
}

impl Default for SurfaceVariant {
    fn default() -> Self {
        Self::Standard
    }
}

impl SurfaceVariant {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Muted => "muted",
            Self::Inset => "inset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Semantic elevation levels for shared primitives.
pub enum Elevation {
    /// Flat surface.
    Flat,
    /// Raised surface.
    Raised,
    /// Overlay surface.
    Overlay,
    /// Inset surface.
    Inset,
    /// Pressed control surface.
    Pressed,
}

impl Default for Elevation {
    fn default() -> Self {
        Self::Flat
    }
}

impl Elevation {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Raised => "raised",
            Self::Overlay => "overlay",
            Self::Inset => "inset",
            Self::Pressed => "pressed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared button variants.
pub enum ButtonVariant {
    /// Standard action button.
    Standard,
    /// Primary emphasized action button.
    Primary,
    /// Quiet/toggle style button.
    Quiet,
    /// Accent/emphasized button.
    Accent,
    /// Danger/destructive button.
    Danger,
}

impl Default for ButtonVariant {
    fn default() -> Self {
        Self::Standard
    }
}

impl ButtonVariant {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Primary => "primary",
            Self::Quiet => "quiet",
            Self::Accent => "accent",
            Self::Danger => "danger",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared button sizing tokens.
pub enum ButtonSize {
    /// Dense button.
    Sm,
    /// Default button.
    Md,
    /// Large button.
    Lg,
}

impl Default for ButtonSize {
    fn default() -> Self {
        Self::Md
    }
}

impl ButtonSize {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared input-field variants.
pub enum FieldVariant {
    /// Standard input.
    Standard,
    /// Inset/editor input.
    Inset,
}

impl Default for FieldVariant {
    fn default() -> Self {
        Self::Standard
    }
}

impl FieldVariant {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Inset => "inset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared text roles.
pub enum TextRole {
    /// Body text.
    Body,
    /// Label text.
    Label,
    /// Caption text.
    Caption,
    /// Title text.
    Title,
    /// Monospace/code text.
    Code,
}

impl Default for TextRole {
    fn default() -> Self {
        Self::Body
    }
}

impl TextRole {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Label => "label",
            Self::Caption => "caption",
            Self::Title => "title",
            Self::Code => "code",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared text tone.
pub enum TextTone {
    /// Primary text.
    Primary,
    /// Secondary text.
    Secondary,
    /// Accent text.
    Accent,
    /// Success/status tone.
    Success,
    /// Warning tone.
    Warning,
    /// Danger tone.
    Danger,
}

impl Default for TextTone {
    fn default() -> Self {
        Self::Primary
    }
}

impl TextTone {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Accent => "accent",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared progress variants.
pub enum ProgressVariant {
    /// Standard progress indicator.
    Standard,
}

impl Default for ProgressVariant {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared layout gap tokens.
pub enum LayoutGap {
    /// No gap.
    None,
    /// Small gap.
    Sm,
    /// Default gap.
    Md,
    /// Large gap.
    Lg,
}

impl Default for LayoutGap {
    fn default() -> Self {
        Self::Md
    }
}

impl LayoutGap {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared layout padding tokens.
pub enum LayoutPadding {
    /// No padding.
    None,
    /// Compact padding.
    Sm,
    /// Default padding.
    Md,
    /// Spacious padding.
    Lg,
}

impl Default for LayoutPadding {
    fn default() -> Self {
        Self::Md
    }
}

impl LayoutPadding {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared layout alignment tokens.
pub enum LayoutAlign {
    /// Stretch/fill alignment.
    Stretch,
    /// Start alignment.
    Start,
    /// Center alignment.
    Center,
    /// End alignment.
    End,
}

impl Default for LayoutAlign {
    fn default() -> Self {
        Self::Stretch
    }
}

impl LayoutAlign {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Stretch => "stretch",
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared layout justification tokens.
pub enum LayoutJustify {
    /// Start justification.
    Start,
    /// Center justification.
    Center,
    /// Space between items.
    Between,
    /// End justification.
    End,
}

impl Default for LayoutJustify {
    fn default() -> Self {
        Self::Start
    }
}

impl LayoutJustify {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::Between => "between",
            Self::End => "end",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared guided-step status tokens.
pub enum StepStatus {
    /// Current active step.
    Current,
    /// Completed prior step.
    Complete,
    /// Pending future step.
    Pending,
    /// Step has a validation error.
    Error,
}

impl StepStatus {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Complete => "complete",
            Self::Pending => "pending",
            Self::Error => "error",
        }
    }
}

pub(crate) fn merge_layout_class(base: &'static str, layout_class: Option<&'static str>) -> String {
    match layout_class {
        Some(layout_class) if !layout_class.is_empty() => format!("{base} {layout_class}"),
        _ => base.to_string(),
    }
}

pub(crate) fn bool_token(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
