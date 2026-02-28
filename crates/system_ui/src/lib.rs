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
    AppShell, Button, ButtonSize, ButtonVariant, Cluster, ColorField, DisclosurePanel, Elevation,
    ElevationLayer, FieldVariant, Grid, Heading, LayoutAlign, LayoutGap, LayoutJustify,
    LayoutPadding, MenuBar, Panel, ProgressBar, ProgressVariant, RangeField, SelectField, Stack,
    StatusBar, StepFlow, StepFlowActions, StepFlowHeader, StepFlowStep, StepStatus, Surface,
    SurfaceVariant, Text, TextArea, TextField, TextRole, TextTone, ToolBar,
};

/// Convenience imports for application crates consuming the shared primitive set.
pub mod prelude {
    pub use crate::{
        AppShell, Button, ButtonSize, ButtonVariant, Cluster, ColorField, DisclosurePanel,
        Elevation, ElevationLayer, FieldVariant, Grid, Heading, Icon, IconName, IconSize,
        LayoutAlign, LayoutGap, LayoutJustify, LayoutPadding, MenuBar, Panel, ProgressBar,
        ProgressVariant, RangeField, SelectField, Stack, StatusBar, StepFlow, StepFlowActions,
        StepFlowHeader, StepFlowStep, StepStatus, Surface, SurfaceVariant, Text, TextArea,
        TextField, TextRole, TextTone, ToolBar,
    };
}
