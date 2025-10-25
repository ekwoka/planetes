use crate::{
    nodes::{
        accordion,
        entity_viewer::{EntityEditor, Viewing},
    },
    scene::EditorScene,
};
use bevy::{camera::visibility::RenderLayers, prelude::*};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (update_tree, update_branches))
        .add_observer(select_entity);
}

#[derive(Component)]
pub struct SceneTreeView;

#[derive(Component)]
pub struct SceneTreeBranch;

pub fn view() -> impl Bundle {
    (
        SceneTreeView,
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
    )
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
                    parent.spawn((
                        Name::new(name),
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
                                ImageNode::new(
                                    asset_server
                                        .load("embedded://planetes_editor/assets/file_icon.png")
                                ),
                                Node {
                                    height: Val::Px(10.0),
                                    ..default()
                                }
                            ),
                            (
                                Text::new(text),
                                TextLayout::new_with_linebreak(LineBreak::WordBoundary),
                                RenderLayers::layer(1),
                            )
                        ],
                    ));
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
    (
        SceneTreeBranch,
        Node {
            padding: px(2.0).left(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(8.0),
            width: percent(100.0),
            ..default()
        },
        RenderLayers::layer(1),
        Represents(target_entity),
        children![(
            Text::new(format!("Child {target_entity}")),
            TextLayout::new_with_linebreak(LineBreak::WordBoundary),
            RenderLayers::layer(1),
        )],
    )
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
