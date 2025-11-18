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
}

mod prelude {
    pub use super::*;
    
    
}
