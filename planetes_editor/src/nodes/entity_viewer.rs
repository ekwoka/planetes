//! Core Entity Viewer/Editor

use std::{any::TypeId, iter::once};

use bevy::{
    app::Propagate,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusedInput, InputFocus},
    platform::collections::HashSet,
    prelude::*,
    ui_widgets::Activate,
};
use planetes_input::prelude::*;
use planetes_scene_state::{CanonicalScene, ComponentsChanged};

use crate::{
    atoms::{button, highlight_selected_input, input_field},
    events::AddComponentToEntity,
    nodes::{accordion, component_editor, component_selector::OpenAddComponent},
    prelude::*,
};
pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            update_entity_viewer,
            component_editor::update_component_editor,
            highlight_selected_input,
        )
            .chain(),
    )
    .add_observer(handle_components_changed)
    .add_observer(update_required_components);
}

pub fn view() -> impl Bundle {
    html! {
        <EntityViewer
            padding="8px"
            flex-grow="1"
            flex-shrink="1"
            display="flex"
            flex-direction="col"
            row-gap="8px"
            width="100%"
            height="100%"
        >
            <div padding="2px">
               "Entity Viewer"
            </div>
            <EntityEditor
                padding="2px"
                flex-grow="1"
                flex-shrink="1"
                display="flex"
                flex-direction="col"
                row-gap="8px"
            >
              "No Entity Selected"
            </EntityEditor>
        </EntityViewer>
    }
}

pub fn update_entity_viewer(
    mut commands: Commands,
    entity_viewer: Single<(Entity, &Viewing), (Changed<Viewing>, With<EntityEditor>)>,
    canonical_scene: Res<CanonicalScene>,
    scenes: Res<Assets<DynamicScene>>,
    assets: Res<AssetServer>,
) {
    let (editor, &Viewing(target)) = *entity_viewer;

    let Some(entity) = canonical_scene.get_entity(&scenes, target) else {
        return;
    };

    let components_data = &entity.components;

    info!("Found Components: {}", components_data.len());
    let components = components_data
        .iter()
        .filter(|component| !component.represents::<Name>())
        .filter_map(|component| {
            component.get_represented_type_info().map(|type_info| {
                accordion::view(
                    type_info
                        .type_path_table()
                        .ident()
                        .map(|ident| ident.to_string())
                        .unwrap_or("Unknown".into()),
                    SpawnIter(once(component_editor::base(type_info.type_id()))),
                    assets.clone(),
                )
            })
        })
        .collect::<Vec<_>>();
    let name = components_data
        .iter()
        .find(|component| component.represents::<Name>())
        .and_then(|name| Name::from_reflect(name.as_partial_reflect()));
    commands
        .entity(editor)
        .despawn_children()
        .with_children(move |parent| {
            parent.spawn(html! {
                <div
                  display="flex"
                  flex-direction="row"
                  align-items={AlignItems::Center}
                  onInput={update_name}
                >
                    <span>"Selected: "</span>
                    {
                        input_field::<String>(if let Some(name) = name {
                            format!("{name}")
                        } else {
                            format!("{target}")
                        })
                    }
                </div>
            });


            parent.spawn(html! {
                <div
                   display="flex"
                   flex-direction="col"
                   row-gap="4px">
                   <iter>
                    {components.into_iter()}
                   </iter>
                </div>
            });
            parent.spawn(html! {
                <div>
                    {button::render("+ Add Component", |_event: On<Activate>, mut commands: Commands, target: Single<&Viewing>| {
                        commands.trigger(OpenAddComponent { entity: target.0 });
                    })}
                </div>
            });
            parent.spawn(html! {
                <div display="flex" flex-direction="col" row-gap="4px" components={Propagate(TextColor(Color::srgb_u8(180, 180, 180)))}>
                   <span>"Required Components:"</span>
                   <div display="flex" flex-direction="col" row-gap="4px" components={RequiredComponentsUI(target)}>

                   </div>
                </div>
            });
        });
}

#[derive(Component)]
pub struct RequiredComponentsUI(pub Entity);

#[derive(Component)]
pub struct RequiredComponent(pub Entity, pub TypeId);

pub fn update_required_components(
    event: On<Add, RequiredComponentsUI>,
    query: Query<&RequiredComponentsUI>,
    world: &World,
    canonical: Res<CanonicalScene>,
    scenes: Res<Assets<DynamicScene>>,
    mut commands: Commands,
) {
    let Ok(&RequiredComponentsUI(target)) = query.get(event.entity) else {
        return;
    };
    let Some(dyn_entity) = canonical.get_entity(&scenes, target) else {
        return;
    };
    let components = world.components();
    let mut required_components = HashSet::<(String, TypeId)>::new();
    let existing_ids = dyn_entity
        .components
        .iter()
        .map(|c| c.get_represented_type_info().map(|info| info.type_id()))
        .collect::<Vec<_>>();
    for component in &dyn_entity.components {
        let Some(id) = component
            .get_represented_type_info()
            .map(|info| info.type_id())
        else {
            continue;
        };
        let Some(component_id) = components.get_id(id) else {
            continue;
        };
        let Some(component_info) = components.get_info(component_id) else {
            continue;
        };
        for required_component_id in component_info.required_components().iter_ids() {
            let Some(required_component_info) = components.get_info(required_component_id) else {
                continue;
            };
            if existing_ids.contains(&required_component_info.type_id()) {
                continue;
            }
            required_components.insert((
                required_component_info.name().shortname().to_string(),
                required_component_info.type_id().unwrap(),
            ));
        }
    }
    commands.entity(event.entity).with_children(|parent| {
        for (name, id) in required_components.into_iter() {
            parent.spawn(html! {
                <div
                    display="flex"
                    flex-direction="row"
                    justify-items="space-between"
                    align-items={AlignItems::Center}
                    column-gap="2px"
                    components={RequiredComponent(target, id)}>
                    <span>{name}</span>
                    <div flex-grow="100"/>
                    {button::render("Include", handle_include_required_component)}
                </div>
            });
        }
    });
}

