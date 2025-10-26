use bevy::{camera::visibility::RenderLayers, prelude::*};

use crate::ReflectPlanetesComponent;
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
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8.0),
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
    registry: Res<AppTypeRegistry>,
    world: &World,
) {
    let (editor, &Viewing(target)) = *entity_viewer;

    let registry = registry.read();
    let allowed_types = registry
        .iter_with_data::<ReflectPlanetesComponent>()
        .map(|(type_reg, _)| type_reg.type_id())
        .collect::<Vec<_>>();

    let Some(components) = world.inspect_entity(target).ok() else {
        return;
    };

    let components = components.filter_map(|component| {
        component.type_id().and_then(|type_id| {
            if allowed_types.contains(&type_id) && component.name() != "bevy_ecs::name::Name".into()
            {
                Some((format!("{}", component.name().shortname()), type_id))
            } else {
                None
            }
        })
    });

    commands
        .entity(editor)
        .despawn_children()
        .with_children(|parent| {
            parent.spawn((
                Node::default(),
                Text::new(if let Ok(name) = names.get(target) {
                    format!("Selected: {name}")
                } else {
                    format!("Selected: {target}")
                }),
            ));

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    row_gap: px(4.0),
                    ..default()
                },))
                .with_children(|parent| {
                    for (name, _) in components {
                        parent.spawn(Text::new(name));
                    }
                });
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
