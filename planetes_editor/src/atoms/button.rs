//! Provides simple rendering of a Button with Hover States

use crate::prelude::*;
use bevy::{ecs::system::IntoObserverSystem, prelude::*};

pub fn render<E: EntityEvent, M: 'static, I: IntoObserverSystem<E, M> + Clone + Send + Sync>(
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
