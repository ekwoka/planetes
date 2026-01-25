//! Editor UI
//!
//! Handles all the UI and functionality for the Editor

use bevy::{
    app::HierarchyPropagatePlugin,
    asset::embedded_asset,
    camera::{ImageRenderTarget, RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::TextureFormat,
};
use bevy_feathers::{
    FeathersPlugin, FeathersPlugins, dark_theme::create_dark_theme, theme::UiTheme,
};

use crate::{
    EditorMode, ReflectPlanetesComponent,
    atoms::*,
    infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    nodes::{scene_tree::UpdateSceneTree, *},
    prelude::*,
    scene::load_scene,
};

#[cfg(feature = "avian")]
use avian3d::schedule::{Physics, PhysicsTime};

/// Plugin for adding all Editor Functionality and UI
pub fn plugin(app: &mut App) {
    embedded_asset!(app, "assets/directory_icon.png");
    embedded_asset!(app, "assets/file_icon.png");
    embedded_asset!(app, "assets/empty_triangle.png");
    embedded_asset!(app, "assets/filled_triangle.png");

    app.add_plugins((
        InfiniteGridPlugin,
        FeathersPlugins,
        crate::canonical::plugin,
        crate::scene::plugin,
        scene_tree::plugin,
        accordion::plugin,
        entity_viewer::plugin,
        HierarchyPropagatePlugin::<TextFont>::new(Update),
        HierarchyPropagatePlugin::<TextColor>::new(Update),
        planetes_input::plugin,
        component_selector::plugin,
    ))
    .insert_resource(UiTheme(create_dark_theme()))
    .init_state::<EditorMode>()
    .register_type_data::<Transform, ReflectPlanetesComponent>()
    .register_type_data::<Transform, component_editor::ReflectEditorView>()
    .register_type_data::<Children, ReflectPlanetesComponent>()
    .add_systems(
        OnEnter(EditorMode::Edit),
        (setup_camera_system, build_ui).chain().before(load_scene),
    )
    .add_systems(Update, update_viewport)
    .add_observer(button::hover_menu_item)
    .add_observer(button::unhover_menu_item)
    .add_message::<UpdateSceneTree>();
    #[cfg(feature = "avian")]
    {
        app.add_systems(OnEnter(EditorMode::Edit), pause_physics);
        app.add_systems(OnExit(EditorMode::Edit), resume_physics);
    }
}

/// Pauses [avian3d] Physics when in Edit Mode
#[cfg(feature = "avian")]
fn pause_physics(mut time: ResMut<Time<Physics>>) {
    info!("Physics Paused");
    time.pause();
}

/// Resumes [avian3d] Physics when in View Mode
#[cfg(feature = "avian")]
fn resume_physics(mut time: ResMut<Time<Physics>>) {
    info!("Physics Resumed");
    time.unpause();
}

/// Indicates a camera is the primary view.
///
/// Add to your own primary camera
#[derive(Component)]
pub struct MainView;

/// Indicates the Camera that the Editor UI is rendered with.
#[derive(Component)]
pub struct UiView;

/// Indicates the UI Node that the [MainView] is rendered to.
#[derive(Component)]
pub struct ViewPort;

/// Marker component for the MenuBar UI
#[derive(Component)]
pub struct MenuBar;

/// Builds entire Editor UI
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
            text-color="srgb(178 178 178)">
                <MenuBar
                    padding="4px"
                    flex-grow="0"
                    display="flex"
                    flex-direction="row"
                    column-gap="8px"
                    width="100%"
                    border-bottom="1px"
                    border-color="srgb(178 178 178)">
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
                        justify-content="center"
                        align-items={AlignItems::Center}
                        border="1px"
                        border-color="srgb(178 178 178)"
                        components={ViewPort}>
                    </div>
                    <div
                        padding="1px"
                        display="flex"
                        flex-direction="col"
                        flex-grow="0"
                        flex-shrink="0"
                        width="40%"
                        border="1px"
                        border-color="srgb(178 178 178)">
                        {scene_tree::view()}
                        {entity_viewer::view()}
                    </div>
                </div>
                {bottom_bar()}
            </div>
    });
}

/// Sets up the camera system for the editor UI.
pub fn setup_camera_system(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        UiView,
        RenderLayers::layer(1),
        Camera {
            order: 1,
            is_active: true,
            ..default()
        },
        IsDefaultUiCamera,
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

/// Sets up the [ViewPort] node as a Bevy UI [ViewportNode]
pub fn update_viewport(
    mut commands: Commands,
    view_target: Single<Entity, With<ViewPort>>,
    camera: Single<Entity, (With<Camera>, With<MainView>, Changed<MainView>)>,
    mut images: ResMut<Assets<Image>>,
) {
    info!("Updating viewport");
    let entity = camera.into_inner();
    let image = images.add(Image::default_target_texture());
    commands
        .entity(entity)
        .insert(RenderTarget::Image(ImageRenderTarget {
            handle: image,
            scale_factor: 1.0,
        }));
    commands
        .entity(*view_target)
        .insert(ViewportNode::new(entity));
}

trait DefaultTargetTexture {
    fn default_target_texture() -> Self;
}

impl DefaultTargetTexture for Image {
    fn default_target_texture() -> Self {
        Self::new_target_texture(1, 1, TextureFormat::Rgba8UnormSrgb, None)
    }
}

/// Renders the Application bottom status bar.
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

/// Allows Capitalizing Strings
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
