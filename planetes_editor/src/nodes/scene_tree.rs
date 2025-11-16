use crate::{
    nodes::{
        accordion,
        entity_viewer::{EntityEditor, Viewing},
    },
    prelude::*,
    scene::EditorScene,
};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (update_tree, update_branches))
        .add_observer(select_entity);
}

#[derive(Component)]
pub struct SceneTreeView;

#[derive(Component)]
pub struct SceneTreeBranch;

pub fn view() -> impl Bundle {
    html! {
        <SceneTreeView
            padding="8px"
            flex-grow="1"
            flex-shrink="1"
            display="Flex"
            flex-direction={FlexDirection::Column}
            column-gap="8px"
            width="100%"
            height="100%">
        </SceneTreeView>
    }
}

pub fn update_tree(
    mut commands: Commands,
    scene_tree_view: Single<Entity, With<SceneTreeView>>,
    scene_root: Single<&Children, (With<EditorScene>, Changed<Children>)>,
    asset_server: Res<AssetServer>,
) {
    info!("Updating scene tree");
    let branch_entities = scene_root.iter().collect::<Vec<_>>();
    if let Ok(mut view_commands) = commands.get_entity(*scene_tree_view) {
        view_commands.despawn_children().with_children(|parent| {
            parent.spawn(accordion::view(
                "Root:",
                SpawnIter(branch_entities.into_iter().map(branch)),
                asset_server.clone(),
            ));
        });
    }
}

pub fn update_branches(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    elements_in_scene: Query<
        (Entity, Option<&Name>, Option<&Children>, &RepresentedBy),
        Or<(Changed<Children>, Changed<RepresentedBy>)>,
    >,
) {
    if elements_in_scene.is_empty() {
        return;
    }

    info!("Updating ChangedBranches");

    for (entity, name, children, represented_by) in elements_in_scene.iter() {
        let child_entities = children
            .map(|children| children.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        if let Some(represented_by) = represented_by.iter().next()
            && let Ok(mut branch_view) = commands.get_entity(represented_by)
        {
            let name =
                name.map_or_else(|| format!("{entity}"), |name| format!("{name} ({entity})"));
            let text = format!("{name}:");
            branch_view.despawn_children().with_children(|parent| {
                if child_entities.is_empty() {
                    parent.spawn(html! {
                        <div
                            name={name}
                            padding="2px"
                            display="Flex"
                            flex-direction={FlexDirection::Row}
                            align-items={AlignItems::Center}
                            column-gap="8px">
                            <img src={asset_server
                                .load("embedded://planetes_editor/assets/file_icon.png")}
                                height="10px"/>
                            <span>{text}</span>
                        </div>
                    });
                } else {
                    parent.spawn(accordion::view(
                        text,
                        SpawnIter(child_entities.into_iter().map(branch)),
                        asset_server.clone(),
                    ));
                }
            });
        };
    }
}

pub fn branch(target_entity: Entity) -> impl Bundle {
    html! {
        <SceneTreeBranch
            padding-left="2px"
                flex-grow="0"
                flex-shrink="1"
            display="Flex"
            flex-direction={FlexDirection::Column}
            row-gap="8px"
            width="100%"
            components={Represents(target_entity)}>
            <span>{format!("Child {target_entity}")}</span>
        </SceneTreeBranch>
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

#[derive(Component)]
#[relationship_target(relationship = Represents, linked_spawn)]
pub struct RepresentedBy(Entity);
