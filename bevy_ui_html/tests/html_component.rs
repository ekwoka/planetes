use bevy::prelude::*;
use bevy_ui_html::{HtmlAttributes, HtmlBundle, HtmlComponent, html};

// ── Marker derive ──────────────────────────────────────────────────────────

// ── Tests ──────────────────────────────────────────────────────────────────

#[test]
fn supports_unit_struct() {
    #[derive(Component, HtmlComponent)]
    struct UnitStructComponent;

    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! { <UnitStructComponent padding="8px" /> })
        .id();
    let node = app
        .world_mut()
        .query_filtered::<&Node, With<UnitStructComponent>>()
        .get(app.world(), root)
        .unwrap();
    assert_eq!(
        *node,
        Node {
            padding: px(8.0).all(),
            ..default()
        }
    );
}

#[test]
fn supports_enum() {
    #[derive(Component, HtmlComponent, PartialEq, Eq, Debug)]
    enum Enum {
        VariantOne,
    }

    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! { <Enum::VariantOne padding="8px" /> })
        .id();
    let (node, variant) = app
        .world_mut()
        .query::<(&Node, &Enum)>()
        .get(app.world(), root)
        .unwrap();
    assert_eq!(*variant, Enum::VariantOne);
    assert_eq!(
        *node,
        Node {
            padding: px(8.0).all(),
            ..default()
        }
    );
}

/// A closure with the build signature works as a custom tag.
#[test]
fn support_closure() {
    #[derive(Component)]
    struct MyMarker;

    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! {
            <{|props: HtmlBundle, _attrs: &[_]| (MyMarker, props)} padding="8px" />
        })
        .id();
    let node = app
        .world_mut()
        .query_filtered::<&Node, With<MyMarker>>()
        .get(app.world(), root)
        .unwrap();
    assert_eq!(
        *node,
        Node {
            padding: px(8.0).all(),
            ..default()
        }
    );
}

/// A named fn with `-> impl Bundle` works as a custom tag without block braces.
#[test]
fn supports_fn() {
    #[derive(Component)]
    struct MyMarker;

    fn make_marker(props: HtmlBundle, _: HtmlAttributes) -> impl Bundle {
        (MyMarker, props)
    }

    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! { <make_marker padding="6px" /> })
        .id();
    let node = app
        .world_mut()
        .query_filtered::<&Node, With<MyMarker>>()
        .get(app.world(), root)
        .unwrap();
    assert_eq!(
        *node,
        Node {
            padding: px(6.0).all(),
            ..default()
        }
    );
}

/// A named fn with `-> impl Bundle` works as a custom tag without block braces.
#[test]
fn supports_struct() {
    #[derive(Component, HtmlComponent)]
    struct StructWithData {
        no: u8,
    }

    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! { <{StructWithData { no: 1 }} padding="6px" /> })
        .id();
    let (node, data) = app
        .world_mut()
        .query::<(&Node, &StructWithData)>()
        .get(app.world(), root)
        .unwrap();
    assert_eq!(data.no, 1);
    assert_eq!(
        *node,
        Node {
            padding: px(6.0).all(),
            ..default()
        }
    );
}

/// A named fn with `-> impl Bundle` works as a custom tag without block braces.
#[test]
fn supports_struct_instance() {
    #[derive(Component, HtmlComponent)]
    struct StructWithData {
        no: u8,
    }

    let data = StructWithData { no: 1 };

    let mut app = App::new();
    let root = app.world_mut().spawn(html! { <data padding="6px" /> }).id();
    let (node, data) = app
        .world_mut()
        .query::<(&Node, &StructWithData)>()
        .get(app.world(), root)
        .unwrap();
    assert_eq!(data.no, 1);
    assert_eq!(
        *node,
        Node {
            padding: px(6.0).all(),
            ..default()
        }
    );
}

/// A custom HtmlComponent impl controls the returned bundle and receives the
/// parsed Node.
#[test]
fn custom_impl_spawns_with_correct_node() {
    // ── Manual HtmlComponent impls ─────────────────────────────────────────────
    #[derive(Component)]
    struct MyMarker;

    #[derive(Component)]
    struct OtherMarker;

    impl HtmlComponent for MyMarker {
        fn build(self, props: HtmlBundle, _: HtmlAttributes) -> impl Bundle {
            (OtherMarker, props)
        }
    }
    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! { <MyMarker padding="8px" /> })
        .id();
    let node = app
        .world_mut()
        .query_filtered::<&Node, (With<OtherMarker>, Without<MyMarker>)>()
        .get(app.world(), root)
        .unwrap();
    assert_eq!(
        *node,
        Node {
            padding: px(8.0).all(),
            ..default()
        }
    );
}

