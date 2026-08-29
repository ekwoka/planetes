//! Rendering of the Scene Tree UI Node

use crate::{
    nodes::{
        accordion,
        entity_viewer::{EntityEditor, Viewing},
    },
    prelude::*,
};
use bevy::{platform::collections::HashMap, prelude::*};
use planetes_scene_state::{CanonicalScene, ReflectHiddenComponent};

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, update_tree)
        .add_observer(select_entity);
}

#[derive(SceneComponent, Default, Clone)]
pub struct SceneTreeView;

impl SceneTreeView {
    fn scene() -> impl Scene {
        html! {
            <div
                padding="8px"
                flex-grow="1"
                flex-shrink="1"
                display="flex"
                flex-direction="col"
                column-gap="8px"
                width="100%"
                height="100%"/>
        }
    }
}

#[derive(SceneComponent, Clone, Default)]
pub struct SceneTreeBranch;

impl SceneTreeBranch {
    fn scene() -> impl Scene {
        html! {
            <div padding-left="2px"
            flex-grow="0"
            flex-shrink="1"
            display="flex"
            flex-direction="col"
            row-gap="8px"
            width="100%" />
        }
    }
}

pub fn scene() -> impl Scene {
    html! {
        <SceneTreeView
            padding="8px"
            flex-grow="1"
            flex-shrink="1"
            display="flex"
            flex-direction="col"
            column-gap="8px"
            width="100%"
            height="100%">
        </SceneTreeView>
    }
}

#[derive(Message)]
pub struct UpdateSceneTree {
    /// The entity to sync from.
    pub entity: Entity,
}

pub fn update_tree(
    mut commands: Commands,
    mut messages: MessageReader<AssetEvent<DynamicWorld>>,
    scene_tree_view: Single<Entity, With<SceneTreeView>>,
    canonical_scene: Res<CanonicalScene>,
    scenes: Res<Assets<DynamicWorld>>,
) {
    for message in messages.read() {
        match message {
            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id } => {
                if !canonical_scene.is_id(id) {
                    continue;
                }
                info!("Updating scene tree");
                if let Ok(mut view_commands) = commands.get_entity(*scene_tree_view) {
                    let scene_children: HashMap<Entity, (Option<Name>, Option<Vec<Entity>>)> =
                        canonical_scene
                            .get_scene(&scenes)
                            .map(|scene| {
                                scene
                                    .entities
                                    .iter()
                                    .map(|entity| {
                                        let name = entity
                                            .components
                                            .iter()
                                            .find(|component| component.represents::<Name>())
                                            .and_then(|name| {
                                                Name::from_reflect(name.as_partial_reflect())
                                            });
                                        let children = entity
                                            .components
                                            .iter()
                                            .find(|component| component.represents::<Children>())
                                            .and_then(|children| {
                                                Children::from_reflect(
                                                    children.as_partial_reflect(),
                                                )
                                                .map(|children| {
                                                    children.iter().collect::<Vec<Entity>>()
                                                })
                                            });
                                        (entity.entity, (name, children))
                                    })
                                    .collect::<HashMap<_, _>>()
                            })
                            .unwrap_or_default();
                    let root_entities = canonical_scene
                        .get_root_entities(&scenes)
                        .iter()
                        .map(|entity| entity.entity)
                        .collect::<Vec<Entity>>();

                    let mut children: Vec<Box<dyn Scene>> = vec![];
                    root_entities
                        .into_iter()
                        .filter_map(|entity| branch(entity, scene_children.clone()))
                        .for_each(|scene| children.push(Box::new(scene)));
                    view_commands
                        .despawn_children()
                        .queue_spawn_related_scenes::<Children>(vec![accordion::scene(
                            "Root:", children,
                        )]);
                } else {
                    info!("No SceneTreeView");
                }
            }
            _ => {}
        }
    }
}

pub fn branch(
    target_entity: Entity,
    scene_children: HashMap<Entity, (Option<Name>, Option<Vec<Entity>>)>,
) -> Option<impl Scene> {
    if let Some((name, children)) = scene_children.get(&target_entity) {
        let name = name.clone().map_or_else(
            || format!("{target_entity}"),
            |name| format!("{name} ({target_entity})"),
        );
        let children = children.clone();
        let text = format!("{name}:");
        let child: Box<dyn Scene> = if let Some(children) = children
            && !children.is_empty()
        {
            let mut content: Vec<Box<dyn Scene>> = vec![];
            children
                .into_iter()
                .filter_map(|entity| branch(entity, scene_children.clone()))
                .for_each(|scene| content.push(Box::new(scene)));
            Box::new(accordion::scene(text, content))
        } else {
            (html! {
                <div
                    name={name}
                    padding="2px"
                    display="flex"
                    flex-direction="row"
                    align-items={AlignItems::Center}
                    column-gap="8px">
                    <span>{text}</span>
                </div>
            })
            .into()
        };
        Some(html! {
            <SceneTreeBranch components={Represents(target_entity)}>
                {child}
            </SceneTreeBranch>
        })
    } else {
        None
    }
}

pub fn select_entity(
    mut event: On<Pointer<Click>>,
    mut commands: Commands,
    representations: Query<&Represents>,
    editors: Query<Entity, With<EntityEditor>>,
) {
    if let Ok(&Represents(scene_entity)) = representations.get(event.entity) {
        info!("Found representing ancestor");
        if let Ok(editor) = editors.single() {
            commands.entity(editor).insert(Viewing(scene_entity));
        }
        event.propagate(false);
    }
}

#[derive(Component, Clone, FromTemplate)]
#[relationship(relationship_target = RepresentedBy)]
pub struct Represents(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = Represents, linked_spawn)]
#[reflect(HiddenComponent)]
pub struct RepresentedBy(Entity);
