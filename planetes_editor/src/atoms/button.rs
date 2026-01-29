//! Provides simple rendering of a Button with Hover States

use bevy::{
    ecs::{relationship::RelatedSpawner, system::IntoObserverSystem},
    prelude::*,
};
use bevy_feathers::{
    controls::{ButtonProps, ButtonVariant},
    rounded_corners::RoundedCorners,
};

#[derive(Component)]
pub struct MenuButton;

pub fn render<E: Event, B: Bundle, M, I: IntoObserverSystem<E, B, M> + Sync>(
    text: impl Into<String>,
    handler: I,
) -> impl Bundle {
    bevy::feathers::controls::button(
        ButtonProps {
            variant: ButtonVariant::Normal,
            corners: RoundedCorners::All,
        },
        MenuButton,
        (
            Spawn(bevy_ui_html::html! {
               <span>{text}</span>
            }),
            SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
                let entity = parent.target_entity();
                parent.spawn(Observer::new(handler).with_entity(entity));
            }),
        ),
    )
}
