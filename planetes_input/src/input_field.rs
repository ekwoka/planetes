use crate::validable::{Validable, Validation};
use bevy::{
    input::keyboard::{Key, KeyboardInput},
    input_focus::{FocusedInput, tab_navigation::TabIndex},
    prelude::*,
};

#[derive(Component, Debug, PartialEq, Eq)]
#[require(Node, TabIndex, EditableText::new(""), Validation)]
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
    app.add_systems(
        PreUpdate,
        (on_value_created::<T>, on_value_changed::<T>).chain(),
    );
    app.add_systems(
        PostUpdate,
        (on_value_created::<T>, on_input_text_changed::<T>).chain(),
    );
}

fn on_value_changed<T: Validable>(
    mut changed_inputs: Query<(&mut InputField<T>, &mut EditableText), Changed<InputField<T>>>,
) {
    for (mut input, mut text) in changed_inputs.iter_mut() {
        eprintln!(
            "Input field value changed from: {} to: {}",
            input.old_value.to_string(),
            input.value.to_string()
        );
        if input.value != input.old_value {
            input.old_value = input.value.clone();
            text.0 = input.value.to_string();
        }
    }
}

fn on_input_text_changed<T: Validable>(
    mut changed_inputs: Query<
        (Entity, &mut InputField<T>, &mut EditableText),
        Changed<EditableText>,
    >,
    mut commands: Commands,
) {
    for (entity, mut input, text) in changed_inputs.iter_mut() {
        eprintln!(
            "Input text changed from: {} to: {}",
            input.value.to_string(),
            text.0.clone()
        );
        if input.value.to_string() != text.0 {
            match T::validate(text.0.as_str()) {
                Ok(value) => {
                    input.value = value;
                    commands.entity(entity).insert(Validation::Valid);
                }
                Err(error) => {
                    commands.entity(entity).insert(Validation::Invalid(error));
                }
            }
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
#[macro_use]
mod input_field {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::input::ButtonState;
    use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
    use bevy::input_focus::IsFocused;
    use bevy::{
        input_focus::{InputDispatchPlugin, InputFocus, IsFocusedHelper},
        window::PrimaryWindow,
    };

    macro_rules! key_event {
        ($key: ident, $text: tt) => {
            KeyboardInput {
                key_code: KeyCode::$key,
                logical_key: Key::Character(stringify!($text).into()),
                state: ButtonState::Pressed,
                text: Some(stringify!($text).into()),
                repeat: false,
                window: Entity::PLACEHOLDER,
            }
        };
    }

    macro_rules! type_event {
        ($app: ident, $key: ident, $text: tt) => {
            $app.world_mut().write_message(key_event!($key, $text));
        };
    }

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

    #[test]
    fn typed_text() {
        let mut app = App::new();

        app.add_plugins((
            input_field_plugin::<u32>,
            bevy::input::InputPlugin,
            InputDispatchPlugin,
            editable_text_plugin,
        ));
        app.world_mut().spawn((Window::default(), PrimaryWindow));

        let input = app.world_mut().spawn(InputField::<u32>::new(1)).id();
        app.world_mut().insert_resource(InputFocus(Some(input)));

        app.update();

        app.world_mut()
            .run_system_once(move |helper: IsFocusedHelper| assert!(helper.is_focused(input)))
            .unwrap();
        {
            let mut fields = app.world_mut().query::<(&InputField<u32>, &Text)>();
            let (input_field, text) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "1");
            assert_eq!(input_field.value, 1);
        }

        type_event!(app, Numpad0, 0);

        app.update();

        {
            let mut fields = app.world_mut().query::<(&InputField<u32>, &Text)>();
            let (input_field, text) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "10");
            assert_eq!(input_field.value, 10);
        }

        type_event!(app, Numpad1, 1);

        app.update();

        {
            let mut fields = app
                .world_mut()
                .query::<(&InputField<u32>, &Text, &Validation)>();
            let (input_field, text, validation) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "101");
            assert_eq!(input_field.value, 101);
            assert!(matches!(validation, &Validation::Valid));
        }

        type_event!(app, KeyH, H);

        app.update();

        {
            let mut fields = app
                .world_mut()
                .query::<(&InputField<u32>, &Text, &Validation)>();
            let (input_field, text, validation) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "101H");
            assert_eq!(input_field.value, 101);
            assert!(matches!(validation, &Validation::Invalid(_)));
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

pub fn editable_text_plugin(app: &mut App) {
    app.add_systems(Update, on_text_change);
    app.add_observer(on_input);
}

pub fn on_input(event: On<FocusedInput<KeyboardInput>>, mut text: Query<&mut EditableText>) {
    eprintln!("Keyboard Input");
    if let Ok(mut editable_text) = text.get_mut(event.focused_entity) {
        eprintln!("Editable text input: {}", editable_text.0);
        if let Key::Character(c) = &event.input.logical_key {
            eprintln!("Found Key: {}", c);
            editable_text.0.push_str(c.as_str());
        }
    }
}

pub fn on_text_change(mut changed_texts: Query<(&EditableText, &mut Text), Changed<EditableText>>) {
    for (editable, mut text) in changed_texts.iter_mut() {
        eprintln!("Editable text changed: {}", editable.0);
        text.0 = editable.0.clone();
    }
}

#[macro_use]
#[cfg(test)]
mod editable_text {
    use bevy::{
        input_focus::{InputDispatchPlugin, InputFocus, IsFocusedHelper},
        window::PrimaryWindow,
    };

    use super::*;

    macro_rules! key_event {
        ($key: ident, $text: ident) => {
            KeyboardInput {
                key_code: KeyCode::$key,
                logical_key: Key::Character(stringify!($text).into()),
                state: ButtonState::Pressed,
                text: Some(stringify!($text).into()),
                repeat: false,
                window: Entity::PLACEHOLDER,
            }
        };
    }

    macro_rules! type_event {
        ($app: ident, $key: ident, $text: ident) => {
            $app.world_mut().write_message(key_event!($key, $text));
        };
    }

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

    #[test]
    fn captures_input() {
        use bevy::ecs::system::RunSystemOnce;
        use bevy::input::ButtonState;
        use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
        use bevy::input_focus::IsFocused;

        let mut app = App::new();

        let mut texts = app.world_mut().query::<&Text>();
        app.add_plugins((
            bevy::input::InputPlugin,
            InputDispatchPlugin,
            editable_text_plugin,
        ));

        app.world_mut().spawn((Window::default(), PrimaryWindow));

        let input = app.world_mut().spawn(EditableText::new("H")).id();
        app.update();

        let text = texts.get(app.world(), input).map(|text| text.0.clone());
        assert_eq!(text, Ok("H".into()));

        app.world_mut().insert_resource(InputFocus(Some(input)));

        app.update();

        app.world_mut()
            .run_system_once(move |helper: IsFocusedHelper| assert!(helper.is_focused(input)))
            .unwrap();

        type_event!(app, KeyI, I);
        type_event!(app, KeyM, M);
        type_event!(app, KeyO, O);
        type_event!(app, KeyM, M);

        app.update();

        let text = texts.get(app.world(), input).map(|text| text.0.clone());
        assert_eq!(text, Ok("HIMOM".into()));
    }
}
