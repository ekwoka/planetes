use std::{fs::File, io::Write};

use crate::{EditorMode, ReflectPlanetesComponent};
use bevy::{
    app::{HierarchyPropagatePlugin, Propagate},
    prelude::*,
    tasks::IoTaskPool,
};

pub fn plugin(app: &mut App) {
    info!("Plugin SCENE");
    app.add_plugins(HierarchyPropagatePlugin::<InScene>::new(Update))
        .register_type_data::<Name, ReflectPlanetesComponent>()
        .add_systems(OnEnter(EditorMode::Edit), load_scene)
        .add_systems(OnEnter(EditorMode::Edit), save_scene);
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

    if let Ok(serialized_scene) = scene.serialize(&registry) {
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

pub fn load_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("loading scene");
    let scene_handle = asset_server.load::<DynamicScene>("test.scn.ron");
    commands.spawn((
        Name::new("Root"),
        EditorScene,
        Transform::default(),
        Propagate(InScene),
        DynamicSceneRoot(scene_handle),
    ));
}
