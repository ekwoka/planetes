use bevy::{camera::visibility::RenderLayers, prelude::*};
pub fn plugin(app: &mut App) {
    app.add_systems(Update, update_entity_viewer);
}

pub fn view() -> impl Bundle {
    (
        EntityViewer,
        Node {
            padding: px(8.0).all(),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            column_gap: px(8.0),
            width: percent(100.0),
            height: percent(100.0),
            ..default()
        },
        RenderLayers::layer(1),
        children![
            (
                Node {
                    padding: px(2.0).all(),
                    ..default()
                },
                children![Text::new("Entity Viewer")]
            ),
            (
                EntityEditor,
                Node {
                    padding: px(2.0).all(),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    ..default()
                },
                children![Text::new("No Entity Selected")]
            )
        ],
    )
}

pub fn update_entity_viewer(
    mut commands: Commands,
    entity_viewer: Single<(Entity, &Viewing), (Changed<Viewing>, With<EntityEditor>)>,
    names: Query<&Name>,
) {
    let (editor, viewing) = *entity_viewer;
    commands
        .entity(editor)
        .despawn_children()
        .with_children(|parent| {
            parent.spawn(Text::new(if let Ok(name) = names.get(viewing.0) {
                format!("Selected: {name}")
            } else {
                format!("Selected: {}", viewing.0)
            }));
        });
}

#[derive(Component)]
pub struct EntityViewer;

#[derive(Component)]
pub struct EntityEditor;

#[derive(Component)]
#[relationship(relationship_target = ViewedBy)]
pub struct Viewing(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = Viewing)]
pub struct ViewedBy(Entity);
