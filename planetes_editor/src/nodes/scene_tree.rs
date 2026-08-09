//! Rendering of the Scene Tree UI Node

use crate::{
    nodes::{
        accordion,
        entity_viewer::{EntityEditor, Viewing},
    },
    prelude::*,
};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_ui_html::HtmlComponent;
use planetes_scene_state::{CanonicalScene, ReflectHiddenComponent};

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, update_tree)
        .add_observer(select_entity);
}

#[derive(Component, HtmlComponent)]
pub struct SceneTreeView;

#[derive(Component, HtmlComponent)]
pub struct SceneTreeBranch;

pub fn view() -> impl Bundle {
    html_bundle! {
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
    asset_server: Res<AssetServer>,
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

                    let asset_server = asset_server.clone();
                    view_commands
                        .despawn_children()
                        .with_children(move |parent| {
                            parent.spawn(accordion::view(
                                "Root:",
                                SpawnIter(
                                    root_entities
                                        .into_iter()
                                        .filter_map(|entity| {
                                            branch(
                                                entity,
                                                scene_children.clone(),
                                                asset_server.clone(),
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                        .into_iter(),
                                ),
                                asset_server,
                            ));
                        });
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
    asset_server: AssetServer,
) -> Option<impl Bundle> {
    if let Some((name, children)) = scene_children.get(&target_entity) {
        let name = name.clone().map_or_else(
            || format!("{target_entity}"),
            |name| format!("{name} ({target_entity})"),
        );
        let children = children.clone();
        let text = format!("{name}:");
        Some(html_bundle! {
            <SceneTreeBranch
                padding-left="2px"
                    flex-grow="0"
                    flex-shrink="1"
                display="flex"
                flex-direction="col"
                row-gap="8px"
                width="100%"
                components={Represents(target_entity)}>
                <with>
                {
                    if let Some(children) = children && !children.is_empty() {
                        parent.spawn(accordion::view(
                            text,
                            SpawnIter(children.into_iter().filter_map(|entity| {
                                branch(entity, scene_children.clone(), asset_server.clone())
                            })
                            .collect::<Vec<_>>()
                            .into_iter()),
                            asset_server.clone(),
                        ));
                    } else {
                        parent.spawn(html_bundle! {
                            <div
                                name={name}
                                padding="2px"
                                display="flex"
                                flex-direction="row"
                                align-items={AlignItems::Center}
                                column-gap="8px">
                                <img src={asset_server
                                    .load("embedded://planetes_editor/assets/file_icon.png")}
                                    height="10px"/>
                                <span>{text}</span>
                            </div>
                        });
                    }
                }
                </with>
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

#[derive(Component)]
#[relationship(relationship_target = RepresentedBy)]
pub struct Represents(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = Represents, linked_spawn)]
#[reflect(HiddenComponent)]
pub struct RepresentedBy(Entity);
