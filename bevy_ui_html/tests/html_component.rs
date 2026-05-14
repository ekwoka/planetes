use bevy::prelude::*;
use bevy_ui_html::html;
use bevy_ui_html_core::{HtmlComponent, HtmlProps};

// ── Manual HtmlComponent impls ─────────────────────────────────────────────

#[derive(Component)]
struct MyMarker;

impl HtmlComponent for MyMarker {
    type Bundle = (MyMarker, Node);
    fn build(props: HtmlProps) -> (MyMarker, Node) {
        (MyMarker, props.node)
    }
}

#[derive(Component)]
struct ThemedButton {
    variant: &'static str,
}

impl HtmlComponent for ThemedButton {
    type Bundle = (ThemedButton, Node);
    fn build(props: HtmlProps) -> (ThemedButton, Node) {
        let variant = props
            .extra_attrs
            .iter()
            .find(|(k, _)| *k == "variant")
            .map(|(_, v)| *v)
            .unwrap_or("default");
        (ThemedButton { variant }, props.node)
    }
}

#[derive(Component)]
struct OverrideButton;

impl HtmlComponent for OverrideButton {
    type Bundle = (OverrideButton, Node);
    fn build(mut props: HtmlProps) -> (OverrideButton, Node) {
        // Always enforce 16px padding regardless of what the attribute says
        props.node.padding = px(16.0).all();
        (OverrideButton, props.node)
    }
}

// ── Marker derive ──────────────────────────────────────────────────────────

#[derive(Component, bevy_ui_html::HtmlComponent)]
struct DerivedMarker;

// ── Tests ──────────────────────────────────────────────────────────────────

/// A custom HtmlComponent impl controls the returned bundle and receives the
/// parsed Node.
#[test]
fn custom_impl_spawns_with_correct_node() {
    let bundle = html! { <MyMarker padding="8px" /> };
    assert_eq!(bundle.1, Node { padding: px(8.0).all(), ..default() });
}

/// The HtmlComponent impl can read extra (non-standard) string attributes.
#[test]
fn extra_attrs_accessible_in_build() {
    let bundle = html! { <ThemedButton variant="primary" /> };
    assert_eq!(bundle.0.variant, "primary");
}

/// Extra attrs default to "default" when the key is absent.
#[test]
fn extra_attrs_absent_key_falls_back() {
    let bundle = html! { <ThemedButton /> };
    assert_eq!(bundle.0.variant, "default");
}

/// The build method can override the node values the macro parsed from attrs.
#[test]
fn build_can_override_node_values() {
    // padding="4px" attribute is parsed but the impl overrides it to 16px
    let bundle = html! { <OverrideButton padding="4px" /> };
    assert_eq!(bundle.1, Node { padding: px(16.0).all(), ..default() });
}

/// Standard attribute components (BackgroundColor, etc.) are still added to
/// the entity tuple alongside the build result.
#[test]
fn standard_attrs_still_spawn_as_extra_components() {
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

/// A type with #[derive(HtmlComponent)] acts as a marker component and passes
/// through the parsed node unmodified.
#[test]
fn derived_html_component_is_marker_with_node() {
    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn(html! { <DerivedMarker padding="6px" /> })
        .id();

    let result = app
        .world_mut()
        .query_filtered::<&Node, With<DerivedMarker>>()
        .get(app.world(), root);

    assert!(result.is_ok(), "entity must have DerivedMarker and Node");
    let node = result.unwrap();
    assert_eq!(node.padding, px(6.0).all());
}
