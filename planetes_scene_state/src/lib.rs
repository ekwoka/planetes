//! Canonical scene state management for the editor.
//!
//! This module provides the infrastructure for maintaining a "clean" representation
//! of entity component data that is separate from the live scene. This separation
//! allows the live scene to have physics, previews, and other runtime modifications
//! while preserving the canonical state that will be saved.
//!
//! # Architecture
//!
//! - [`CanonicalScene`] - The source of truth for all editable entity data
//! - [`EditOp`] - A single field-level edit operation (diff)
//! - [`EditHistory`] - Undo/redo stacks for edit operations
//! - [`ApplyEditMessage`] - Message to request applying an edit to canonical state

use std::{any::TypeId, collections::HashMap};

use bevy::{
    ecs::system::SystemState,
    prelude::*,
    reflect::{PartialReflect, ReflectPath},
};

#[reflect_trait]
pub trait PlanetesComponent {}

impl<T: Reflect + Component> PlanetesComponent for T {}

#[reflect_trait]
pub trait PlanetesBundle {}

impl<T: Reflect + Bundle> PlanetesBundle for T {}

/// Plugin that registers canonical scene management systems.
pub fn plugin(app: &mut App) {
    app.init_resource::<CanonicalScene>()
        .init_resource::<EditHistory>()
        .add_message::<ApplyEditMessage>()
        .add_message::<UndoMessage>()
        .add_message::<RedoMessage>()
        .add_message::<SyncCanonicalMessage>()
        .add_systems(
            Update,
            (
                sync_canonical_scene,
                apply_edit_messages,
                handle_undo,
                handle_redo,
            )
                .chain(),
        );
}

/// The canonical "saveable" state of all editable entities.
///
/// This resource stores reflected component data for entities that are being
/// edited. The UI reads from this resource rather than from live entities,
/// ensuring that physics simulations and preview systems don't affect the
/// data that will be saved.
///
/// # Example
///
/// ```ignore
/// fn read_canonical_transform(
///     canonical: Res<CanonicalScene>,
///     entity: Entity,
/// ) {
///     if let Some(transform_data) = canonical.get_component::<Transform>(entity) {
///         // Use the canonical transform data
///     }
/// }
/// ```
#[derive(Resource, Default)]
pub struct CanonicalScene {
    /// Maps entities to their canonical component data.
    /// Inner map: TypeId -> Reflected component data
    components: HashMap<Entity, HashMap<TypeId, Box<dyn PartialReflect>>>,
}

impl CanonicalScene {
    /// Returns the canonical data for a specific component by id on an entity.
    pub fn get_component_by_id(
        &self,
        entity: Entity,
        type_id: TypeId,
    ) -> Option<&dyn PartialReflect> {
        self.components
            .get(&entity)
            .and_then(|comps| comps.get(&type_id))
            .map(|boxed| boxed.as_ref())
    }

    pub fn get_component<T: Component + 'static>(
        &self,
        entity: Entity,
    ) -> Option<&dyn PartialReflect> {
        self.get_component_by_id(entity, TypeId::of::<T>())
    }

    /// Returns mutable access to the canonical data for a specific component.
    pub fn get_component_mut_by_id(
        &mut self,
        entity: Entity,
        type_id: TypeId,
    ) -> Option<&mut Box<dyn PartialReflect>> {
        self.components
            .get_mut(&entity)
            .and_then(|comps| comps.get_mut(&type_id))
    }

    pub fn get_component_mut<T: Component + 'static>(
        &mut self,
        entity: Entity,
    ) -> Option<&mut Box<dyn PartialReflect>> {
        self.get_component_mut_by_id(entity, TypeId::of::<T>())
    }

    pub fn insert_entity(
        &mut self,
        entity: Entity,
        data: HashMap<TypeId, Box<dyn PartialReflect>>,
    ) {
        self.components.insert(entity, data);
    }

    /// Inserts or updates canonical data for a component on an entity.
    pub fn insert_component(
        &mut self,
        entity: Entity,
        type_id: TypeId,
        data: Box<dyn PartialReflect>,
    ) {
        self.components
            .entry(entity)
            .or_default()
            .insert(type_id, data);
    }

    /// Removes all canonical data for an entity.
    pub fn remove_entity(&mut self, entity: Entity) {
        self.components.remove(&entity);
    }

    /// Returns all component type IDs stored for an entity.
    pub fn get_entity_components(
        &self,
        entity: Entity,
    ) -> Option<&HashMap<TypeId, Box<dyn PartialReflect>>> {
        self.components.get(&entity)
    }

    /// Checks if an entity has any canonical data stored.
    pub fn contains_entity(&self, entity: Entity) -> bool {
        self.components.contains_key(&entity)
    }
}