pub fn handle_include_required_component(
    event: On<Activate>,
    mut commands: Commands,
    required_component_info: Query<&RequiredComponent>,
    child_of: Query<&ChildOf>,
) {
    for parent in child_of.iter_ancestors(event.entity) {
        if let Ok(required_component) = required_component_info.get(parent) {
            let RequiredComponent(entity, component_id) = required_component;
            commands.trigger(AddComponentToEntity {
                entity: *entity,
                component: *component_id,
            });
            break;
        }
    }
}

pub fn handle_components_changed(
    event: On<ComponentsChanged>,
    mut commands: Commands,
    entity_viewer: Single<(Entity, &Viewing), With<EntityEditor>>,
    scenes: Res<Assets<DynamicScene>>,
    canonical_scene: Res<CanonicalScene>,
    assets: Res<AssetServer>,
) {
    let (editor, &Viewing(target)) = *entity_viewer;
    if event.event().0 != target {
        return;
    }

    let Some(entity) = canonical_scene.get_entity(&scenes, target) else {
        return;
    };

    let components_data = &entity.components;

    info!("Found Components: {}", components_data.len());
    let components = components_data
        .iter()
        .filter(|component| !component.represents::<Name>())
        .filter_map(|component| {
            component.get_represented_type_info().map(|type_info| {
                accordion::view(
                    type_info
                        .type_path_table()
                        .ident()
                        .map(|ident| ident.to_string())
                        .unwrap_or("Unknown".into()),
                    SpawnIter(once(component_editor::base(type_info.type_id()))),
                    assets.clone(),
                )
            })
        })
        .collect::<Vec<_>>();
    let name = components_data
        .iter()
        .find(|component| component.represents::<Name>())
        .and_then(|name| Name::from_reflect(name.as_partial_reflect()));
    commands
        .entity(editor)
        .despawn_children()
        .with_children(move |parent| {
            parent.spawn(html! {
                <div
                  display="flex"
                  flex-direction="row"
                  align-items={AlignItems::Center}
                  onInput={update_name}
                >
                    <span>"Selected: "</span>
                    {
                        input_field::<String>(if let Some(name) = name {
                            format!("{name}")
                        } else {
                            format!("{target}")
                        })
                    }
                </div>
            });


            parent.spawn(html! {
                <div
                    display="flex"
                    flex-direction="col"
                    row-gap="4px">
                    <iter>
                    {components.into_iter()}
                    </iter>
                </div>
            });
            parent.spawn(html! {
                <div
                    display="block"
                >
                    <button
                       variant="normal"
                       corners="rounded"
                       onActivate={|_event: On<Activate>, mut commands: Commands, target: Single<&Viewing>| {
                           commands.trigger(OpenAddComponent { entity: target.0 });
                       }}>
                       "+ Add Component"
                    </button>
                </div>
            });
            parent.spawn(html! {
                <div display="flex" flex-direction="col" row-gap="4px" components={Propagate(TextColor(Color::srgb_u8(120, 120, 120)))}>
                    <span>"Required Components:"</span>
                    <div display="flex" flex-direction="col" row-gap="4px" components={RequiredComponentsUI(target)}>

                    </div>
                </div>
            });
        });
}

/// Handles committing an Entity Name Change
fn update_name(
    event: On<FocusedInput<KeyboardInput>>,
    mut commands: Commands,
    inputs: Query<&InputField<String>>,
    mut names: Query<&mut Name>,
    target: Single<&Viewing>,
    focused: Res<InputFocus>,
) {
    if event.input.logical_key != Key::Enter || event.input.state != ButtonState::Pressed {
        return;
    }
    info!("Typed in name input: {:?}", focused);
    if let Some(focused) = focused.0.inspect(|_| {
        info!("Focused Input Exists");
    }) && let Ok(field) = inputs.get(focused).inspect_err(|_| {
        info!("Failed to get field");
    }) {
        if let Ok(mut name) = names.get_mut(target.0) {
            info!("Updating Name");
            *name = Name::new(field.value.clone());
        } else {
            info!("Adding new Name");
            commands
                .entity(target.0)
                .insert(Name::new(field.value.clone()));
        }
    }
}

#[derive(Component)]
pub struct EntityViewer;

#[derive(Component)]
pub struct EntityEditor;

#[derive(Component)]
pub struct Viewing(pub Entity);
