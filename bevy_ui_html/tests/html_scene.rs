use bevy::{prelude::*, scene::ScenePlugin};
use bevy_ui_html::html;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), ScenePlugin));
    app
}

#[test]
fn test_editor_menu_button() {
    #[derive(SceneComponent, Clone, Default)]
    struct MenuButton;

    impl MenuButton {
        fn scene() -> impl Scene {
            bsn! {}
        }
    }

    let mut app = test_app();

    let mut text = app.world_mut().query::<&Text>();
    let _ = app.world_mut().spawn_scene(html! {
        <MenuButton
           padding="4px">
           "Menu"
        </MenuButton>
    });
    let root_entity = app
        .world_mut()
        .query_filtered::<(&Node, &Children), With<MenuButton>>()
        .single(app.world());
    assert!(root_entity.is_ok());
    let root_entity = root_entity.unwrap();
    assert_eq!(
        root_entity.0,
        &Node {
            padding: px(4.0).all(),
            ..default()
        }
    );
    assert_eq!(root_entity.1.len(), 1);
    assert_eq!(
        text.iter_many(app.world(), root_entity.1)
            .collect::<Vec<&Text>>(),
        vec![&Text::new("Menu")]
    )
}

#[test]
fn test_scene_component() {
    #[derive(SceneComponent, Clone, Default)]
    struct MenuButton;

    impl MenuButton {
        fn scene() -> impl Scene {
            html! {
                <div padding="4px"/>
            }
        }
    }

    let mut app = test_app();

    let mut text = app.world_mut().query::<&Text>();
    let _ = app.world_mut().spawn_scene(html! {
        <MenuButton>
           "Menu"
        </MenuButton>
    });
    let root_entity = app
        .world_mut()
        .query_filtered::<(&Node, &Children), With<MenuButton>>()
        .single(app.world());
    assert!(root_entity.is_ok());
    let root_entity = root_entity.unwrap();
    assert_eq!(
        root_entity.0,
        &Node {
            padding: px(4.0).all(),
            ..default()
        }
    );
    assert_eq!(root_entity.1.len(), 1);
    assert_eq!(
        text.iter_many(app.world(), root_entity.1)
            .collect::<Vec<&Text>>(),
        vec![&Text::new("Menu")]
    )
}

#[test]
fn test_scene_component_props() {
    #[derive(SceneComponent, Clone, Debug, Default, PartialEq)]
    struct MenuButton {
        variant: String,
    }

    impl MenuButton {
        fn scene() -> impl Scene {
            html! {
                <div padding="4px"/>
            }
        }
    }

    let mut app = test_app();

    let mut text = app.world_mut().query::<&Text>();
    let _ = app.world_mut().spawn_scene(html! {
        <MenuButton variant="primary">
           "Menu"
        </MenuButton>
    });
    let root_entity = app
        .world_mut()
        .query::<(&Node, &MenuButton, &Children)>()
        .single(app.world());
    assert!(root_entity.is_ok());
    let root_entity = root_entity.unwrap();
    assert_eq!(
        root_entity.0,
        &Node {
            padding: px(4.0).all(),
            ..default()
        }
    );
    assert_eq!(
        root_entity.1,
        &MenuButton {
            variant: "primary".to_string()
        }
    );
    assert_eq!(root_entity.2.len(), 1);
    assert_eq!(
        text.iter_many(app.world(), root_entity.2)
            .collect::<Vec<&Text>>(),
        vec![&Text::new("Menu")]
    )
}

#[test]
fn test_scene_component_scene_props() {
    #[derive(SceneComponent, Clone, Debug, Default, PartialEq)]
    #[scene(MenuButtonProps)]
    struct MenuButton {
        variant: String,
    }

    #[derive(Default)]
    struct MenuButtonProps {
        spacing: Val,
    }

    impl MenuButton {
        fn scene(props: MenuButtonProps) -> impl Scene {
            html! {
                <div padding={props.spacing} />
            }
        }
    }

    let mut app = test_app();

    let mut text = app.world_mut().query::<&Text>();
    let _ = app.world_mut().spawn_scene(html! {
        <MenuButton variant="primary" @spacing={px(5.0)}>
           "Menu"
        </MenuButton>
    });
    let root_entity = app
        .world_mut()
        .query::<(&Node, &MenuButton, &Children)>()
        .single(app.world());
    assert!(root_entity.is_ok());
    let root_entity = root_entity.unwrap();
    assert_eq!(root_entity.0.padding, px(5.0).all());
    assert_eq!(
        root_entity.1,
        &MenuButton {
            variant: "primary".to_string()
        }
    );
    assert_eq!(root_entity.2.len(), 1);
    assert_eq!(
        text.iter_many(app.world(), root_entity.2)
            .collect::<Vec<&Text>>(),
        vec![&Text::new("Menu")]
    )
}
