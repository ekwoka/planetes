//! # Planetes Editor
//!
//! Builds out the UI and provides all functionality for the Planetes Game Editor.
//!
//! This can be added to any Bevy App to provide a map/scene editor that uses all your custom components
//! and behaviors, through grotesque abuse of [bevy::reflect]

use bevy::prelude::*;
mod atoms;
pub mod editor_ui;
pub mod events;
pub mod nodes;
mod sample_data;
pub mod scene;

pub use editor_ui::{MainView, plugin};

pub use planetes_scene_state::{
    self as canonical, ApplyEdit, CanonicalScene, EditHistory, EditOp, PlanetesBundle,
    PlanetesComponent, Redo, ReflectPlanetesBundle, ReflectPlanetesComponent, Undo,
};

/// State for tracking if the Editor is available or not.
///
/// Most Editor related systems disable themselves when the Editor is not available
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
pub enum EditorMode {
    #[default]
    Edit,
    View,
}

pub mod prelude {
    pub use bevy_ui_html::html;
}
