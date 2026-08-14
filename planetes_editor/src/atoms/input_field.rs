//! Provides simple rendering of an [InputField] with hover states

use crate::prelude::*;
use bevy::{
    feathers::cursor::EntityCursor, input_focus::InputFocus, prelude::*, window::SystemCursorIcon,
};
use planetes_input::prelude::{EditableText, InputField, Validable};

pub fn input_field<T: Validable>(value: T) -> impl Bundle {
    html_bundle! {
        <div
            border="1px"
            border-radius="4px"
            border-color={Color::srgb_u8(77, 77, 77)}
            padding-top="1px"
            padding-bottom="1px"
            padding-left="3px"
            padding-right="3px"
            components={EntityCursor::System(SystemCursorIcon::Text)}
            >
            <div
                components={InputField::<T>::new(value)}
                min-height="14px"
                min-width="36px"
            />
        </div>
    }
}

pub fn highlight_selected_input(
    mut commands: Commands,
    inputs: Query<(Entity, &ChildOf), With<EditableText>>,
    focused: Res<InputFocus>,
) {
    if !focused.is_changed() {
        return;
    }
    for (input, child_of) in inputs.iter() {
        if let Some(focused_entity) = focused.get()
            && focused_entity == input
        {
            commands
                .entity(child_of.0)
                .insert(BorderColor::from(Color::srgb_u8(178, 178, 178)));
        } else {
            commands
                .entity(child_of.0)
                .insert(BorderColor::from(Color::srgb_u8(77, 77, 77)));
        }
    }
}
