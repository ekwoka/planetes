//! A simple Accordion UI component

use bevy::{
    app::{HierarchyPropagatePlugin, Propagate, PropagateSet},
    feathers::cursor::EntityCursor,
    prelude::*,
    ui_widgets::{Activate, Button},
    window::SystemCursorIcon,
};

use crate::{
    nodes::{
        entity_viewer::{EntityEditor, Viewing},
        scene_tree::Represents,
    },
    prelude::*,
};

pub fn plugin(app: &mut App) {
    app.add_plugins(HierarchyPropagatePlugin::<AccordionState>::new(Update))
        .add_systems(
            Update,
            update_accordion.after(PropagateSet::<AccordionState>::default()),
        )
        .add_observer(actuate_accordion);
}

pub fn update_accordion(
    mut containers: Query<
        (&mut Node, &AccordionState),
        (With<AccordionContainer>, Changed<AccordionState>),
    >,
    mut icons: Query<
        (&mut UiTransform, &AccordionState),
        (With<AccordionIcon>, Changed<AccordionState>),
    >,
) {
    for (mut node, state) in containers.iter_mut() {
        info!("checking state: {:?}", state);
        node.display = match state {
            AccordionState::Closed => Display::None,
            AccordionState::Open => Display::Flex,
        };
    }
    for (mut transform, state) in icons.iter_mut() {
        match state {
            AccordionState::Closed => {
                transform.rotation = Rot2::degrees(90.0);
            }
            AccordionState::Open => {
                transform.rotation = Rot2::degrees(180.0);
            }
        }
    }
}

pub fn actuate_accordion(
    event: On<Activate>,
    mut commands: Commands,
    accordions: Query<(Entity, &Propagate<AccordionState>)>,
    containers: Query<&AccordionContainer>,
    parents: Query<&ChildOf>,
    representations: Query<&Represents>,
    editors: Query<Entity, With<EntityEditor>>,
) {
    if let Some((entity, state)) = parents
        .iter_ancestors(event.entity)
        .take_while(|&ancestor| containers.get(ancestor).ok().is_none())
        .filter_map(|ancestor| accordions.get(ancestor).ok())
        .next()
    {
        commands
            .entity(entity)
            .try_insert(Propagate(state.0.toggled()));
        if let Some(&Represents(scene_entity)) = parents
            .iter_ancestors(entity)
            .filter_map(|ancestor| representations.get(ancestor).ok())
            .next()
        {
            if let Ok(editor) = editors.single() {
                commands.entity(editor).insert(Viewing(scene_entity));
            }
        }
    }
}

pub fn scene(label: impl Into<String> + Clone, content: impl SceneList) -> impl Scene + Sized {
    html! {
        <div
            name={Into::<String>::into(label.clone())}
            display="flex"
            flex-direction="col"
            row-gap="8px"
            components={template(|_| Ok(Propagate(AccordionState::Closed)))}>
            <AccordionControl>
                <AccordionIcon/>
                <span>{label}</span>
            </AccordionControl>
            <AccordionContainer>
                {{content}}
            </AccordionContainer>
        </div>
    }
}

#[derive(SceneComponent, Clone, Default)]
pub struct AccordionControl;

impl AccordionControl {
    fn scene() -> impl Scene {
        html! {
            <div
                padding="2px"
                display="flex"
                flex-direction="row"
                align-items={AlignItems::Center}
                column-gap="8px"
                components={
                    (Button, EntityCursor::System(SystemCursorIcon::Pointer))
                } />
        }
    }
}

#[derive(Component, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum AccordionState {
    Open,
    #[default]
    Closed,
}

impl AccordionState {
    pub fn default_closed() -> Self {
        Self::Closed
    }

    pub fn toggle(&mut self) {
        *self = match *self {
            AccordionState::Closed => AccordionState::Open,
            AccordionState::Open => AccordionState::Closed,
        };
    }
    pub fn toggled(&self) -> Self {
        match *self {
            AccordionState::Closed => AccordionState::Open,
            AccordionState::Open => AccordionState::Closed,
        }
    }
}

#[derive(SceneComponent, Clone, Default)]
pub struct AccordionContainer;

impl AccordionContainer {
    fn scene() -> impl Scene {
        html! {
            <div
                padding-left="2px"
                margin-left="16px"
                display="none"
                flex-direction="col"
                row-gap="8px"/>
        }
    }
}

#[derive(SceneComponent, Default, Clone)]
pub struct AccordionIcon;

impl AccordionIcon {
    fn scene() -> impl Scene {
        html! {
            <img
                src="embedded://planetes_editor/assets/filled_triangle.png"
                height="8px"
                width="8px"
                components={UiTransform::from_rotation(Rot2::degrees(90.0))}/>
        }
    }
}
