use std::borrow::Cow;

use bevy::{
    app::{HierarchyPropagatePlugin, Propagate, PropagateSet},
    camera::visibility::RenderLayers,
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
        (&mut UiTransform, &mut ImageNode, &AccordionState),
        (With<AccordionIcon>, Changed<AccordionState>),
    >,
    asset_server: Res<AssetServer>,
) {
    for (mut node, state) in containers.iter_mut() {
        info!("checking state: {:?}", state);
        node.display = match state {
            AccordionState::Closed => Display::None,
            AccordionState::Open => Display::Flex,
        };
    }
    for (mut transform, mut image, state) in icons.iter_mut() {
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
    }
}

pub fn view<I: Iterator<Item = impl Bundle> + Send + Sync + 'static>(
    label: impl Into<String> + Clone,
    content: SpawnIter<I>,
    asset_server: AssetServer,
) -> impl Bundle {
    (
        Name::new(Cow::from(Into::<String>::into(label.clone()))),
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(8.0),
            ..default()
        },
        RenderLayers::layer(1),
        Propagate(AccordionState::Open),
        children![
            (
                Node {
                    padding: px(2.0).all(),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(8.0),
                    ..default()
                },
                RenderLayers::layer(1),
                children![
                    (
                        AccordionIcon,
                        ImageNode::new(
                            asset_server
                                .load("embedded://planetes_editor/assets/filled_triangle.png")
                        ),
                        Node {
                            height: Val::Px(8.0),
                            width: Val::Px(8.0),
                            ..default()
                        },
                        UiTransform::from_rotation(Rot2::degrees(90.0))
                    ),
                    (
                        Text::new(label),
                        TextLayout::new_with_linebreak(LineBreak::WordBoundary),
                        RenderLayers::layer(1),
                    )
                ]
            ),
            (
                Node {
                    padding: px(2.0).left(),
                    margin: px(16.0).left(),
                    display: Display::None,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8.0),
                    ..default()
                },
                AccordionContainer,
                RenderLayers::layer(1),
                Children::spawn(content)
            )
        ],
    )
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
