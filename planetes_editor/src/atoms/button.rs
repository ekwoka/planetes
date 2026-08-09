//! Provides simple rendering of a Button with Hover States

use crate::prelude::*;
use bevy::{ecs::system::IntoObserverSystem, prelude::*};

pub fn render<E: Event, B: Bundle, M, I: IntoObserverSystem<E, B, M> + Sync>(
    text: impl Into<String>,
    handler: I,
) -> impl Bundle {
    html_bundle! {
        <button variant="normal" corners="all" onActivate={handler}>
            <span>{text}</span>
        </button>
    }
}
