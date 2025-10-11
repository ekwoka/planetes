use bevy::prelude::*;
mod editor_ui;
mod infinite_grid;

pub use editor_ui::{MainView, plugin};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
pub enum EditorMode {
    #[default]
    Edit,
    View,
}
