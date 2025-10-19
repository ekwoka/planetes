use crate::{ReflectPlanetesComponent, scene::EditorScene};
use bevy::{camera::visibility::RenderLayers, prelude::*};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (update_tree, update_branches));
}

#[derive(Component)]
pub struct SceneTreeView;

#[derive(Component)]
pub struct SceneTreeBranch;

pub fn view() -> impl Bundle {
    (
        SceneTreeView,
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
    )
}

pub fn update_tree(
    mut commands: Commands,
    scene_tree_view: Single<Entity, With<SceneTreeView>>,
    scene_root: Single<&Children, (With<EditorScene>, Changed<Children>)>,
) {
    info!("Updating scene tree");

    if let Ok(mut view_commands) = commands.get_entity(*scene_tree_view) {
        view_commands.despawn_children().with_children(|parent| {
            parent.spawn((
                Node {
                    padding: px(8.0).left(),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8.0),
                    ..default()
                },
                children![(
                    Text::new("Root:"),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextLayout::new_with_linebreak(LineBreak::WordBoundary),
                    TextColor::from(Color::linear_rgb(0.7, 0.7, 0.7)),
                    RenderLayers::layer(1),
                )],
            ));
            scene_root.iter().for_each(|child| {
                parent.spawn(branch(child));
            })
        });
    }
}

pub fn update_branches(
    mut commands: Commands,
    registry: Res<AppTypeRegistry>,
    elements_in_scene: Query<
        (Entity, Option<&Name>, Option<&Children>, &RepresentedBy),
        Or<(Changed<Children>, Changed<RepresentedBy>)>,
    >,
    world: &World,
) {
    if elements_in_scene.is_empty() {
        return;
    }

    info!("Updating ChangedBranches");
    let registry = registry.read();
    let allowed_types = registry
        .iter_with_data::<ReflectPlanetesComponent>()
        .map(|(type_reg, _)| type_reg.type_id())
        .collect::<Vec<_>>();

    for (entity, name, children, represented_by) in elements_in_scene.iter() {
        if let Some(represented_by) = represented_by.iter().next()
            && let Ok(mut branch_view) = commands.get_entity(represented_by)
        {
            let name =
                name.map_or_else(|| format!("{entity}"), |name| format!("{name} ({entity})"));
            let component_names: String = world
                .inspect_entity(entity)
                .ok()
                .map(|component_iter| {
                    component_iter
                        .filter_map(|component| {
                            component.type_id().and_then(|type_id| {
                                if allowed_types.contains(&type_id)
                                    && component.name() != "bevy_ecs::name::Name".into()
                                {
                                    Some((format!("{}", component.name().shortname()), type_id))
                                } else {
                                    None
                                }
                            })
                        })
                        .map(|(name, type_id)| {
                            let type_info = registry
                                .get_type_info(type_id)
                                .and_then(|type_info| type_info.as_struct().ok())
                                .map(|struct_info| struct_info.field_names().join(", "));
                            format!("{name}: {type_info:?}")
                        })
                        .collect::<Vec<_>>()
                        .join("\n  - ")
                })
                .unwrap_or("No Components".to_string());
            let text = if !component_names.is_empty() {
                format!("> {name}:\n  - {component_names}")
            } else {
                format!("{name}:")
            };
            branch_view.despawn_children().with_children(|parent| {
                parent.spawn((
                    Node {
                        padding: px(8.0).left(),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(8.0),
                        ..default()
                    },
                    children![(
                        Text::new(text),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextLayout::new_with_linebreak(LineBreak::WordBoundary),
                        TextColor::from(Color::linear_rgb(0.7, 0.7, 0.7)),
                        RenderLayers::layer(1),
                    )],
                ));
                if let Some(children) = children {
                    children.iter().for_each(|child| {
                        parent.spawn(branch(child));
                    });
                }
            });
        };
    }
}

pub fn branch(target_entity: Entity) -> impl Bundle {
    (
        SceneTreeBranch,
        Node {
            padding: px(16.0).left().with_top(px(8.0)),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(8.0),
            width: percent(100.0),
            ..default()
        },
        RenderLayers::layer(1),
        Represents(target_entity),
        children![(
            Text::new(format!("Child {target_entity}")),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextLayout::new_with_linebreak(LineBreak::WordBoundary),
            TextColor::from(Color::linear_rgb(0.7, 0.7, 0.7)),
            RenderLayers::layer(1),
        )],
    )
}

#[derive(Component)]
#[relationship(relationship_target = RepresentedBy)]
pub struct Represents(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = Represents, linked_spawn)]
pub struct RepresentedBy(Entity);
