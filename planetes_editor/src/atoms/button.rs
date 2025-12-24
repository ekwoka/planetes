use bevy::prelude::*;

#[derive(Component)]
pub struct MenuButton;

pub fn render(text: impl Into<String>) -> impl Bundle {
    bevy_ui_html::html! {
        <MenuButton
          padding="4px"
          border-radius="2px">
              <span>{text}</span>
        </MenuButton>
    }
}

pub fn hover_menu_item(
    trigger: On<Pointer<Over>>,
    mut menu_items: Query<&mut BackgroundColor, With<MenuButton>>,
) {
    if let Ok(mut color) = menu_items.get_mut(trigger.entity) {
        *color = BackgroundColor::from(Color::srgba_u8(100, 100, 255, 127));
    }
}

pub fn unhover_menu_item(
    trigger: On<Pointer<Out>>,
    mut menu_items: Query<&mut BackgroundColor, With<MenuButton>>,
) {
    if let Ok(mut color) = menu_items.get_mut(trigger.entity) {
        *color = BackgroundColor::DEFAULT;
    }
}
