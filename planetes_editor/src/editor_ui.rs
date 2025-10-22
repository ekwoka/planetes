use bevy::{
    app::{HierarchyPropagatePlugin, Propagate, PropagateStop},
    asset::embedded_asset,
    camera::{Viewport, visibility::RenderLayers},
    math::Affine2,
    prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{
    EditorMode, ReflectPlanetesBundle, ReflectPlanetesComponent,
    infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    nodes::*,
};

#[cfg(feature = "avian")]
use avian3d::schedule::{Physics, PhysicsTime};

pub fn plugin(app: &mut App) {
    embedded_asset!(app, "assets/directory_icon.png");
    embedded_asset!(app, "assets/file_icon.png");
    embedded_asset!(app, "assets/empty_triangle.png");
    embedded_asset!(app, "assets/filled_triangle.png");

    app.add_plugins((
        InfiniteGridPlugin,
        crate::scene::plugin,
        scene_tree::plugin,
        accordion::plugin,
        entity_viewer::plugin,
        HierarchyPropagatePlugin::<TextFont>::new(Update),
        HierarchyPropagatePlugin::<TextColor>::new(Update),
    ))
    .init_state::<EditorMode>()
    .register_type_data::<Transform, ReflectPlanetesComponent>()
    .register_type_data::<Children, ReflectPlanetesComponent>()
    .add_systems(
        OnEnter(EditorMode::Edit),
        (setup_camera_system, build_ui).chain(),
    )
    .add_systems(
        Update,
        (update_viewport).chain().run_if(in_state(EditorMode::Edit)),
    )
    .add_observer(hover_menu_item)
    .add_observer(unhover_menu_item);
    #[cfg(feature = "avian")]
    {
        app.add_systems(OnEnter(EditorMode::Edit), pause_physics);
        app.add_systems(OnExit(EditorMode::Edit), resume_physics);
    }
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
        Propagate(TextFont {
            font_size: 12.0,
            ..default()
        }),
        Propagate(TextColor::from(Color::linear_rgb(0.7, 0.7, 0.7))),
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
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            width: percent(50.0),
                            height: percent(100.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: px(1.0).all(),
                            ..default()
                        },
                        BorderColor::all(Color::linear_rgb(0.7, 0.7, 0.7)),
                        RenderLayers::layer(1),
                        children![Text::new("Viewport")]
                    ),
                    (
                        Node {
                            padding: px(1.0).all(),
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            flex_grow: 0.0,
                            width: percent(20.0),
                            border: px(1.0).all(),
                            ..default()
                        },
                        BorderColor::all(Color::linear_rgb(0.7, 0.7, 0.7)),
                        RenderLayers::layer(1),
                        children![scene_tree::view(), entity_viewer::view()]
                    ),
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
        children![(Text::new(test))],
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
        PropagateStop::<TextFont>::default(),
        RenderLayers::layer(1),
        children![(
            Text::new(format!(
                "{} v{}",
                env!("CARGO_PKG_NAME").capitalize_words(),
                env!("CARGO_PKG_VERSION")
            )),
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

pub trait Capitalize {
    fn capitalize(&self) -> String;
    fn capitalize_words(&self) -> String;
}

impl Capitalize for String {
    fn capitalize(&self) -> String {
        let mut chars = self.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
    fn capitalize_words(&self) -> String {
        self.split_whitespace()
            .map(|word| word.capitalize())
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl Capitalize for &str {
    fn capitalize(&self) -> String {
        let mut chars = self.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
    fn capitalize_words(&self) -> String {
        self.split_whitespace()
            .map(|word| word.capitalize())
            .collect::<Vec<String>>()
            .join(" ")
    }
}