/// The HtmlComponent impl can read extra (non-standard) string attributes.
#[test]
fn extra_attrs_accessible_in_build() {
    #[derive(Component, Default)]
    struct ThemedButton {
        variant: &'static str,
    }

    impl HtmlComponent for ThemedButton {
        fn build(self, props: HtmlBundle, additional_attributes: HtmlAttributes) -> impl Bundle {
            let variant = additional_attributes
                .iter()
                .find(|(k, _)| *k == "variant")
                .map(|(_, v)| *v)
                .unwrap_or("default");
            (ThemedButton { variant }, props)
        }
    }

    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! { <{ThemedButton::default()} variant="primary"/> })
        .id();
    let button = app
        .world_mut()
        .query::<&ThemedButton>()
        .get(app.world(), root)
        .unwrap();
    assert_eq!(button.variant, "primary");
}

/// Extra attrs default to "default" when the key is absent.
#[test]
fn extra_attrs_absent_key_falls_back() {
    #[derive(Component, Default)]
    struct ThemedButton {
        variant: &'static str,
    }

    impl HtmlComponent for ThemedButton {
        fn build(self, props: HtmlBundle, additional_attributes: HtmlAttributes) -> impl Bundle {
            let variant = additional_attributes
                .iter()
                .find(|(k, _)| *k == "variant")
                .map(|(_, v)| *v)
                .unwrap_or("default");
            (ThemedButton { variant }, props)
        }
    }

    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! { <{ThemedButton::default()} /> })
        .id();
    let button = app
        .world_mut()
        .query::<&ThemedButton>()
        .get(app.world(), root)
        .unwrap();
    assert_eq!(button.variant, "default");
}

/// The build method can override the node values the macro parsed from attrs.
#[test]
fn build_can_override_node_values() {
    #[derive(Component)]
    struct OverrideButton;

    impl HtmlComponent for OverrideButton {
        fn build(self, mut props: HtmlBundle, _: HtmlAttributes) -> impl Bundle {
            // Always enforce 16px padding regardless of what the attribute says
            props.node.padding = px(16.0).all();
            (self, props)
        }
    }
    // padding="4px" attribute is parsed but the impl overrides it to 16px
    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! { <OverrideButton padding="4px" /> })
        .id();
    let node = app
        .world_mut()
        .query_filtered::<&Node, With<OverrideButton>>()
        .get(app.world(), root)
        .unwrap();
    assert_eq!(
        *node,
        Node {
            padding: px(16.0).all(),
            ..default()
        }
    );
}

/// Standard attribute components (BackgroundColor, etc.) are passed inside
/// HtmlBundle and end up on the entity via the build() return value.
#[test]
fn standard_attrs_still_spawn_as_extra_components() {
    #[derive(Component, HtmlComponent)]
    struct MyMarker;

    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! { <MyMarker padding="4px" background-color="black" /> })
        .id();

    let result = app
        .world_mut()
        .query_filtered::<(&Node, &BackgroundColor), With<MyMarker>>()
        .get(app.world(), root);

    assert!(result.is_ok(), "entity must have Node and BackgroundColor");
    let (node, bg) = result.unwrap();
    assert_eq!(node.padding, px(4.0).all());
    assert_eq!(bg.0, Color::BLACK);
}

/// Children are still spawned as entity children even when HtmlComponent is
/// used.
#[test]
fn children_still_spawn_correctly() {
    #[derive(Component, HtmlComponent)]
    struct MyMarker;

    let mut app = App::new();
    let mut text_query = app.world_mut().query::<&Text>();

    let root = app
        .world_mut()
        .spawn(html! {
            <MyMarker padding="4px">
                "Hello"
            </MyMarker>
        })
        .id();

    let children = app
        .world()
        .get::<Children>(root)
        .expect("root must have children");
    assert_eq!(children.len(), 1);

    let texts: Vec<&Text> = text_query.iter_many(app.world(), children).collect();
    assert_eq!(texts, vec![&Text::new("Hello")]);
}
