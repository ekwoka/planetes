//! Handling Scene Loading and Saving

use std::{fs::File, io::Write};

use crate::{EditorMode, ReflectPlanetesComponent, nodes::scene_tree::UpdateSceneTree};
use bevy::{
    app::{HierarchyPropagatePlugin, Propagate},
    prelude::*,
    tasks::IoTaskPool,
};
use planetes_scene_state::CanonicalScene;

pub fn plugin(app: &mut App) {
    info!("Plugin SCENE");
    app.add_plugins(HierarchyPropagatePlugin::<InScene>::new(Update))
        .register_type_data::<Name, ReflectPlanetesComponent>()
        .add_systems(OnEnter(EditorMode::Edit), load_scene)
        .add_systems(OnEnter(EditorMode::Edit), save_scene)
        .add_systems(Update, add_meshes_to_scene)
        .add_systems(Update, on_load);
}

#[derive(Component)]
pub struct EditorScene;

#[derive(Component, PartialEq, Eq, Clone, Copy, Debug)]
pub struct InScene;

pub fn save_scene(
    world: &World,
    in_scene: Query<Entity, (With<InScene>, Without<EditorScene>)>,
    registry: Res<AppTypeRegistry>,
) {
    info!("Saving Scene");
    if in_scene.is_empty() {
        info!("No entities to save");
        return;
    }
    let registry = registry.clone();
    let registry = registry.read();
    let filter = SceneFilter::Allowlist(
        registry
            .iter_with_data::<ReflectPlanetesComponent>()
            .map(|(registration, _)| registration.type_id())
            .collect(),
    );

    let scene = DynamicSceneBuilder::from_world(world)
        .with_component_filter(filter)
        .extract_entities(in_scene.iter())
        .remove_empty_entities()
        .build();

    if let Ok(serialized_scene) = scene.serialize(&registry)
        && false
    {
        info!("Saving Scene");
        info!("{}", serialized_scene);
        IoTaskPool::get()
            .spawn(async move {
                // Write the scene RON data to file
                File::create("assets/test.scn.ron")
                    .and_then(|mut file| file.write(serialized_scene.as_bytes()))
                    .expect("Error while writing scene to file");
            })
            .detach();
    }
}

pub fn load_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut canonical: ResMut<CanonicalScene>,
) {
    info!("loading scene");
    let scene_handle = asset_server.load::<DynamicScene>("test.scn.ron");
    let scene_root = commands
        .spawn((
            Name::new("Root"),
            EditorScene,
            Transform::default(),
            Propagate(InScene),
            DynamicSceneRoot(scene_handle.clone()),
        ))
        .id();
    canonical.insert(scene_handle);
    commands.write_message(UpdateSceneTree { entity: scene_root });
}

pub fn on_load(
    mut events: MessageReader<AssetEvent<DynamicScene>>,
    assets: Res<Assets<DynamicScene>>,
) {
    for event in events.read() {
        match event {
            AssetEvent::Added { id } => {
                info!("Scene added {id}");
            }
            AssetEvent::LoadedWithDependencies { id } => {
                info!("Scene Loaded {id}");
                if let Some(scene) = assets.get(*id) {
                    info!("Scene Available");
                    info!("With {} Entities", scene.entities.len());
                    for entity in scene.entities.iter() {
                        info!("   Entity: {}", entity.entity);
                        info!("   With {} Components", entity.components.len());
                    }
                }
            }
            _ => {
                info!("{event:?}");
            }
        }
    }
}

fn add_meshes_to_scene(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    entities: Query<Entity, (With<InScene>, Without<EditorScene>, Without<Mesh3d>)>,
) {
    let mesh = mesh_assets.add(Sphere::new(1.0));
    let material = materials.add(StandardMaterial::default());
    entities.iter().for_each(|entity| {
        info!("Adding Mesh to Entity: {entity}");
        commands
            .entity(entity)
            .try_insert((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
    });
}
