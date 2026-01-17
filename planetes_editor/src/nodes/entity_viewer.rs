//! Core Entity Viewer/Editor

use std::iter::once;

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusedInput, InputFocus},
    prelude::*,
};
use planetes_input::prelude::*;
use planetes_scene_state::CanonicalScene;

use crate::{
    atoms::{
        button, highlight_selected_checkbox, highlight_selected_input, input_field,
        on_checkbox_change,
    },
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
            highlight_selected_checkbox,
            on_checkbox_change,
        )
            .chain(),
    )
    .add_observer(handle_update_entity_viewer);
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
                  flex-direction="row"
                  align-items={AlignItems::Center}
                  onClick={|_event: On<Pointer<Click>>, mut commands: Commands, target: Single<&Viewing>| {
                      commands.trigger(OpenAddComponent { entity: target.0 });
                  }}
                >
                    <span>"Add Component: "</span>
                    {button::render("+")}
                </div>
            });
            parent.spawn(html! {
                <div
                   display="flex"
                   flex-direction="col"
                   flex-grow="1"
                   row-gap="4px">
                   <iter>
                    {components.into_iter()}
                   </iter>
                </div>
            });
        });
}

#[derive(Event)]
pub struct UpdateEntityViewer(pub Entity);

pub fn handle_update_entity_viewer(
    event: On<UpdateEntityViewer>,
    mut commands: Commands,
    entity_viewer: Single<(Entity, &Viewing), With<EntityEditor>>,
    scenes: Res<Assets<DynamicScene>>,
    canonical_scene: Res<CanonicalScene>,
    assets: Res<AssetServer>,
) {
    let (editor, &Viewing(target)) = *entity_viewer;
    if event.0 != target {
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
                  flex-direction="row"
                  align-items={AlignItems::Center}
                  onClick={|_event: On<Pointer<Click>>, mut commands: Commands, target: Single<&Viewing>| {
                      commands.trigger(OpenAddComponent { entity: target.0 });
                  }}
                >
                    <span>"Add Component: "</span>
                    {button::render("+")}
                </div>
            });
            parent.spawn(html! {
                <div
                   display="flex"
                   flex-direction="col"
                   flex-grow="1"
                   row-gap="4px">
                   <iter>
                    {components.into_iter()}
                   </iter>
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
