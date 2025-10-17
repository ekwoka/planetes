use crate::{ReflectPlanetesComponent, scene::EditorScene};
use bevy::{camera::visibility::RenderLayers, prelude::*};

#[derive(Component)]
pub struct SceneTreeView;

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

pub fn update(
    mut commands: Commands,
    scene_tree_view: Query<Entity, With<SceneTreeView>>,
    registry: Res<AppTypeRegistry>,
    scene_root: Single<Entity, (With<EditorScene>, Without<SceneTreeView>)>,
    children: Query<&Children>,
    world: &World,
) {
    let registry = registry.read();
    let allowed_types = registry
        .iter_with_data::<ReflectPlanetesComponent>()
        .map(|(type_reg, _)| type_reg.type_id())
        .collect::<Vec<_>>();

    let mut entity_stack = vec![(*scene_root, 0.0)];
    let mut entities = Vec::<Entity>::new();

    while let Some((entity, depth)) = entity_stack.pop() {
        info!("Checking entity: {:?}", entity);
        let component_names: String = world
            .inspect_entity(entity)
            .ok()
            .map(|component_iter| {
                component_iter
                    .filter_map(|component| {
                        component.type_id().and_then(|type_id| {
                            if allowed_types.contains(&type_id) {
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
        let text = if component_names.len() > 0 {
            if depth > 0.0 {
                format!("> {entity}:\n  - {component_names}")
            } else {
                format!("{entity}:\n  - {component_names}")
            }
        } else {
            format!("{entity}:")
        };
        let text_entity = commands
            .spawn((
                Node {
                    padding: px(depth * 16.0).left().with_top(px(8.0)),
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
            ))
            .id();
        entities.push(text_entity);
        if let Ok(children) = children.get(entity) {
            for child in children.iter() {
                info!("Child entity: {:?}", child);
                entity_stack.push((child, depth + 1.0));
            }
        }
    }

    for view in scene_tree_view.iter() {
        if let Ok(mut view_commands) = commands.get_entity(view) {
            view_commands.despawn_children().replace_children(&entities);
        }
    }
}
