//! # Planetes Editor
//!
//! Builds out the UI and provides all functionality for the Planetes Game Editor.
//!
//! This can be added to any Bevy App to provide a map/scene editor that uses all your custom components
//! and behaviors, through grotesque abuse of [bevy::reflect]

use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*};
mod atoms;
pub mod editor_ui;
mod infinite_grid;
pub mod nodes;
mod sample_data;
pub mod scene;

pub use editor_ui::{MainView, plugin};

pub use planetes_scene_state::{
    self as canonical, ApplyEdit, CanonicalScene, EditHistory, EditOp, PlanetesBundle,
    PlanetesComponent, Redo, ReflectPlanetesBundle, ReflectPlanetesComponent, SyncCanonicalMessage,
    Undo,
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

/// Trait for implementing custom editor views for Components
#[reflect_trait]
pub trait EditorView {
    fn add_to_parent(&self, parent: &mut RelatedSpawnerCommands<'_, ChildOf>) -> ();
}

impl EditorView for Transform {
    fn add_to_parent(&self, parent: &mut RelatedSpawnerCommands<'_, ChildOf>) {
        parent.spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: px(2.0),
                ..default()
            },
            children![(
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(4.0),
                    ..default()
                },
                children![
                    (
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                        Text::new("Translation:"),
                    ),
                    (
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            column_gap: px(8.0),
                            ..default()
                        },
                        children![
                            (
                                Node {
                                    display: Display::Flex,
                                    flex_direction: FlexDirection::Row,
                                    row_gap: px(4.0),
                                    ..default()
                                },
                                children![
                                    Text::new("X:"),
                                    Text::new(format!("{:.2}", self.translation.x))
                                ]
                            ),
                            (
                                Node {
                                    display: Display::Flex,
                                    flex_direction: FlexDirection::Row,
                                    row_gap: px(2.0),
                                    ..default()
                                },
                                children![
                                    Text::new("Y:"),
                                    Text::new(format!("{:.2}", self.translation.y))
                                ]
                            ),
                            (
                                Node {
                                    display: Display::Flex,
                                    flex_direction: FlexDirection::Row,
                                    row_gap: px(2.0),
                                    ..default()
                                },
                                children![
                                    Text::new("Z:"),
                                    Text::new(format!("{:.2}", self.translation.z))
                                ]
                            )
                        ]
                    )
                ]
            )],
        ));
    }
}

pub mod prelude {
    pub use bevy_ui_html::html;
}
