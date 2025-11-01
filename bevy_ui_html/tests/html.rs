use bevy_ecs::{
    hierarchy::ChildOf,
    spawn::{Spawn, SpawnRelatedBundle},
};
use bevy_ui::{Node, px, widget::Text};
use bevy_ui_html::html;

#[test]
fn test_basic_div() {
    let input = html! {
        <div padding="10px">
            "Hello World"
        </div>
    };

    assert!(matches!(input.0, Node { .. }));
    let Node { padding, .. } = input.0;
    assert_eq!(padding, px(10.0).all());
    assert!(matches!(
        input.1,
        SpawnRelatedBundle::<ChildOf, Spawn<Text>> { .. }
    ))
}

#[test]
fn test_basic_span() {
    let input = html! {
        <span>
            "Hello World"
        </span>
    };

    assert_eq!(input, Text("Hello World".to_string()));
}
