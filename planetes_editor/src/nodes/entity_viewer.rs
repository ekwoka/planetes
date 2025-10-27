use std::{any::TypeId, iter::once};

use bevy::{
    camera::visibility::RenderLayers,
    ecs::{component::ComponentId, relationship::RelatedSpawnerCommands},
    prelude::*,
    reflect::{StructInfo, TupleStructInfo, TypeInfo},
};

use crate::{ReflectEditorView, ReflectPlanetesComponent, editor_ui::Capitalize, nodes::accordion};
pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (update_entity_viewer, update_component_editor).chain(),
    );
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
    assets: Res<AssetServer>,
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
                Some((
                    component.id(),
                    format!("{}", component.name().shortname()),
                    type_id,
                ))
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
                    for (id, name, type_id) in components {
                        parent.spawn(accordion::view(
                            name,
                            SpawnIter(once(component_editor(id, type_id))),
                            assets.clone(),
                        ));
                    }
                });
        });
}

#[derive(Component)]
pub struct ComponentEditor((ComponentId, TypeId));

pub fn component_editor(id: ComponentId, type_id: TypeId) -> impl Bundle {
    (
        ComponentEditor((id, type_id)),
        Node {
            padding: px(2.0).left(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(4.0),
            width: percent(100.0),
            ..default()
        },
        children![(
            Text::new(format!("{id:?}")),
            TextLayout::new_with_linebreak(LineBreak::WordBoundary),
        )],
    )
}

pub fn update_component_editor(
    mut commands: Commands,
    target: Single<Entity, With<ViewedBy>>,
    editors: Query<(Entity, &ComponentEditor), Changed<ComponentEditor>>,
    registry: Res<AppTypeRegistry>,
    world: &World,
) {
    let registry = registry.read();
    for (editor, &ComponentEditor((id, type_id))) in editors {
        let Some(registration) = registry.get(type_id) else {
            continue;
        };
        let Some(component_data) = registration.data::<ReflectComponent>() else {
            continue;
        };
        let Some(type_info) = registry.get_type_info(type_id) else {
            continue;
        };
        let Ok(entity) = world.get_entity(*target) else {
            continue;
        };
        let Some(reflected) = component_data.reflect(entity) else {
            continue;
        };
        let reflected_editor_view = registration
            .data::<ReflectEditorView>()
            .and_then(|editor_view| editor_view.get(reflected));

        commands
            .entity(editor)
            .despawn_children()
            .with_children(|parent| {
                parent.spawn((
                    Text::new(format!("{id:?}")),
                    TextLayout::new_with_linebreak(LineBreak::WordBoundary),
                ));
                if let Some(editor_view) = reflected_editor_view {
                    editor_view.add_to_parent(parent);
                } else {
                    spawn_component_editor(parent, type_info, reflected);
                }
            });
    }
}

fn spawn_component_editor(
    parent: &mut RelatedSpawnerCommands<'_, ChildOf>,
    type_info: &TypeInfo,
    reflect: &dyn Reflect,
) {
    match type_info {
        TypeInfo::Struct(info) => spawn_struct_editor(parent, info, reflect),
        _ => spawn_empty_component(parent),
    };
}

fn spawn_empty_component(parent: &mut RelatedSpawnerCommands<'_, ChildOf>) {
    parent.spawn((
        Text::new("Empty"),
        TextLayout::new_with_linebreak(LineBreak::WordBoundary),
    ));
}

fn spawn_struct_editor(
    parent: &mut RelatedSpawnerCommands<'_, ChildOf>,
    info: &StructInfo,
    reflect: &dyn Reflect,
) {
    let struct_data = reflect.reflect_ref().as_struct().unwrap();
    parent
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(2.0),
            ..default()
        })
        .with_children(|parent| {
            info.iter().for_each(|field| {
                let name = format!("{}: ", field.name().capitalize_words());
                let value = struct_data.field(field.name());
                parent
                    .spawn(Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        row_gap: px(2.0),
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn((
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                            Text::new(name),
                        ));
                        match value {
                            None => {
                                parent.spawn(Text::new("Unknown Field"));
                            }
                            Some(value) => match value.get_represented_type_info() {
                                Some(TypeInfo::TupleStruct(info)) => {
                                    spawn_reflected_tuple_struct(parent, info, value);
                                }
                                Some(TypeInfo::Struct(info)) => {
                                    spawn_reflected_struct(parent, info, value);
                                }
                                Some(TypeInfo::Tuple(_)) => {
                                    parent.spawn(Text::new("Unknown Tuple"));
                                }
                                Some(TypeInfo::List(_)) => {
                                    parent.spawn(Text::new("Unknown List"));
                                }
                                _ => {
                                    parent.spawn(Text::new("Unknown Type"));
                                }
                            },
                        };
                    });
            });
        });
}

fn spawn_reflected_tuple_struct(
    parent: &mut RelatedSpawnerCommands<'_, ChildOf>,
    info: &TupleStructInfo,
    reflect: &dyn PartialReflect,
) {
    let name = info.type_path_table().ident().unwrap_or("Unknown Ident");
    parent
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: px(4.0),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(Text::new(name));
            parent
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(2.0),
                    ..default()
                })
                .with_children(|parent| {
                    let tuple_struct = reflect.reflect_ref().as_tuple_struct().unwrap();
                    tuple_struct.iter_fields().for_each(|field| {
                        parent.spawn(Text::new(format!("{:?}", field)));
                    })
                });
        });
}

fn spawn_reflected_struct(
    parent: &mut RelatedSpawnerCommands<'_, ChildOf>,
    info: &StructInfo,
    reflect: &dyn PartialReflect,
) {
    let name = info.type_path_table().ident().unwrap_or("Unknown Ident");
    parent
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: px(16.0),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(Text::new(name));
            parent
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8.0),
                    ..default()
                })
                .with_children(|parent| {
                    let reflect_struct = reflect.reflect_ref().as_struct().unwrap();
                    info.iter()
                        .zip(reflect_struct.iter_fields())
                        .for_each(|(field, value)| {
                            parent.spawn((
                                Node {
                                    display: Display::Flex,
                                    flex_direction: FlexDirection::Row,
                                    column_gap: px(4.0),
                                    ..default()
                                },
                                children![
                                    Text::new(field.name().capitalize_words().to_string()),
                                    Text::new(format!("{:?}", value))
                                ],
                            ));
                        })
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
