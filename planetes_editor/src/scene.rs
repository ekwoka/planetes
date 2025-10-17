use crate::{EditorMode, PlanetesComponent, ReflectPlanetesComponent};
use bevy::{prelude::*, tasks::IoTaskPool};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Write};

pub fn plugin(app: &mut App) {
    info!("Plugin SCENE");
    app.add_systems(OnEnter(EditorMode::Edit), load_scene)
        .add_systems(OnEnter(EditorMode::Edit), save_scene);
}

#[derive(Component, Reflect, Serialize, Deserialize, Debug)]
#[reflect(Component, PlanetesComponent, Serialize, Deserialize)]
pub struct EditorScene;

pub fn save_scene(
    world: &World,
    scene_root: Single<Entity, With<EditorScene>>,
    children: Query<&Children>,
    registry: Res<AppTypeRegistry>,
) {
    info!("Saving Scene");
    let registry = registry.clone();
    let registry = registry.read();
    let mut filter = SceneFilter::deny_all();
    for type_id in registry
        .iter_with_data::<ReflectPlanetesComponent>()
        .map(|(registration, _)| {
            info!("Name: {}", registration.type_info().type_path());
            registration.type_id()
        })
    {
        filter = filter.allow_by_id(type_id);
    }

    let mut scene = DynamicSceneBuilder::from_world(world).with_component_filter(filter);

    let mut stack = vec![*scene_root];

    info!("Stack: {:?}", stack);

    while let Some(entity) = stack.pop() {
        info!("Checking entity: {:?}", entity);
        scene = scene.extract_entity(entity);
        if let Ok(children) = children.get(entity) {
            for child in children.iter() {
                info!("Child entity: {:?}", child);
                stack.push(child);
            }
        }
    }

    let scene = scene.remove_empty_entities().build();

    let serialized_scene = scene.serialize(&registry).unwrap();
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

pub fn load_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("loading scene");
    let scene_handle = asset_server.load::<DynamicScene>("test.scn.ron");
    commands.spawn((Transform::default(), DynamicSceneRoot(scene_handle)));
}
