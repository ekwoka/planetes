/// Trait for implementing custom editor views for Components
use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*};

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