/// A single field-level edit operation representing a change to component data.
///
/// Edit operations store both the old and new values, enabling undo/redo
/// functionality. The `field_path` uses Bevy's reflection path syntax
/// (e.g., `"translation.x"` or `".0.color"`).
pub struct EditOp {
    /// The entity being edited.
    pub entity: Entity,
    /// The component type being modified.
    pub component_type: TypeId,
    /// The reflection path to the field being edited.
    /// Uses Bevy's path syntax: `"field_name"`, `".0"` for tuple fields, etc.
    pub field_path: String,
    /// The value before this edit was applied.
    pub old_value: Box<dyn PartialReflect>,
    /// The value after this edit is applied.
    pub new_value: Box<dyn PartialReflect>,
}

impl std::fmt::Debug for EditOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditOp")
            .field("entity", &self.entity)
            .field("component_type", &self.component_type)
            .field("field_path", &self.field_path)
            .finish_non_exhaustive()
    }
}

/// Undo/redo history for edit operations.
///
/// Maintains two stacks:
/// - `undo_stack`: Operations that can be undone (most recent last)
/// - `redo_stack`: Operations that were undone and can be redone
///
/// When a new edit is applied, the redo stack is cleared.
#[derive(Resource, Default)]
pub struct EditHistory {
    /// Stack of operations that can be undone.
    pub undo_stack: Vec<EditOp>,
    /// Stack of operations that can be redone.
    pub redo_stack: Vec<EditOp>,
}

impl EditHistory {
    /// Pushes a new edit operation onto the undo stack and clears the redo stack.
    pub fn push(&mut self, op: EditOp) {
        self.undo_stack.push(op);
        self.redo_stack.clear();
    }

    /// Pops the most recent edit from the undo stack.
    pub fn pop_undo(&mut self) -> Option<EditOp> {
        self.undo_stack.pop()
    }

    /// Pops the most recent edit from the redo stack.
    pub fn pop_redo(&mut self) -> Option<EditOp> {
        self.redo_stack.pop()
    }

    /// Pushes an operation onto the redo stack (used after undo).
    pub fn push_redo(&mut self, op: EditOp) {
        self.redo_stack.push(op);
    }

    /// Pushes an operation onto the undo stack (used after redo).
    pub fn push_undo(&mut self, op: EditOp) {
        self.undo_stack.push(op);
    }

