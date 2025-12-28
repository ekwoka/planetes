//! # Planetes Input
//!
//! Provides basic functionality for input fields and validation.

pub mod input_field;
pub mod validable;

use bevy::{
    input_focus::{InputDispatchPlugin, tab_navigation::TabNavigationPlugin},
    prelude::*,
};

use input_field::input_field_plugin;

/// Sets up plugins necessary for input handling.
pub fn plugin(app: &mut App) {
    if !app.is_plugin_added::<InputDispatchPlugin>() {
        app.add_plugins(InputDispatchPlugin);
    }
    if !app.is_plugin_added::<TabNavigationPlugin>() {
        app.add_plugins(TabNavigationPlugin);
    }
    app.add_plugins((
        input_field_plugin::<String>,
        (
            input_field_plugin::<u8>,
            input_field_plugin::<u16>,
            input_field_plugin::<u32>,
            input_field_plugin::<u64>,
            input_field_plugin::<u128>,
            input_field_plugin::<usize>,
        ),
        (
            input_field_plugin::<i8>,
            input_field_plugin::<i16>,
            input_field_plugin::<i32>,
            input_field_plugin::<i64>,
            input_field_plugin::<i128>,
            input_field_plugin::<isize>,
        ),
        (input_field_plugin::<f32>, input_field_plugin::<f64>),
        input_field::editable_text_plugin,
    ));
}

pub mod prelude {
    pub use super::{input_field::*, validable::*, *};
}
