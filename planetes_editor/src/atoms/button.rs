//! Provides simple rendering of a Button with Hover States

use bevy::{ecs::system::IntoObserverSystem, prelude::*};
use bevy_ui_html::html;

pub fn render<E: Event, B: Bundle, M, I: IntoObserverSystem<E, B, M> + Sync>(
    text: impl Into<String>,
    handler: I,
) -> impl Bundle {
    html! {
        <button variant="normal" corners="all" onActivate={handler}>
            <span>{text}</span>
        </button>
    }
}
