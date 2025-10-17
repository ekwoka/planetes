use bevy::{
    camera::{Viewport, visibility::RenderLayers},
    math::Affine2,
    prelude::*,
    tasks::IoTaskPool,
};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Write};

use crate::{
    EditorMode, ReflectPlanetesBundle, ReflectPlanetesComponent,
    infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    nodes::*,
};

#[cfg(feature = "avian")]
use avian3d::schedule::{Physics, PhysicsTime};

pub fn plugin(app: &mut App) {
    app.add_plugins(InfiniteGridPlugin)
        .init_state::<EditorMode>()
        .register_type_data::<Transform, ReflectPlanetesComponent>()
        .register_type_data::<ChildOf, ReflectPlanetesComponent>()
        .add_systems(
            OnEnter(EditorMode::Edit),
            (setup_camera_system, build_ui, build_demo_scene)
                .chain()
                .before(save_scene),
        )
        .add_systems(OnEnter(EditorMode::Edit), save_scene)
        .add_systems(
            Update,
            (update_viewport, scene_tree::update)
                .chain()
                .run_if(in_state(EditorMode::Edit)),
        )
        .add_observer(hover_menu_item)
        .add_observer(unhover_menu_item);
    #[cfg(feature = "avian")]
    {
        app.add_systems(OnEnter(EditorMode::Edit), pause_physics);
        app.add_systems(OnExit(EditorMode::Edit), resume_physics);
    }
}

pub fn build_demo_scene(mut commands: Commands) {
    commands.spawn((
        EditorScene,
        children![
            (Transform::from_xyz(30.0, 0.0, 30.0)),
            (Transform::from_xyz(10.0, 0.0, 10.0)),
            (
                Transform::from_xyz(20.0, 0.0, 20.0),
                children![(Thingy, Transform::default())]
            )
        ],
    ));
}

pub fn save_scene(
    world: &World,
    scene_root: Single<&Children, With<EditorScene>>,
    children: Query<&Children, Without<EditorScene>>,
    registry: Res<AppTypeRegistry>,
) {
    let registry = registry.clone();
    let registry = registry.read();
    let mut filter = SceneFilter::deny_all();
    for type_id in registry
        .iter_with_data::<ReflectPlanetesComponent>()
        .map(|(registration, _)| registration.type_id())
    {
        filter = filter.allow_by_id(type_id);
    }

    let mut scene = DynamicSceneBuilder::from_world(world)
        .with_component_filter(filter)
        .deny_component::<ChildOf>()
        .extract_entities(scene_root.into_iter().copied())
        .allow_component::<ChildOf>();

    let mut stack = scene_root.iter().collect::<Vec<_>>();

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

#[cfg(feature = "avian")]
fn pause_physics(mut time: ResMut<Time<Physics>>) {
    info!("Physics Paused");
    time.pause();
}

#[cfg(feature = "avian")]
fn resume_physics(mut time: ResMut<Time<Physics>>) {
    info!("Physics Resumed");
    time.unpause();
}

#[derive(Component)]
pub struct MainView;

#[derive(Component)]
pub struct UiView;

#[derive(Component)]
pub struct ViewPort;

#[derive(Component)]
pub struct MenuBar;

#[derive(Component)]
pub struct MenuButton;

pub fn hover_menu_item(
    trigger: On<Pointer<Over>>,
    mut menu_items: Query<&mut BackgroundColor, With<MenuButton>>,
) {
    if let Ok(mut color) = menu_items.get_mut(trigger.entity) {
        *color = BackgroundColor::from(Color::linear_rgba(0.2, 0.2, 1.0, 0.50));
    }
}

pub fn unhover_menu_item(
    trigger: On<Pointer<Out>>,
    mut menu_items: Query<&mut BackgroundColor, With<MenuButton>>,
) {
    if let Ok(mut color) = menu_items.get_mut(trigger.entity) {
        *color = BackgroundColor::DEFAULT;
    }
}

pub fn build_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            padding: px(1.0).all(),
            flex_grow: 0.0,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            width: percent(100.0),
            height: percent(100.0),
            // Prevent children from expanding the height of this node.
            min_height: px(0.0),
            ..default()
        },
        RenderLayers::layer(1),
        children![
            (
                Node {
                    padding: px(4.0).all(),
                    flex_grow: 0.0,
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8.0),
                    width: percent(100.0),
                    border: px(1.0).bottom(),
                    ..default()
                },
                MenuBar,
                BorderColor::all(Color::linear_rgb(0.7, 0.7, 0.7)),
                RenderLayers::layer(1),
                children![
                    menu_button("File"),
                    menu_button("Edit"),
                    menu_button("View"),
                    menu_button("Help")
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    width: percent(100.0),
                    height: percent(50.0),
                    ..default()
                },
                children![
                    (
                        Node {
                            padding: px(1.0).all(),
                            flex_grow: 0.0,
                            width: percent(50.0),
                            ..default()
                        },
                        RenderLayers::layer(1),
                        children![scene_tree::view()]
                    ),
                    (
                        Node {
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            width: percent(50.0),
                            height: percent(100.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: px(1.0).left(),
                            ..default()
                        },
                        BorderColor::all(Color::linear_rgb(0.7, 0.7, 0.7)),
                        RenderLayers::layer(1),
                        children![Text::new("Viewport")]
                    )
                ]
            ),
            bottom_bar()
        ],
    ));
}

fn menu_button(test: impl Into<String>) -> impl Bundle {
    (
        Node {
            padding: px(4.0).all(),
            ..default()
        },
        MenuButton,
        BackgroundColor::DEFAULT,
        BorderRadius::all(px(2.0)),
        children![(
            Text::new(test),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor::from(Color::linear_rgb(0.7, 0.7, 0.7))
        )],
    )
}

pub fn setup_camera_system(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        UiView,
        RenderLayers::layer(1),
        Camera {
            order: 1,
            ..default()
        },
    ));

    commands.spawn((
        InfiniteGrid,
        InfiniteGridSettings {
            x_axis_color: Color::WHITE,
            z_axis_color: Color::WHITE,
            major_line_color: Color::WHITE,
            minor_line_color: Color::WHITE,
            ..default()
        },
    ));
}

pub fn update_viewport(
    view_target: Single<(&ComputedNode, &UiGlobalTransform), With<ViewPort>>,
    mut camera: Single<&mut Camera, With<MainView>>,
) {
    let (viewport, transform) = *view_target;
    let size = viewport.size();
    if size.x == 0.0 || size.y == 0.0 {
        return;
    }
    let pos = Affine2::from(transform).translation - size * Vec2::new(0.5, 0.5);
    camera.viewport = Some(Viewport {
        physical_position: pos.as_uvec2(),
        physical_size: UVec2::new(size.x as u32, size.y as u32),
        ..default()
    });
}

fn bottom_bar() -> impl Bundle {
    (
        Node {
            padding: UiRect::axes(px(8.0), px(2.0)),
            flex_grow: 0.0,
            width: percent(100.0),
            ..default()
        },
        RenderLayers::layer(1),
        children![(
            Text::new("Planetes Editor v0.0.1"),
            TextFont {
                font_size: 10.0,
                ..default()
            },
            RenderLayers::layer(1)
        )],
    )
}

#[derive(Component, Reflect, Serialize, Deserialize, Debug)]
#[reflect(Component, PlanetesComponent, Serialize, Deserialize)]
struct Thingy;

#[derive(Bundle, Reflect)]
#[reflect(PlanetesBundle)]
struct ThingyBundle {
    thingy: Thingy,
    transform: Transform,
    camera: Camera3d,
}

#[derive(Component)]
pub struct EditorScene;
