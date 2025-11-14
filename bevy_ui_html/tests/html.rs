use std::time::Duration;

use bevy::{
    camera::NormalizedRenderTarget,
    ecs::{
        relationship::RelatedSpawner,
        spawn::{Spawn, SpawnRelatedBundle},
    },
    picking::pointer::Location,
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

#[test]
fn test_editor_menu_button() {
    #[derive(Component)]
    struct MenuButton;

    let mut app = App::new();

    let mut text = app.world_mut().query::<&Text>();
    let root = app
        .world_mut()
        .spawn(html! {
            <MenuButton
               padding="4px"
               border-radius="2px">
               "Menu"
            </MenuButton>
        })
        .id();
    let root_entity = app
        .world_mut()
        .query_filtered::<(&Node, &BorderRadius, &Children), With<MenuButton>>()
        .get(app.world(), root);
    assert!(root_entity.is_ok());
    let root_entity = root_entity.unwrap();
    assert_eq!(
        root_entity.0,
        &Node {
            padding: px(4.0).all(),
            ..default()
        }
    );
    assert_eq!(root_entity.1, &BorderRadius::all(px(2.0)));
    assert_eq!(root_entity.2.len(), 1);
    assert_eq!(
        text.iter_many(app.world(), root_entity.2)
            .collect::<Vec<&Text>>(),
        vec![&Text::new("Menu")]
    )
}

#[test]
fn test_simple_with() {
    let mut app = App::new();

    let mut children = app.world_mut().query::<&Children>();
    let mut text = app.world_mut().query::<&Text>();

    let root = app
        .world_mut()
        .spawn(html! {
            <div>
                <with>
                    {
                        let thing = true;
                        if thing {
                            parent.spawn(html! { <span>"Hello World"</span> });
                        } else {
                            parent.spawn(html! { <span>"Hello Mom"</span> });
                        }
                    }
                </with>
            </div>
        })
        .id();

    let children = children.get(app.world(), root).unwrap();

    assert_eq!(children.len(), 1);

    assert_eq!(
        text.iter_many(app.world(), children)
            .collect::<Vec<&Text>>(),
        vec![&Text::new("Hello World")]
    );
}

#[test]
fn allows_listeners() {
    let mut app = App::new();

    let mut text = app.world_mut().query::<&Text>();

    let root = app
        .world_mut()
        .spawn(html! {
            <div onClick={|_event: On<Pointer<Click>>,
                mut commands: Commands,
                text: Single<Entity, With<Text>>| {
                    commands.entity(*text).insert(Text::new("Hi, Mom!"));
                }}>
                "Hello, World!"
            </div>
        })
        .id();

    app.update();

    app.world_mut().commands().trigger(Pointer {
        entity: root,
        pointer_id: bevy::picking::backend::prelude::PointerId::Mouse,
        event: Click {
            button: PointerButton::Primary,
            hit: bevy::picking::backend::HitData {
                camera: root,
                depth: 0.0,
                position: None,
                normal: None,
            },
            duration: Duration::from_millis(100),
        },
        pointer_location: Location {
            target: NormalizedRenderTarget::None {
                width: 0,
                height: 0,
            },
            position: Vec2::ZERO,
        },
    });

    app.update();

    assert_eq!(
        text.iter(app.world()).collect::<Vec<&Text>>(),
        vec![&Text::new("Hi, Mom!")]
    );
}

#[test]
fn allows_listeners_no_macro() {
    let mut app = App::new();

    let mut text = app.world_mut().query::<&Text>();

    let root = app
        .world_mut()
        .spawn((
            Node::default(),
            Children::spawn((
                SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
                    let entity = parent.target_entity();
                    parent.spawn(
                        Observer::new(
                            |_event: On<Pointer<Click>>,
                             mut commands: Commands,
                             text: Single<Entity, With<Text>>| {
                                commands.entity(*text).insert(Text::new("Hi, Mom!"));
                            },
                        )
                        .with_entity(entity),
                    );
                }),
                Spawn(Text::new("Hello, World!")),
            )),
        ))
        .id();

    app.update();

    app.world_mut().commands().trigger(Pointer {
        entity: root,
        pointer_id: bevy::picking::backend::prelude::PointerId::Mouse,
        event: Click {
            button: PointerButton::Primary,
            hit: bevy::picking::backend::HitData {
                camera: root,
                depth: 0.0,
                position: None,
                normal: None,
            },
            duration: Duration::from_millis(100),
        },
        pointer_location: Location {
            target: NormalizedRenderTarget::None {
                width: 0,
                height: 0,
            },
            position: Vec2::ZERO,
        },
    });

    app.update();

    assert_eq!(
        text.iter(app.world()).collect::<Vec<&Text>>(),
        vec![&Text::new("Hi, Mom!")]
    );
}
