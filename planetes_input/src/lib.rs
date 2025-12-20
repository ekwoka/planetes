pub mod input_field;
pub mod validable;

use bevy::{
    input_focus::{InputDispatchPlugin, tab_navigation::TabNavigationPlugin},
    prelude::*,
};

pub fn plugin(app: &mut App) {
    if !app.is_plugin_added::<InputDispatchPlugin>() {
        app.add_plugins(InputDispatchPlugin);
    }
    if !app.is_plugin_added::<TabNavigationPlugin>() {
        app.add_plugins(TabNavigationPlugin);
    }
    app.add_plugins((
        input_field::input_field_plugin::<String>,
        input_field::input_field_plugin::<f32>,
        input_field::editable_text_plugin,
    ));
}

pub mod prelude {
    pub use super::{input_field::*, validable::*, *};
}
