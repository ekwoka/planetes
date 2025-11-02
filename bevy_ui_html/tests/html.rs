use bevy::{
    ecs::spawn::{Spawn, SpawnRelatedBundle},
    prelude::*,
};
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

#[test]
fn test_simple_iter() {
    let mut app = App::new();

    let mut children = app.world_mut().query::<&Children>();
    let mut text = app.world_mut().query::<&Text>();

    let root = app
        .world_mut()
        .spawn(html! {
            <div>
                <iter>
                    {
                        (1..3).map(|i| Text::new(format!("Item {}", i)))
                    }
                </iter>
            </div>
        })
        .id();

    let children = children.get(app.world(), root).unwrap();
    assert_eq!(children.len(), 2);

    assert_eq!(
        text.iter_many(app.world(), children)
            .collect::<Vec<&Text>>(),
        vec![&Text::new("Item 1"), &Text::new("Item 2")]
    );
}
