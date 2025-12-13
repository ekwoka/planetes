use bevy::{
    app::HierarchyPropagatePlugin,
    asset::embedded_asset,
    camera::{Viewport, visibility::RenderLayers},
    math::Affine2,
    prelude::*,
};

use crate::{
    EditorMode, ReflectPlanetesComponent,
    atoms::*,
    infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    nodes::*,
    prelude::*,
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
        crate::canonical::plugin,
        crate::scene::plugin,
        scene_tree::plugin,
        accordion::plugin,
        entity_viewer::plugin,
        HierarchyPropagatePlugin::<TextFont>::new(Update),
        HierarchyPropagatePlugin::<TextColor>::new(Update),
        planetes_input::plugin,
    ))
    .init_state::<EditorMode>()
    .register_type_data::<Transform, ReflectPlanetesComponent>()
    //.register_type_data::<Transform, ReflectEditorView>()
    .register_type_data::<Children, ReflectPlanetesComponent>()
    .add_systems(
        OnEnter(EditorMode::Edit),
        (setup_camera_system, build_ui).chain(),
    )
    .add_systems(
        Update,
        (update_viewport).chain().run_if(in_state(EditorMode::Edit)),
    )
    .add_observer(button::hover_menu_item)
    .add_observer(button::unhover_menu_item);
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

pub fn build_ui(mut commands: Commands) {
    commands.spawn(html! {
        <div
            padding="1px"
            flex-grow="0"
            display="flex"
            flex-direction="col"
            width="100%"
            height="100%"
            font-size="12"
            text-color={Color::linear_rgb(0.7, 0.7, 0.7)}>
                <MenuBar
                    padding="4px"
                    flex-grow="0"
                    display="flex"
                    flex-direction="row"
                    column-gap="8px"
                    width="100%"
                    border-bottom="1px"
                    border-color={Color::linear_rgb(0.7, 0.7, 0.7)}>
                    <iter>
                        {["File", "Edit", "View", "Help"].into_iter().map(|item| {
                            button::render(item)
                        })}
                    </iter>
                </MenuBar>
                <div
                    display="flex"
                    flex-direction="row"
                    flex-grow="1"
                    flex-shrink="1"
                    width="100%"
                    height="50%">
                    <div
                        flex-grow="1"
                        flex-shrink="1"
                        width="50%"
                        height="100%"
                        justify-content={JustifyContent::Center}
                        align-items={AlignItems::Center}
                        border="1px"
                        border-color={Color::linear_rgb(0.7, 0.7, 0.7)}>
                        "Viewport"
                    </div>
                    <div
                        padding="1px"
                        display="flex"
                        flex-direction="col"
                        flex-grow="0"
                        flex-shrink="0"
                        width="40%"
                        border="1px"
                        border-color={Color::linear_rgb(0.7, 0.7, 0.7)}>
                        {scene_tree::view()}
                        {entity_viewer::view()}
                    </div>
                </div>
                {bottom_bar()}
            </div>
    });
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
    html! {
        <div
            padding="8px"
            padding-top="2px"
            padding-bottom="2px"
            flex-grow="0"
            font-size="10"
            width="100%">
            <span>
                {format!(
                "{} v{}",
                env!("CARGO_PKG_NAME").capitalize_words(),
                env!("CARGO_PKG_VERSION")
                )}
            </span>
        </div>
    }
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
