use crate::validable::Validable;
use bevy::{input_focus::tab_navigation::TabIndex, prelude::*};

#[derive(Component, Debug, PartialEq, Eq)]
#[require(Node, TabIndex, EditableText::new(""))]
pub struct InputField<T: Validable> {
    pub value: T,
    pub old_value: T,
}

impl<T: Validable> Default for InputField<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Validable> InputField<T> {
    /// Create a new validated input field with the given value
    pub fn new(value: T) -> Self {
        Self {
            value: value.clone(),
            old_value: value,
        }
    }
}

pub fn input_field_plugin<T: Validable>(app: &mut App) {
    app.add_systems(PreUpdate, on_value_changed::<T>);
    app.add_systems(PreUpdate, on_value_created::<T>);
}

fn on_value_changed<T: Validable>(
    mut changed_inputs: Query<(&mut InputField<T>, &mut EditableText), Changed<InputField<T>>>,
) {
    for (mut input, mut text) in changed_inputs.iter_mut() {
        eprintln!("Input field value changed: {}", input.value.to_string());
        if input.value != input.old_value {
            input.old_value = input.value.clone();
            text.0 = input.value.to_string();
        }
    }
}

fn on_value_created<T: Validable>(
    mut created_inputs: Query<(&InputField<T>, &mut EditableText), Added<InputField<T>>>,
) {
    for (input, mut text) in created_inputs.iter_mut() {
        eprintln!("Input field created: {}", input.value.to_string());
        text.0 = input.value.to_string();
    }
}

#[cfg(test)]
mod input_field {
    use super::*;

    #[test]
    fn works() {
        let mut app = App::new();

        let mut nodes = app.world_mut().query::<&Node>();
        app.add_plugins(input_field_plugin::<u32>);

        let input = app.world_mut().spawn(InputField::<u32>::new(10)).id();

        let node = nodes.get(app.world(), input);

        assert_eq!(node, Ok(&Node::default()));
    }

    #[test]
    fn initial_field_value_persisted_to_text() {
        let mut app = App::new();

        app.add_plugins((input_field_plugin::<u32>, editable_text_plugin));

        let input = app.world_mut().spawn(InputField::<u32>::new(10)).id();

        let mut fields = app
            .world_mut()
            .query::<(&InputField<u32>, &EditableText, &Text)>();

        app.update();

        let (input_field, editable_text, text) = fields.get(app.world(), input).unwrap();

        assert_eq!(
            input_field,
            &InputField {
                value: 10,
                old_value: 10
            }
        );
        assert_eq!(editable_text.0, "10");
        assert_eq!(text.0, "10");
    }

    #[test]
    fn update_field_value_persisted_to_text() {
        let mut app = App::new();

        app.add_plugins((input_field_plugin::<u32>, editable_text_plugin));

        let input = app.world_mut().spawn(InputField::<u32>::new(10)).id();

        app.update();

        {
            let mut fields = app.world_mut().query::<(&mut InputField<u32>, &Text)>();
            let (mut input_field, text) = fields.get_mut(app.world_mut(), input).unwrap();
            assert_eq!(text.0, "10");
            input_field.value = 20;
        }

        app.update();

        {
            let mut field_texts = app.world_mut().query::<&Text>();
            let text = field_texts.get(app.world(), input).unwrap();
            assert_eq!(text.0, "20");
        }
    }
}

#[derive(Component, Debug, PartialEq, Eq)]
#[require(Text)]
pub struct EditableText(pub String);

impl EditableText {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

fn editable_text_plugin(app: &mut App) {
    app.add_systems(PostUpdate, on_text_change);
}

pub fn on_text_change(mut changed_texts: Query<(&EditableText, &mut Text), Changed<EditableText>>) {
    for (editable, mut text) in changed_texts.iter_mut() {
        eprintln!("Editable text changed: {}", editable.0);
        text.0 = editable.0.clone();
    }
}

#[cfg(test)]
mod editable_text {
    use super::*;

    #[test]
    fn copies_editable_text_to_text() {
        let mut app = App::new();

        let mut texts = app.world_mut().query::<&Text>();
        app.add_plugins(editable_text_plugin);

        let input = app.world_mut().spawn(EditableText::new("Hello")).id();
        app.update();

        let text = texts.get(app.world(), input).map(|text| text.0.clone());
        assert_eq!(text, Ok("Hello".into()));
    }
}
