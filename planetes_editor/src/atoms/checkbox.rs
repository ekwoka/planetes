//! Provides simple rendering of a [Checkbox] with hover states

use bevy::{
    input_focus::{InputFocus, tab_navigation::TabIndex},
    prelude::*,
};
use bevy_ui_html::html;
use planetes_input::prelude::InputValue;

/// Represents a checkbox component and current state.
#[derive(Component, Debug, PartialEq, Eq)]
#[require(Node, TabIndex)]
pub struct Checkbox(pub bool);

/// Renders a checkbox
pub fn check_box(value: bool) -> impl Bundle {
    html! {
        <div
            border-radius="4px"
            border="1px"
            border-color="srgb(77 77 77)"
            width="16px"
            height="16px"
            display="flex"
            flex-direction="col"
            justify-content={JustifyContent::Center}
            align-items={AlignItems::Center}
            components={(Checkbox(value), InputValue::new(&value))}
            onClick={on_checkbox_click}>
            <span>{
                if value { "Y" } else { "N" }
            }</span>
        </div>
    }
}

/// Un/Checks checkbox
pub fn on_checkbox_click(event: On<Pointer<Click>>, mut checkboxes: Query<&mut Checkbox>) {
    if let Ok(mut checkbox) = checkboxes.get_mut(event.entity) {
        checkbox.0 = !checkbox.0;
    }
}

/// Updates InputValue from Checkbox state
pub fn on_checkbox_change(
    mut commands: Commands,
    checkboxes: Query<(Entity, &Checkbox, &Children), Changed<Checkbox>>,
    mut texts: Query<&mut Text>,
) {
    for (entity, checkbox, children) in checkboxes.iter() {
        commands.entity(entity).insert(InputValue::new(&checkbox.0));
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = if checkbox.0 { "Y" } else { "N" }.to_string();
            }
        }
    }
}

/// Highlights selected checkbox
pub fn highlight_selected_checkbox(
    mut commands: Commands,
    checkboxes: Query<Entity, With<Checkbox>>,
    focused: Res<InputFocus>,
) {
    if !focused.is_changed() {
        return;
    }
    for checkbox in checkboxes.iter() {
        if let Some(focused_entity) = focused.0
            && focused_entity == checkbox
        {
            commands
                .entity(checkbox)
                .insert(BorderColor::from(Color::srgb_u8(178, 178, 178)));
        } else {
            commands
                .entity(checkbox)
                .insert(BorderColor::from(Color::srgb_u8(77, 77, 77)));
        }
    }
}
