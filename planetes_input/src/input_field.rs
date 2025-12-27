//! Provides systems and components for managing input fields

use crate::validable::{Validable, Validation};
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusedInput, tab_navigation::TabIndex},
    prelude::*,
};

/// Component that manages the data managed by an input field.
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

/// Adds systems for syncing the InputField data with the EditableText
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

/// Updates the EditableText to use the value that exists in the InputField
fn on_value_changed<T: Validable>(
    mut changed_inputs: Query<(&mut InputField<T>, &mut EditableText), Changed<InputField<T>>>,
) {
    for (mut input, mut text) in changed_inputs.iter_mut() {
        info!(
            "Input field value changed from: {} to: {}",
            input.old_value.to_string(),
            input.value.to_string()
        );
        if input.value != input.old_value {
            input.old_value = input.value.clone();
            text.text = input.value.to_string();
        }
    }
}

/// When the EditableText changes, validates the text and updates the InputField
fn on_input_text_changed<T: Validable>(
    mut changed_inputs: Query<
        (Entity, &mut InputField<T>, &mut EditableText),
        Changed<EditableText>,
    >,
    mut commands: Commands,
) {
    for (entity, mut input, text) in changed_inputs.iter_mut() {
        info!(
            "Input text changed from: {} to: {}",
            input.value.to_string(),
            text.text.clone()
        );
        if input.value.to_string() != text.text {
            match T::validate(text.text.as_str()) {
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

/// Syncs the EditableText accepted_chars with those indicated by the InputField Type
fn on_value_created<T: Validable>(
    mut created_inputs: Query<(&InputField<T>, &mut EditableText), Added<InputField<T>>>,
) {
    for (input, mut text) in created_inputs.iter_mut() {
        info!("Input field created: {}", input.value.to_string());
        text.text = input.value.to_string();
        text.accepted_chars = T::accepted_chars();
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
        assert_eq!(editable_text.text, "10");
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
            input_field_plugin::<String>,
            bevy::input::InputPlugin,
            InputDispatchPlugin,
            editable_text_plugin,
        ));
        app.world_mut().spawn((Window::default(), PrimaryWindow));

        let input = app
            .world_mut()
            .spawn(InputField::<String>::new("1".to_string()))
            .id();
        app.world_mut().insert_resource(InputFocus(Some(input)));

        app.update();

        app.world_mut()
            .run_system_once(move |helper: IsFocusedHelper| assert!(helper.is_focused(input)))
            .unwrap();
        {
            let mut fields = app.world_mut().query::<(&InputField<String>, &Text)>();
            let (input_field, text) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "1");
            assert_eq!(input_field.value, "1");
        }

        type_event!(app, Numpad0, 0);

        app.update();

        {
            let mut fields = app.world_mut().query::<(&InputField<String>, &Text)>();
            let (input_field, text) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "10");
            assert_eq!(input_field.value, "10");
        }

        type_event!(app, Numpad1, 1);

        app.update();

        {
            let mut fields = app
                .world_mut()
                .query::<(&InputField<String>, &Text, &Validation)>();
            let (input_field, text, validation) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "101");
            assert_eq!(input_field.value, "101");
            assert!(matches!(validation, &Validation::Valid));
        }
    }

    #[test]
    fn sets_invalid_when_fails_validation() {
        let mut app = App::new();

        app.add_plugins((
            input_field_plugin::<f32>,
            bevy::input::InputPlugin,
            InputDispatchPlugin,
            editable_text_plugin,
        ));
        app.world_mut().spawn((Window::default(), PrimaryWindow));

        let input = app.world_mut().spawn(InputField::<f32>::new(1.0)).id();
        app.world_mut().insert_resource(InputFocus(Some(input)));

        app.update();

        app.world_mut()
            .run_system_once(move |helper: IsFocusedHelper| assert!(helper.is_focused(input)))
            .unwrap();
        {
            let mut fields = app.world_mut().query::<(&InputField<f32>, &Text)>();
            let (input_field, text) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "1");
            assert_eq!(input_field.value, 1.0);
        }

        type_event!(app, Period, .);

        app.update();

        {
            let mut fields = app.world_mut().query::<(&InputField<f32>, &Text)>();
            let (input_field, text) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "1.");
            assert_eq!(input_field.value, 1.0);
        }

        type_event!(app, Numpad2, 2);

        app.update();

        {
            let mut fields = app
                .world_mut()
                .query::<(&InputField<f32>, &Text, &Validation)>();
            let (input_field, text, validation) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "1.2");
            assert_eq!(input_field.value, 1.2);
            assert_eq!(validation, &Validation::Valid);
        }

        type_event!(app, Period, .);

        app.update();

        {
            let mut fields = app
                .world_mut()
                .query::<(&InputField<f32>, &Text, &Validation)>();
            let (input_field, text, validation) = fields.get(app.world(), input).unwrap();
            assert_eq!(text.0, "1.2.");
            assert_eq!(input_field.value, 1.2);
            assert_eq!(
                validation,
                &Validation::Invalid("Invalid f32 number".into())
            );
        }
    }
}

