use bevy::{
    app::{HierarchyPropagatePlugin, Propagate, PropagateSet},
    prelude::*,
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
    mut event: On<Pointer<Click>>,
    mut commands: Commands,
    accordions: Query<&Propagate<AccordionState>>,
    containers: Query<&AccordionContainer>,
    parents: Query<&ChildOf>,
    representations: Query<&Represents>,
    editors: Query<Entity, With<EntityEditor>>,
) {
    info!("Click Detected on {:?}", event.entity);
    if containers.contains(event.entity) {
        event.propagate(false);
    } else if let Ok(state) = accordions.get(event.entity) {
        info!("Actuating Accordion: {:?}", state.0);
        commands
            .entity(event.entity)
            .try_insert(Propagate(state.0.toggled()));
        event.propagate(false);
        if let Some(&Represents(scene_entity)) = parents
            .iter_ancestors(event.entity)
            .filter_map(|ancestor| representations.get(ancestor).ok())
            .next()
        {
            info!("Found representing ancestor");
            if let Ok(editor) = editors.single() {
                commands.entity(editor).insert(Viewing(scene_entity));
            }
        }
    }
}

pub fn view<I: Iterator<Item = impl Bundle> + Send + Sync + 'static>(
    label: impl Into<String> + Clone,
    content: SpawnIter<I>,
    asset_server: AssetServer,
) -> impl Bundle {
    html! {
        <div
            name={Into::<String>::into(label.clone())}
            display="flex"
            flex-direction="col"
            row-gap="8px"
            components={Propagate(AccordionState::Closed)}>
            <Button
                padding="2px"
                display="flex"
                flex-direction="row"
                align-items={AlignItems::Center}
                column-gap="8px">
                <img
                    src={asset_server
                        .load("embedded://planetes_editor/assets/filled_triangle.png")}
                    height="8px"
                    width="8px"
                    components={
                        (
                            AccordionIcon,
                            UiTransform::from_rotation(Rot2::degrees(90.0))
                        )
                    }/>
                <span>{label}</span>
            </Button>
            <AccordionContainer
                padding-left="2px"
                margin-left="16px"
                display="none"
                flex-direction="col"
                row-gap="8px"
                components={Children::spawn(content)}/>
        </div>
    }
}

#[derive(Component, PartialEq, Eq, Clone, Copy, Debug)]
pub enum AccordionState {
    Open,
    Closed,
}

impl AccordionState {
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

#[derive(Component)]
pub struct AccordionContainer;

#[derive(Component)]
pub struct AccordionIcon;