    /// Returns true if there are operations that can be undone.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns true if there are operations that can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clears all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

/// Message to request applying an edit to the canonical scene.
///
/// Send this message when a field value changes in the UI. The system will:
/// 1. Capture the old value from canonical state
/// 2. Apply the new value to canonical state
/// 3. Record the operation in edit history for undo/redo
#[derive(Message)]
pub struct ApplyEditMessage {
    /// The entity being edited.
    pub entity: Entity,
    /// The component type being modified.
    pub component_type: TypeId,
    /// The reflection path to the field being edited.
    pub field_path: String,
    /// The new value to apply.
    pub new_value: Box<dyn PartialReflect>,
}

/// Message to request an undo operation.
#[derive(Message)]
pub struct UndoMessage;

/// Message to request a redo operation.
#[derive(Message)]
pub struct RedoMessage;

/// Message to request syncing canonical state from live entities.
///
/// This is typically sent when an entity is first selected for editing,
/// to populate the canonical scene with the current live state.
#[derive(Message)]
pub struct SyncCanonicalMessage {
    /// The entity to sync.
    pub entity: Entity,
}

/// Exclusive system that syncs canonical state from live entities when requested.
///
/// This is an exclusive system because it needs both world access for entity
/// inspection and mutable access to the CanonicalScene resource.
fn sync_canonical_scene(
    world: &mut World,
    params: &mut SystemState<MessageReader<SyncCanonicalMessage>>,
) {
    let mut messages = params.get_mut(world);
    // Collect entity IDs first
    let mut entities: Vec<Entity> = messages.read().map(|m| m.entity).collect();

    if entities.is_empty() {
        return;
    }

    // Get allowed types from registry
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry_guard = registry.read();
    let allowed_types: Vec<_> = registry_guard
        .iter_with_data::<ReflectPlanetesComponent>()
        .map(|(type_reg, _)| type_reg.type_id())
        .collect();

    // Collect component data for each entity

    while let Some(entity) = entities.pop() {
        let Ok(entity_ref) = world.get_entity(entity) else {
            continue;
        };

        if let Some(children) = entity_ref.get_components::<&Children>() {
            entities.extend(children.iter());
        }

        let Ok(components) = world.inspect_entity(entity) else {
            continue;
        };

        let mut updates: HashMap<TypeId, Box<dyn PartialReflect>> = HashMap::new();
        for component_info in components {
            let Some(type_id) = component_info.type_id() else {
                continue;
            };

            if !allowed_types.contains(&type_id) {
                continue;
            }

            let Some(registration) = registry_guard.get(type_id) else {
                continue;
            };

            let Some(reflect_component) = registration.data::<ReflectComponent>() else {
                continue;
            };

            let Some(reflected) = reflect_component.reflect(entity_ref) else {
                continue;
            };

            updates.insert(type_id, reflected.to_dynamic());
        }
        world.resource_scope(|_world, mut canonical: Mut<CanonicalScene>| {
            canonical.insert_entity(entity, updates);
        });
    }
}

/// System that applies edit messages to the canonical scene.
fn apply_edit_messages(
    mut messages: MessageReader<ApplyEditMessage>,
    mut canonical: ResMut<CanonicalScene>,
    mut history: ResMut<EditHistory>,
) {
    for msg in messages.read() {
        let Some(component_data) =
            canonical.get_component_mut_by_id(msg.entity, msg.component_type)
        else {
            warn!(
                "Cannot apply edit: no canonical data for entity {:?} component {:?}",
                msg.entity, msg.component_type
            );
            continue;
        };

        // Capture old value before modification
        let old_value = if msg.field_path.is_empty() {
            component_data.to_dynamic()
        } else {
            match msg
                .field_path
                .as_str()
                .reflect_element(component_data.as_ref())
            {
                Ok(field) => field.to_dynamic(),
                Err(e) => {
                    warn!("Cannot read field path '{}': {:?}", msg.field_path, e);
                    continue;
                }
            }
        };

        // Apply new value
        let apply_result = if msg.field_path.is_empty() {
            component_data.apply(msg.new_value.as_ref());
            Ok(())
        } else {
            msg.field_path
                .as_str()
                .reflect_element_mut(component_data.as_mut())
                .map(|field| field.apply(msg.new_value.as_ref()))
        };

        if let Err(e) = apply_result {
            warn!(
                "Cannot apply edit to field path '{}': {:?}",
                msg.field_path, e
            );
            continue;
        }

        // Record in history
        let op = EditOp {
            entity: msg.entity,
            component_type: msg.component_type,
            field_path: msg.field_path.clone(),
            old_value,
            new_value: msg.new_value.to_dynamic(),
        };
        history.push(op);

        info!(
            "Applied edit to {:?}.{} on entity {:?}",
            msg.component_type, msg.field_path, msg.entity
        );
    }
}

/// System that handles undo requests.
fn handle_undo(
    mut messages: MessageReader<UndoMessage>,
    mut canonical: ResMut<CanonicalScene>,
    mut history: ResMut<EditHistory>,
) {
    for _ in messages.read() {
        let Some(op) = history.pop_undo() else {
            info!("Nothing to undo");
            continue;
        };

        let Some(component_data) = canonical.get_component_mut_by_id(op.entity, op.component_type)
        else {
            warn!(
                "Cannot undo: no canonical data for entity {:?} component {:?}",
                op.entity, op.component_type
            );
            continue;
        };

        // Apply old value (reverse the edit)
        let apply_result = if op.field_path.is_empty() {
            component_data.apply(op.old_value.as_ref());
            Ok(())
        } else {
            op.field_path
                .as_str()
                .reflect_element_mut(component_data.as_mut())
                .map(|field| field.apply(op.old_value.as_ref()))
        };

        if let Err(e) = apply_result {
            warn!(
                "Cannot undo edit to field path '{}': {:?}",
                op.field_path, e
            );
            continue;
        }

        // Move to redo stack
        history.push_redo(op);
        info!("Undo applied");
    }
}

/// System that handles redo requests.
fn handle_redo(
    mut messages: MessageReader<RedoMessage>,
    mut canonical: ResMut<CanonicalScene>,
    mut history: ResMut<EditHistory>,
) {
    for _ in messages.read() {
        let Some(op) = history.pop_redo() else {
            info!("Nothing to redo");
            continue;
        };

        let Some(component_data) = canonical.get_component_mut_by_id(op.entity, op.component_type)
        else {
            warn!(
                "Cannot redo: no canonical data for entity {:?} component {:?}",
                op.entity, op.component_type
            );
            continue;
        };

        // Apply new value (re-apply the edit)
        let apply_result = if op.field_path.is_empty() {
            component_data.apply(op.new_value.as_ref());
            Ok(())
        } else {
            op.field_path
                .as_str()
                .reflect_element_mut(component_data.as_mut())
                .map(|field| field.apply(op.new_value.as_ref()))
        };

        if let Err(e) = apply_result {
            warn!(
                "Cannot redo edit to field path '{}': {:?}",
                op.field_path, e
            );
            continue;
        }

        // Move back to undo stack
        history.push_undo(op);
        info!("Redo applied");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::reflect::Reflect;

    #[derive(Component, Reflect, Default, Clone, Debug, PartialEq)]
    #[reflect(Component, PlanetesComponent)]
    struct TestComponent {
        value: f32,
        name: String,
    }

    #[derive(Component, Reflect, Default, Clone, Debug, PartialEq)]
    #[reflect(Component, PlanetesComponent)]
    struct AnotherComponent {
        count: i32,
    }

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, plugin));
        app
    }

    mod canonical_scene {
        use super::*;

        #[test]
        fn sync_and_get_component() {
            let mut app = setup_test_app();

            let entity = app
                .world_mut()
                .spawn(TestComponent {
                    value: 42.0,
                    name: "test".to_string(),
                })
                .id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            let canonical = app.world().resource::<CanonicalScene>();
            let retrieved = canonical.get_component::<TestComponent>(entity);
            assert!(retrieved.is_some());

            let retrieved = retrieved.unwrap();
            let value_field = "value".reflect_element(retrieved).unwrap();
            assert_eq!(value_field.try_downcast_ref::<f32>(), Some(&42.0));
        }

        #[test]
        fn syncs_entity_and_children() {
            let mut app = setup_test_app();

            let parent = app
                .world_mut()
                .spawn(TestComponent {
                    value: 42.0,
                    name: "test".to_string(),
                })
                .id();

            let child = app
                .world_mut()
                .spawn((
                    TestComponent {
                        value: 40.0,
                        name: "child".to_string(),
                    },
                    ChildOf(parent),
                ))
                .id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity: parent });
            app.update();

            let canonical = app.world().resource::<CanonicalScene>();
            let retrieved_parent = canonical.get_component::<TestComponent>(parent);
            assert!(retrieved_parent.is_some());

            let retrieved_child = canonical.get_component::<TestComponent>(child);
            assert!(retrieved_child.is_some());

            let value_field = "value".reflect_element(retrieved_parent.unwrap()).unwrap();
            assert_eq!(value_field.try_downcast_ref::<f32>(), Some(&42.0));

            let value_field = "value".reflect_element(retrieved_child.unwrap()).unwrap();
            assert_eq!(value_field.try_downcast_ref::<f32>(), Some(&40.0));
        }

        #[test]
        fn get_nonexistent_component_returns_none() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn_empty().id();
            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            let canonical = app.world().resource::<CanonicalScene>();
            assert!(canonical.get_component::<TestComponent>(entity).is_none());
        }

        #[test]
        fn contains_entity_after_sync() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn(TestComponent::default()).id();

            let other_entity = app.world_mut().spawn_empty().id();

            {
                let canonical = app.world().resource::<CanonicalScene>();
                assert!(!canonical.contains_entity(entity));
                assert!(!canonical.contains_entity(other_entity));
            }

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.world_mut().write_message(SyncCanonicalMessage {
                entity: other_entity,
            });
            app.update();

            let canonical = app.world().resource::<CanonicalScene>();
            assert!(canonical.contains_entity(entity));
            assert!(canonical.contains_entity(other_entity));
        }

        #[test]
        fn get_entity_components_returns_all_synced() {
            let mut app = setup_test_app();

            let entity = app
                .world_mut()
                .spawn((TestComponent::default(), AnotherComponent::default()))
                .id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            let canonical = app.world().resource::<CanonicalScene>();
            let components = canonical.get_entity_components(entity);
            assert!(components.is_some());

            let components = components.unwrap();
            assert_eq!(components.len(), 2);
            assert!(components.contains_key(&TypeId::of::<TestComponent>()));
            assert!(components.contains_key(&TypeId::of::<AnotherComponent>()));
        }

        #[test]
        fn multiple_entities_independent() {
            let mut app = setup_test_app();

            let entity1 = app
                .world_mut()
                .spawn(TestComponent {
                    value: 1.0,
                    name: "first".to_string(),
                })
                .id();

            let entity2 = app
                .world_mut()
                .spawn(TestComponent {
                    value: 2.0,
                    name: "second".to_string(),
                })
                .id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity: entity1 });
            app.world_mut()
                .write_message(SyncCanonicalMessage { entity: entity2 });
            app.update();

            let canonical = app.world().resource::<CanonicalScene>();
            let comp1 = canonical.get_component::<TestComponent>(entity1).unwrap();
            let comp2 = canonical.get_component::<TestComponent>(entity2).unwrap();

            let value1 = "value"
                .reflect_element(comp1)
                .unwrap()
                .try_downcast_ref::<f32>();
            let value2 = "value"
                .reflect_element(comp2)
                .unwrap()
                .try_downcast_ref::<f32>();

            assert_eq!(value1, Some(&1.0));
            assert_eq!(value2, Some(&2.0));
        }

        #[test]
        fn edit_overwrites_canonical_value() {
            let mut app = setup_test_app();

            let entity = app
                .world_mut()
                .spawn(TestComponent {
                    value: 1.0,
                    name: "original".to_string(),
                })
                .id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(99.0f32),
            });
            app.update();

            let canonical = app.world().resource::<CanonicalScene>();
            let retrieved = canonical.get_component::<TestComponent>(entity).unwrap();
            let value = "value"
                .reflect_element(retrieved)
                .unwrap()
                .try_downcast_ref::<f32>();
            assert_eq!(value, Some(&99.0));
        }

        #[test]
        fn get_component_mut_allows_modification() {
            let mut app = setup_test_app();

            let entity = app
                .world_mut()
                .spawn(TestComponent {
                    value: 1.0,
                    name: "test".to_string(),
                })
                .id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            {
                let mut canonical = app.world_mut().resource_mut::<CanonicalScene>();
                let component_data = canonical
                    .get_component_mut_by_id(entity, TypeId::of::<TestComponent>())
                    .unwrap();
                let field = "value"
                    .reflect_element_mut(component_data.as_mut())
                    .unwrap();
                field.apply(&50.0f32);
            }

            let canonical = app.world().resource::<CanonicalScene>();
            let retrieved = canonical.get_component::<TestComponent>(entity).unwrap();
            let value = "value"
                .reflect_element(retrieved)
                .unwrap()
                .try_downcast_ref::<f32>();
            assert_eq!(value, Some(&50.0));
        }
    }

    mod edit_history {
        use super::*;

        #[test]
        fn edit_adds_to_undo_stack() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn(TestComponent::default()).id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            {
                let history = app.world().resource::<EditHistory>();
                assert!(!history.can_undo());
            }

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(1.0f32),
            });
            app.update();

            let history = app.world().resource::<EditHistory>();
            assert!(history.can_undo());
            assert!(!history.can_redo());
        }

        #[test]
        fn new_edit_clears_redo_stack() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn(TestComponent::default()).id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(1.0f32),
            });
            app.update();

            app.world_mut().write_message(UndoMessage);
            app.update();

            {
                let history = app.world().resource::<EditHistory>();
                assert!(history.can_redo());
            }

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(3.0f32),
            });
            app.update();

            let history = app.world().resource::<EditHistory>();
            assert!(!history.can_redo());
        }

        #[test]
        fn undo_pops_most_recent_edit() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn(TestComponent::default()).id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(1.0f32),
            });
            app.update();

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(2.0f32),
            });
            app.update();

            {
                let canonical = app.world().resource::<CanonicalScene>();
                let comp = canonical.get_component::<TestComponent>(entity).unwrap();
                let value = "value"
                    .reflect_element(comp)
                    .unwrap()
                    .try_downcast_ref::<f32>();
                assert_eq!(value, Some(&2.0));
            }

            app.world_mut().write_message(UndoMessage);
            app.update();

            {
                let canonical = app.world().resource::<CanonicalScene>();
                let comp = canonical.get_component::<TestComponent>(entity).unwrap();
                let value = "value"
                    .reflect_element(comp)
                    .unwrap()
                    .try_downcast_ref::<f32>();
                assert_eq!(value, Some(&1.0));
            }

            app.world_mut().write_message(UndoMessage);
            app.update();

            let canonical = app.world().resource::<CanonicalScene>();
            let comp = canonical.get_component::<TestComponent>(entity).unwrap();
            let value = "value"
                .reflect_element(comp)
                .unwrap()
                .try_downcast_ref::<f32>();
            assert_eq!(value, Some(&0.0));
        }

        #[test]
        fn clear_history_removes_all() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn(TestComponent::default()).id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(1.0f32),
            });
            app.update();

            app.world_mut().write_message(UndoMessage);
            app.update();

            {
                let history = app.world().resource::<EditHistory>();
                assert!(history.can_redo());
            }

            app.world_mut().resource_mut::<EditHistory>().clear();

            let history = app.world().resource::<EditHistory>();
            assert!(!history.can_undo());
            assert!(!history.can_redo());
        }
    }

    mod undo_redo {
        use super::*;

        #[test]
        fn undo_reverts_to_old_value() {
            let mut app = setup_test_app();

            let entity = app
                .world_mut()
                .spawn(TestComponent {
                    value: 10.0,
                    name: "test".to_string(),
                })
                .id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(50.0f32),
            });
            app.update();

            {
                let canonical = app.world().resource::<CanonicalScene>();
                let component = canonical.get_component::<TestComponent>(entity).unwrap();
                let value = "value"
                    .reflect_element(component)
                    .unwrap()
                    .try_downcast_ref::<f32>();
                assert_eq!(value, Some(&50.0));
            }

            app.world_mut().write_message(UndoMessage);
            app.update();

            let canonical = app.world().resource::<CanonicalScene>();
            let component = canonical.get_component::<TestComponent>(entity).unwrap();
            let value = "value"
                .reflect_element(component)
                .unwrap()
                .try_downcast_ref::<f32>();
            assert_eq!(value, Some(&10.0));
        }

        #[test]
        fn undo_moves_op_to_redo_stack() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn(TestComponent::default()).id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(50.0f32),
            });
            app.update();

            {
                let history = app.world().resource::<EditHistory>();
                assert!(history.can_undo());
                assert!(!history.can_redo());
            }

            app.world_mut().write_message(UndoMessage);
            app.update();

            let history = app.world().resource::<EditHistory>();
            assert!(!history.can_undo());
            assert!(history.can_redo());
        }

        #[test]
        fn redo_reapplies_value() {
            let mut app = setup_test_app();

            let entity = app
                .world_mut()
                .spawn(TestComponent {
                    value: 10.0,
                    name: "test".to_string(),
                })
                .id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(50.0f32),
            });
            app.update();

            app.world_mut().write_message(UndoMessage);
            app.update();

            {
                let canonical = app.world().resource::<CanonicalScene>();
                let component = canonical.get_component::<TestComponent>(entity).unwrap();
                let value = "value"
                    .reflect_element(component)
                    .unwrap()
                    .try_downcast_ref::<f32>();
                assert_eq!(value, Some(&10.0));
            }

            app.world_mut().write_message(RedoMessage);
            app.update();

            let canonical = app.world().resource::<CanonicalScene>();
            let component = canonical.get_component::<TestComponent>(entity).unwrap();
            let value = "value"
                .reflect_element(component)
                .unwrap()
                .try_downcast_ref::<f32>();
            assert_eq!(value, Some(&50.0));
        }

        #[test]
        fn redo_moves_op_back_to_undo_stack() {
            let mut app = setup_test_app();

            let entity = app.world_mut().spawn(TestComponent::default()).id();

            app.world_mut()
                .write_message(SyncCanonicalMessage { entity });
            app.update();

            app.world_mut().write_message(ApplyEditMessage {
                entity,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(50.0f32),
            });
            app.update();

            app.world_mut().write_message(UndoMessage);
            app.update();

            app.world_mut().write_message(RedoMessage);
            app.update();

            let history = app.world().resource::<EditHistory>();
            assert!(history.can_undo());
            assert!(!history.can_redo());
        }
    }
}
