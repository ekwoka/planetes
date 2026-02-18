use bevy::prelude::*;
use planetes_scene_state::ApplyEdit;
use std::any::TypeId;

pub fn handle_add_component(event: On<AddComponentToEntity>, mut commands: Commands) {
    info!(
        "Adding component: {:?} to {:?}",
        event.component, event.entity
    );
    commands.write_message(ApplyEdit::AddComponent {
        entity: event.entity,
        component_type: event.component,
    });
}

#[derive(Event)]
pub struct AddComponentToEntity {
    pub entity: Entity,
    pub component: TypeId,
}

#[derive(Event)]
pub struct RemoveComponentFromEntity {
    pub entity: Entity,
    pub component: TypeId,
}

pub fn handle_remove_component(event: On<RemoveComponentFromEntity>, mut commands: Commands) {
    info!(
        "Removing component: {:?} from {:?}",
        event.component, event.entity
    );
    commands.write_message(ApplyEdit::RemoveComponent {
        entity: event.entity,
        component_type: event.component,
    });
}