/// Component that handles the actual text present in an input field.
#[derive(Component, Debug, PartialEq, Eq)]
#[require(Text)]
pub struct EditableText {
    /// Active input Value
    pub text: String,
    /// Characters accepted by the input field
    pub accepted_chars: &'static str,
}

impl EditableText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            accepted_chars: "",
        }
    }

    pub fn accepts(&self, char: &str) -> bool {
        if self.accepted_chars.is_empty() {
            true
        } else {
            self.accepted_chars.contains(char)
        }
    }
}

/// Adds systems and observers to handle input on EditableText
pub fn editable_text_plugin(app: &mut App) {
    app.add_systems(Update, on_text_change);
    app.add_observer(on_input);
}

/// Consumes FocusedInput events on EditableText, applying the input to the text value
pub fn on_input(event: On<FocusedInput<KeyboardInput>>, mut text: Query<&mut EditableText>) {
    if event.input.state == ButtonState::Pressed
        && let Ok(mut editable_text) = text.get_mut(event.focused_entity)
    {
        info!("Editable text input: {}", editable_text.text);

        match &event.input.logical_key {
            Key::Character(c) => {
                if editable_text.accepts(c.as_str()) {
                    editable_text.text.push_str(c.as_str());
                }
            }
            Key::Backspace => {
                editable_text.text.pop();
            }
            Key::Space => {
                editable_text.text.push(' ');
            }
            _ => {}
        }
    }
}

/// Updates the associated Text to use the value tracked by EditableText
pub fn on_text_change(mut changed_texts: Query<(&EditableText, &mut Text), Changed<EditableText>>) {
    for (editable, mut text) in changed_texts.iter_mut() {
        info!("Editable text changed: {}", editable.text);
        text.0 = editable.text.clone();
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

    #[test]
    fn doesnt_capture_input_release() {
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

        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: Key::Character("A".into()),
            state: ButtonState::Released,
            text: Some("A".into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });

        app.update();

        let text = texts.get(app.world(), input).map(|text| text.0.clone());
        assert_eq!(text, Ok("H".into()));
    }

    #[test]
    fn only_allows_accepted_characters_unsigned() {
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
            input_field_plugin::<u32>,
        ));

        app.world_mut().spawn((Window::default(), PrimaryWindow));

        let input = app.world_mut().spawn(InputField::<u32>::new(10)).id();
        app.update();

        let text = texts.get(app.world(), input).map(|text| text.0.clone());
        assert_eq!(text, Ok("10".into()));

        app.world_mut().insert_resource(InputFocus(Some(input)));

        app.update();

        app.world_mut()
            .run_system_once(move |helper: IsFocusedHelper| assert!(helper.is_focused(input)))
            .unwrap();

        type_event!(app, KeyH, H);
        type_event!(app, KeyI, I);

        app.update();

        let text = texts.get(app.world(), input).map(|text| text.0.clone());
        assert_eq!(text, Ok("10".into()));

        type_event!(app, Numpad1, 1);

        app.update();

        let text = texts.get(app.world(), input).map(|text| text.0.clone());
        assert_eq!(text, Ok("101".into()));
    }

    #[test]
    fn handles_space_and_backspace() {
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

        let input = app.world_mut().spawn(EditableText::new("Hi")).id();
        app.update();

        let text = texts.get(app.world(), input).map(|text| text.0.clone());
        assert_eq!(text, Ok("Hi".into()));

        app.world_mut().insert_resource(InputFocus(Some(input)));

        app.update();

        app.world_mut()
            .run_system_once(move |helper: IsFocusedHelper| assert!(helper.is_focused(input)))
            .unwrap();

        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Space,
            logical_key: Key::Space,
            state: ButtonState::Pressed,
            text: Some(" ".into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });

        type_event!(app, KeyM, M);
        type_event!(app, KeyO, O);
        type_event!(app, KeyM, M);

        app.update();

        let text = texts.get(app.world(), input).map(|text| text.0.clone());
        assert_eq!(text, Ok("Hi MOM".into()));

        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Backspace,
            logical_key: Key::Backspace,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });

        app.update();

        let text = texts.get(app.world(), input).map(|text| text.0.clone());
        assert_eq!(text, Ok("Hi MO".into()));
    }
}
