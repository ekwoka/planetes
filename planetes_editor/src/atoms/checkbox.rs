//! Provides simple rendering of a [Checkbox] with hover states

use crate::prelude::*;
use bevy::{
    input_focus::tab_navigation::TabIndex, prelude::*, ui::Checked, ui_widgets::ValueChange,
};
use planetes_input::prelude::InputValue;

/// Represents a checkbox component and current state.
///
/// [Clone] and [Default] are what make this usable as a template in `html!`.
#[derive(Component, Debug, PartialEq, Eq, Clone, Default)]
#[require(Node, TabIndex)]
pub struct Checkbox(pub bool);

/// Renders a checkbox
pub fn check_box(value: bool) -> impl Scene {
    html! {
        <input type="checkbox" components={Checkbox(value)} />
    }
}

pub fn on_checkbox_add(
    event: On<Add, Checkbox>,
    mut commands: Commands,
    checkboxes: Query<(Entity, &Checkbox)>,
) {
    if let Ok((entity, checkbox)) = checkboxes.get(event.entity) {
        commands.entity(entity).insert(InputValue::new(&checkbox.0));
        if checkbox.0 {
            commands.entity(entity).insert(Checked);
        } else {
            commands.entity(entity).try_remove::<Checked>();
        }
    }
}

/// Updates InputValue from Checkbox state change
pub fn on_checkbox_value_change(
    event: On<ValueChange<bool>>,
    mut commands: Commands,
    mut checkboxes: Query<(Entity, &mut Checkbox)>,
) {
    if let Ok((entity, mut checkbox)) = checkboxes.get_mut(event.source) {
        checkbox.0 = event.value;
        commands
            .entity(entity)
            .insert(InputValue::new(&event.value));
        if event.value {
            commands.entity(entity).insert(Checked);
        } else {
            commands.entity(entity).try_remove::<Checked>();
        }
    }
}
