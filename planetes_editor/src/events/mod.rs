use bevy::prelude::*;
use planetes_scene_state::CanonicalScene;
use std::any::TypeId;

use crate::nodes::entity_viewer::UpdateEntityViewer;

pub fn handle_add_component(
    event: On<AddComponentToEntity>,
    mut commands: Commands,
    registry: Res<AppTypeRegistry>,
    canonical_scene: Res<CanonicalScene>,
    mut scenes: ResMut<Assets<DynamicScene>>,
) {
    info!(
        "Adding component: {:?} to {:?}",
        event.component, event.entity
    );
    let registry = registry.read();
    info!("Using Default Component");
    let component_default = registry
        .get_type_data::<ReflectDefault>(event.component)
        .unwrap();
    if let Some(entity) = canonical_scene.get_entity_mut(&mut scenes, event.entity) {
        entity.components.push(component_default.default());
    }
    commands.trigger(UpdateEntityViewer(event.entity));
}

#[derive(Event)]
pub struct AddComponentToEntity {
    pub entity: Entity,
    pub component: TypeId,
}
