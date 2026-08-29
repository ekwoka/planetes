//! Provides simple rendering of a Button with Hover States

use crate::prelude::*;
use bevy::{ecs::system::IntoObserverSystem, prelude::*};

pub fn render<
    E: EntityEvent,
    B: Bundle,
    M: 'static,
    I: IntoObserverSystem<E, B, M> + Clone + Send + Sync,
>(
    text: impl Into<String>,
    handler: I,
) -> impl Scene {
    let text: String = text.into();
    html! {
        <button variant="normal" corners="all" onActivate={handler}>
            <span>{text}</span>
        </button>
    }
}
