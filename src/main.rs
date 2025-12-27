//! # Planetes Sandbox
//!
//! This is a sandbox binary for playing around with the tools that make up the Planetes Ecosystem.
//!
//! Core logic is not implemented in this binary, but simply pulled together to test.
//!
//! ## Features
//! - `editor` - Load and use the Planetes Editor
//! - `dev` - Enables basic Bevy Dev features

// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

use avian3d::prelude::*;
use bevy::gltf::GltfPlugin;
use bevy::light::DirectionalLightShadowMap;
use bevy::{
    asset::AssetMetaCheck,
    image::{ImageAddressMode, ImageSamplerDescriptor},
    prelude::*,
};
use bevy_enhanced_input::prelude::EnhancedInputPlugin;
use bevy_tnua::prelude::TnuaControllerPlugin;
use bevy_tnua_avian3d::TnuaAvian3dPlugin;

/// Runs the Sandbox
fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

/// Current Core Planetes builder plugin
#[cfg(feature = "dev")]
pub struct AppPlugin;

#[cfg(not(feature = "dev"))]
struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // Add Bevy plugins.
        app.add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    // This causes errors and even panics on web build on itch.
                    // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Window {
                        title: if cfg!(feature = "editor") {
                            "Planetes Editor".to_string()
                        } else {
                            "Planetes".to_string()
                        },
                        fit_canvas_to_parent: true,
                        position: WindowPosition::At(IVec2::new(100, 600)),
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .set(ImagePlugin {
                    default_sampler: default_image_sampler_descriptor(),
                })
                .set(GltfPlugin {
                    use_model_forward_direction: true,
                    ..default()
                }),
            bevy_ui_anchor::AnchorUiPlugin::<UICamera>::new(),
            EnhancedInputPlugin,
            PhysicsPlugins::default(),
            TnuaAvian3dPlugin::new(PhysicsSchedule),
            TnuaControllerPlugin::new(PhysicsSchedule),
        ));
        // Add other plugins.
        #[cfg(feature = "editor")]
        app.add_plugins(planetes_editor::plugin);
        app.insert_resource(DirectionalLightShadowMap { size: 4096 });
        // Spawn the main camera.
        app.add_systems(Startup, spawn_camera);
    }
}

/// Identifies the camera to be used for Anchored UI Elements from [bevy_ui_anchor::AnchorUiPlugin]
#[derive(Component)]
struct UICamera;

/// Spawns the main view camera.
fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Camera"),
        Camera {
            order: 2,
            is_active: true,
            ..Default::default()
        },
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 30.0),
        UICamera,
        #[cfg(feature = "editor")]
        planetes_editor::MainView,
    ));
}

/// Creates the default image sampler to allow textures to be tiled.
pub(crate) fn default_image_sampler_descriptor() -> ImageSamplerDescriptor {
    ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        anisotropy_clamp: 16,
        ..ImageSamplerDescriptor::linear()
    }
}
