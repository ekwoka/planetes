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
        highlight_selected_checkbox, highlight_selected_input, input_field, on_checkbox_change,
    },
    nodes::{accordion, component_editor},
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
    );
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
    names: Query<&Name>,
    canonical_scene: Res<CanonicalScene>,
    assets: Res<AssetServer>,
) {
    let (editor, &Viewing(target)) = *entity_viewer;

    let Some(components) = canonical_scene.get_entity_components(target) else {
        return;
    };

    info!("Found Components: {}", components.len());
    let components = components
        .values()
        .filter(|component| component.name() != &"bevy_ecs::name::Name".into())
        .map(|component| {
            accordion::view(
                component.name().shortname().to_string(),
                SpawnIter(once(component_editor::base(component.type_id()))),
                assets.clone(),
            )
        })
        .collect::<Vec<_>>();

    commands
        .entity(editor)
        .despawn_children()
        .with_children(move |parent| {
            parent.spawn(html! {
                <div
                  display="flex"
                  flex-direction="row"
                  align-items={AlignItems::Center}
                  oninput={update_name}
                >
                    <span>"Selected: "</span>
                    {
                        input_field::<String>(if let Ok(name) = names.get(target) {
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
    target: Single<Entity, With<ViewedBy>>,
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
        if let Ok(mut name) = names.get_mut(*target) {
            info!("Updating Name");
            *name = Name::new(field.value.clone());
        } else {
            info!("Adding new Name");
            commands
                .entity(*target)
                .insert(Name::new(field.value.clone()));
        }
    }
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
