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

#[reflect_trait]
pub trait PlanetesComponent {}

impl<T: Reflect + Component> PlanetesComponent for T {}

#[reflect_trait]
pub trait PlanetesBundle {}

impl<T: Reflect + Bundle> PlanetesBundle for T {}
