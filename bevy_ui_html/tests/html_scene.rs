use bevy::{prelude::*, scene::ScenePlugin};
use bevy_ui_html::html;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), ScenePlugin));
    app
}

#[test]
fn test_editor_menu_button() {
    #[derive(Component, Clone, Default)]
    struct MenuButton;

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
