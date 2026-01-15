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
//! - [`ApplyEdit`] - Message to request applying an edit to canonical state

use std::{any::TypeId, collections::HashMap};

use bevy::{
    ecs::{component::ComponentId, system::SystemState},
    prelude::*,
    reflect::{PartialReflect, ReflectPath},
    scene::DynamicEntity,
};

/// Reflectable Trait to mark Components as being available to the Editor
#[reflect_trait]
pub trait PlanetesComponent {}

impl<T: Reflect + Component> PlanetesComponent for T {}

/// Reflectable Trait to mark Components as being Hidden from the Editor
#[reflect_trait]
pub trait HiddenComponent {}

impl<T: Reflect + Component> HiddenComponent for T {}

/// Reflectable Trait to mark Bundles
#[reflect_trait]
pub trait PlanetesBundle {}

impl<T: Reflect + Bundle> PlanetesBundle for T {}

macro_rules! register_hidden_components {
    ($app: ident, $($t:ty),*) => {
        $app$(.register_type_data::<$t, ReflectHiddenComponent>())*;
    };
}

/// Plugin that registers canonical scene management systems.
pub fn plugin(app: &mut App) {
    app.init_resource::<CanonicalScene>()
        .init_resource::<EditHistory>()
        .add_message::<ApplyEdit>()
        .add_message::<Undo>()
        .add_message::<Redo>()
        .add_systems(
            Update,
            (
                collect_edit_history,
                apply_edit_messages,
                handle_undo,
                handle_redo,
            )
                .chain(),
        );
    register_hidden_components!(app, GlobalTransform, TransformTreeChanged);
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
/// ```no_run
/// # use planetes_scene_state::*;
/// # use bevy::prelude::*;
/// fn read_canonical_transform(
///     canonical: Res<CanonicalScene>,
///     mut transforms: Query<&mut Transform>,
///     entity: Entity,
/// ) {
///     if let Some(transform_data) = canonical.get_component::<Transform>(entity)
///         && let Ok(mut transform) = transforms.get_mut(entity) {
///         // Update the live Transform with the Canonical State
///         transform.apply(transform_data.data.as_ref());
///     }
/// }
/// ```
#[derive(Resource, Default)]
pub struct CanonicalScene {
    /// Maps entities to their canonical component data.
    /// Inner map: TypeId -> Reflected component data
    handle: Handle<DynamicScene>,
}

impl CanonicalScene {
    pub fn insert(&mut self, handle: Handle<DynamicScene>) {
        self.handle = handle;
    }

    pub fn get_scene<'a>(&self, assets: &'a Assets<DynamicScene>) -> Option<&'a DynamicScene> {
        assets.get(&self.handle)
    }

    pub fn get_entity<'a>(
        &self,
        assets: &'a Assets<DynamicScene>,
        entity: Entity,
    ) -> Option<&'a DynamicEntity> {
        assets
            .get(&self.handle)
            .and_then(|scene| scene.entities.iter().find(|e| e.entity == entity))
    }

    pub fn get_component_by_id<'a>(
        &self,
        assets: &'a Assets<DynamicScene>,
        entity: Entity,
        type_id: TypeId,
    ) -> Option<&'a dyn PartialReflect> {
        assets
            .get(&self.handle)
            .and_then(|scene| scene.entities.iter().find(|e| e.entity == entity))
            .and_then(|entity| {
                entity.components.iter().find(|c| {
                    c.as_ref()
                        .get_represented_type_info()
                        .map(|info| info.type_id() == type_id)
                        .unwrap_or_default()
                })
            })
            .map(|component| component.as_ref())
    }

    pub fn get_component<'a, T: Component + FromReflect + 'static>(
        &self,
        assets: &'a Assets<DynamicScene>,
        entity: Entity,
    ) -> Option<&'a T> {
        assets
            .get(&self.handle)
            .and_then(|scene| scene.entities.iter().find(|e| e.entity == entity))
            .and_then(|entity| {
                entity.components.iter().find(|c| {
                    c.as_ref()
                        .get_represented_type_info()
                        .map(|info| info.type_id() == TypeId::of::<T>())
                        .unwrap_or_default()
                })
            })
            .and_then(|c| c.as_ref().try_as_reflect())
            .and_then(|c| c.downcast_ref::<T>())
    }

    /// Returns mutable access to the canonical data for a specific component.
    pub fn get_component_mut_by_id<'a>(
        &self,
        assets: &'a mut Assets<DynamicScene>,
        entity: Entity,
        type_id: TypeId,
    ) -> Option<&'a mut dyn PartialReflect> {
        assets
            .get_mut(&self.handle)
            .and_then(|scene| scene.entities.iter_mut().find(|e| e.entity == entity))
            .and_then(|entity| {
                entity.components.iter_mut().find(|c| {
                    c.as_ref()
                        .get_represented_type_info()
                        .map(|info| info.type_id() == type_id)
                        .unwrap_or_default()
                })
            })
            .map(|component| component.as_mut())
    }

    pub fn get_component_mut<'a, T: Component + FromReflect + 'static>(
        &self,
        assets: &'a mut Assets<DynamicScene>,
        entity: Entity,
    ) -> Option<&'a mut T> {
        assets
            .get_mut(&self.handle)
            .and_then(|scene| scene.entities.iter_mut().find(|e| e.entity == entity))
            .and_then(|entity| {
                entity.components.iter_mut().find(|c| {
                    c.as_ref()
                        .get_represented_type_info()
                        .map(|info| info.type_id() == TypeId::of::<T>())
                        .unwrap_or_default()
                })
            })
            .and_then(|c| c.as_mut().try_as_reflect_mut())
            .and_then(|c| c.downcast_mut::<T>())
    }

    pub fn insert_entity(
        &mut self,
        entity: Entity,
        components: HashMap<TypeId, CanonicalComponentState>,
    ) {
        // self.entities.insert(
        //     entity,
        //     CanonicalEntityState {
        //         entity,
        //         components,
        //         changed: true,
        //     },
        // );
    }

    /// Inserts or updates canonical data for a component on an entity.
    pub fn insert_component(
        &mut self,
        entity: Entity,
        type_id: TypeId,
        component: CanonicalComponentState,
    ) {
        // self.entities
        //     .entry(entity)
        //     .or_insert(CanonicalEntityState::new(entity))
        //     .components
        //     .insert(type_id, component);
    }

    /// Removes all canonical data for an entity.
    pub fn remove_entity(&mut self, entity: Entity) {
        // self.entities.remove(&entity);
    }

    /// Returns all component type IDs stored for an entity.
    pub fn get_entity_components(
        &self,
        entity: Entity,
    ) -> Option<&HashMap<TypeId, CanonicalComponentState>> {
        None
        // self.entities.get(&entity).map(|state| &state.components)
    }

    /// Checks if an entity has any canonical data stored.
    pub fn contains_entity(&self, entity: Entity) -> bool {
        false
        // self.entities.contains_key(&entity)
    }
}

/// Contains all the canonical data for an Entity
pub struct CanonicalEntityState {
    pub entity: Entity,
    pub components: HashMap<TypeId, CanonicalComponentState>,
    /// Whether the entity has been modified. Makes it easier to apply changes without extra work.
    pub changed: bool,
}

impl CanonicalEntityState {
    pub fn new(entity: Entity) -> Self {
        CanonicalEntityState {
            entity,
            components: HashMap::new(),
            changed: false,
        }
    }
}

/// Tracks the state of a specific Component.
pub struct CanonicalComponentState {
    id: ComponentId,
    name: DebugName,
    type_id: TypeId,
    /// The data that the component holds.
    pub data: Box<dyn PartialReflect>,
}

impl CanonicalComponentState {
    pub fn id(&self) -> ComponentId {
        self.id
    }

    pub fn name(&self) -> &DebugName {
        &self.name
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
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

impl Extend<EditOp> for EditHistory {
    fn extend<T: IntoIterator<Item = EditOp>>(&mut self, iter: T) {
        self.undo_stack.extend(iter);
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
pub struct ApplyEdit {
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
/// Opposite of [Redo]
#[derive(Message)]
pub struct Undo;

/// Message to request a redo operation.
/// Opposite of [Undo]
#[derive(Message)]
pub struct Redo;

/// System that applies edit messages to the canonical scene.
fn apply_edit_messages(
    mut messages: MessageReader<ApplyEdit>,
    mut assets: ResMut<Assets<DynamicScene>>,
    canonical_scene: Res<CanonicalScene>,
) {
    for msg in messages.read() {
        let Some(component_data) =
            canonical_scene.get_component_mut_by_id(&mut assets, msg.entity, msg.component_type)
        else {
            warn!(
                "Cannot apply edit: no canonical data for entity {:?} component {:?}",
                msg.entity, msg.component_type
            );
            continue;
        };

        // Apply new value
        let apply_result = if msg.field_path.is_empty() {
            component_data.apply(msg.new_value.as_ref());
            Ok(())
        } else {
            msg.field_path
                .as_str()
                .reflect_element_mut(component_data)
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
        info!(
            "Applied edit to {:?}.{} on entity {:?}",
            msg.component_type, msg.field_path, msg.entity
        );
    }
}

/// Collects all edit events and records them in the history, to provide Undo/Redo functionality.
fn collect_edit_history(
    mut messages: MessageReader<ApplyEdit>,
    mut history: ResMut<EditHistory>,
    assets: Res<Assets<DynamicScene>>,
    canonical_scene: Res<CanonicalScene>,
) {
    if messages.is_empty() {
        return;
    }

    history.extend(messages.read().filter_map(|msg| {
        canonical_scene
            .get_component_by_id(&assets, msg.entity, msg.component_type)
            .and_then(|component_state| {
                if msg.field_path.is_empty() {
                    Some(component_state.to_dynamic())
                } else {
                    match msg.field_path.as_str().reflect_element(component_state) {
                        Ok(field) => Some(field.to_dynamic()),
                        Err(e) => {
                            warn!("Cannot read field path '{}': {:?}", msg.field_path, e);
                            None
                        }
                    }
                }
            })
            .map(|old_value| EditOp {
                entity: msg.entity,
                component_type: msg.component_type,
                field_path: msg.field_path.clone(),
                old_value,
                new_value: msg.new_value.to_dynamic(),
            })
    }));
}

/// System that handles undo requests.
fn handle_undo(
    mut messages: MessageReader<Undo>,
    mut assets: ResMut<Assets<DynamicScene>>,
    canonical_scene: Res<CanonicalScene>,
    mut history: ResMut<EditHistory>,
) {
    for _ in messages.read() {
        let Some(op) = history.pop_undo() else {
            info!("Nothing to undo");
            continue;
        };

        let Some(component_data) =
            canonical_scene.get_component_mut_by_id(&mut assets, op.entity, op.component_type)
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
                .reflect_element_mut(component_data)
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
    mut messages: MessageReader<Redo>,
    mut assets: ResMut<Assets<DynamicScene>>,
    canonical_scene: Res<CanonicalScene>,
    mut history: ResMut<EditHistory>,
) {
    for _ in messages.read() {
        let Some(op) = history.pop_redo() else {
            info!("Nothing to redo");
            continue;
        };

        let Some(component_data) =
            canonical_scene.get_component_mut_by_id(&mut assets, op.entity, op.component_type)
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
                .reflect_element_mut(component_data)
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
    use bevy::{
        reflect::Reflect,
        scene::{ScenePlugin, scene_spawner_system},
    };

    #[derive(Component, Reflect, Default, Clone, Debug, PartialEq)]
    #[reflect(Component, PlanetesComponent, Default)]
    struct TestComponent {
        value: f32,
        name: String,
    }

    #[derive(Component, Reflect, Default, Clone, Debug, PartialEq)]
    #[reflect(Component, PlanetesComponent, Default)]
    struct AnotherComponent {
        count: i32,
    }

    const TEST_ENTITY: Entity = Entity::from_bits(4294967160);

    fn setup_test_app() -> App {
        use bevy::scene::DynamicEntity;
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            ScenePlugin::default(),
            plugin,
        ));

        app.world_mut()
            .resource_scope(|world, server: Mut<AssetServer>| {
                let scene = DynamicScene {
                    entities: vec![DynamicEntity {
                        entity: TEST_ENTITY,
                        components: vec![Box::new(TestComponent {
                            value: 42.0,
                            name: "test".to_string(),
                        })],
                    }],
                    resources: Vec::new(),
                };
                let handle = server.add(scene);
                world.resource_scope(|_world, mut canonical: Mut<CanonicalScene>| {
                    canonical.insert(handle.clone());
                });
                world.spawn(DynamicSceneRoot(handle));
            });
        app.world_mut().flush();
        for _ in 0..16 {
            app.update();
        }
        app
    }

    mod canonical_scene {

        use bevy::reflect::ReflectRef;

        use super::*;

        #[test]
        fn get_scene() {
            let app = setup_test_app();

            let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                panic!("Failed to get Server");
            };

            let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                panic!("Failed to get CanonicalScene");
            };

            assert!(canonical.get_scene(&assets).is_some());
        }

        #[test]
        fn get_entity() {
            let app = setup_test_app();

            let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                panic!("Failed to get Server");
            };

            let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                panic!("Failed to get CanonicalScene");
            };

            let other = canonical.get_entity(&assets, TEST_ENTITY);
            assert!(other.is_some());
            assert_eq!(other.unwrap().entity, TEST_ENTITY);
        }

        #[test]
        fn get_component() {
            let app = setup_test_app();

            let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                panic!("Failed to get Server");
            };

            let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                panic!("Failed to get CanonicalScene");
            };

            let other = canonical.get_component::<TestComponent>(&assets, TEST_ENTITY);
            assert!(other.is_some());
            assert_eq!(other.unwrap().value, 42.0);

            let by_id =
                canonical.get_component_by_id(&assets, TEST_ENTITY, TypeId::of::<TestComponent>());
            assert!(by_id.is_some());
            match by_id.unwrap().reflect_ref() {
                ReflectRef::Struct(data) => {
                    assert_eq!(
                        data.field("value")
                            .unwrap()
                            .try_as_reflect()
                            .expect("Should be Reflectable")
                            .downcast_ref::<f32>(),
                        Some(&42.0)
                    );
                }
                _ => panic!("Unexpected component type"),
            }

            assert!(
                canonical
                    .get_component::<Children>(&assets, TEST_ENTITY)
                    .is_none()
            );
        }
    }

    #[test]
    fn edit_overwrites_canonical_value() {
        let mut app = setup_test_app();

        {
            let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                panic!("Failed to get Server");
            };

            let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                panic!("Failed to get CanonicalScene");
            };

            let component = canonical
                .get_component::<TestComponent>(&assets, TEST_ENTITY)
                .unwrap();
            assert_eq!(component.value, 42.0);
        }

        app.update();

        app.world_mut().write_message(ApplyEdit {
            entity: TEST_ENTITY,
            component_type: TypeId::of::<TestComponent>(),
            field_path: "value".to_string(),
            new_value: Box::new(99.0f32),
        });

        app.update();

        {
            let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                panic!("Failed to get Server");
            };

            let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                panic!("Failed to get CanonicalScene");
            };

            let component = canonical
                .get_component::<TestComponent>(&assets, TEST_ENTITY)
                .unwrap();
            assert_eq!(component.value, 99.0);
        }
    }

    #[test]
    fn edit_applies_to_live_entity() {
        let mut app = setup_test_app();

        {
            let mut query = app.world_mut().query::<&TestComponent>();
            let component = query.single(&app.world()).unwrap();
            assert_eq!(component.value, 42.0);
            app.world_mut().write_message(ApplyEdit {
                entity: TEST_ENTITY,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(99.0f32),
            });
        }

        for _ in 0..16 {
            app.update();
        }

        {
            let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                panic!("Failed to get Server");
            };

            let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                panic!("Failed to get CanonicalScene");
            };

            let component = canonical
                .get_component::<TestComponent>(&assets, TEST_ENTITY)
                .unwrap();
            assert_eq!(component.value, 99.0);
        }

        {
            let mut query = app.world_mut().query::<&TestComponent>();
            let component = query.single(&app.world()).unwrap();
            assert_eq!(component.value, 99.0);
        }
    }

    mod edit_history {
        use super::*;

        #[test]
        fn edit_adds_to_undo_stack() {
            let mut app = setup_test_app();

            {
                let history = app.world().resource::<EditHistory>();
                assert!(!history.can_undo());
            }

            app.world_mut().write_message(ApplyEdit {
                entity: TEST_ENTITY,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(1.0f32),
            });
            app.update();

            let history = app.world().resource::<EditHistory>();
            assert!(history.can_undo());
            assert!(!history.can_redo());
            assert!(history.undo_stack.len() == 1);
            let edit_op = history.undo_stack.get(0).unwrap();
            assert_eq!(edit_op.entity, TEST_ENTITY);
            assert_eq!(edit_op.component_type, TypeId::of::<TestComponent>());
            assert_eq!(edit_op.field_path, "value".to_string());
            assert_eq!(
                edit_op.old_value.as_ref().try_downcast_ref::<f32>(),
                Some(&42.0f32)
            );
            assert_eq!(
                edit_op.new_value.as_ref().try_downcast_ref::<f32>(),
                Some(&1.0f32)
            );
        }

        #[test]
        fn new_edit_clears_redo_stack() {
            let mut app = setup_test_app();

            app.world_mut().write_message(ApplyEdit {
                entity: TEST_ENTITY,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(1.0f32),
            });
            app.update();

            {
                let history = app.world().resource::<EditHistory>();
                assert!(!history.can_redo());
            }

            app.world_mut().write_message(Undo);
            app.update();

            {
                let history = app.world().resource::<EditHistory>();
                assert!(history.can_redo());
            }

            app.world_mut().write_message(ApplyEdit {
                entity: TEST_ENTITY,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(3.0f32),
            });
            app.update();

            let history = app.world().resource::<EditHistory>();
            assert!(!history.can_redo());
        }

        #[test]
        fn clear_history_removes_all() {
            let mut app = setup_test_app();

            app.world_mut().write_message(ApplyEdit {
                entity: TEST_ENTITY,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(1.0f32),
            });
            app.update();

            {
                let history = app.world().resource::<EditHistory>();
                assert!(history.can_undo());
                assert!(!history.can_redo());
            }

            app.world_mut().write_message(Undo);
            app.update();

            {
                let history = app.world().resource::<EditHistory>();
                assert!(history.can_redo());
                assert!(!history.can_undo());
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

            app.world_mut().write_message(ApplyEdit {
                entity: TEST_ENTITY,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(50.0f32),
            });
            app.update();

            {
                let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                    panic!("Failed to get Server");
                };

                let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                    panic!("Failed to get CanonicalScene");
                };

                let component = canonical
                    .get_component::<TestComponent>(&assets, TEST_ENTITY)
                    .unwrap();
                assert_eq!(component.value, 50.0);
            }

            app.world_mut().write_message(ApplyEdit {
                entity: TEST_ENTITY,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(99.0f32),
            });
            app.update();

            {
                let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                    panic!("Failed to get Server");
                };

                let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                    panic!("Failed to get CanonicalScene");
                };

                let component = canonical
                    .get_component::<TestComponent>(&assets, TEST_ENTITY)
                    .unwrap();
                assert_eq!(component.value, 99.0);
            }

            app.world_mut().write_message(Undo);
            app.update();

            {
                let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                    panic!("Failed to get Server");
                };

                let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                    panic!("Failed to get CanonicalScene");
                };

                let component = canonical
                    .get_component::<TestComponent>(&assets, TEST_ENTITY)
                    .unwrap();
                assert_eq!(component.value, 50.0);
            }

            app.world_mut().write_message(Undo);
            app.update();

            {
                let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                    panic!("Failed to get Server");
                };

                let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                    panic!("Failed to get CanonicalScene");
                };

                let component = canonical
                    .get_component::<TestComponent>(&assets, TEST_ENTITY)
                    .unwrap();
                assert_eq!(component.value, 42.0);
            }
        }

        #[test]
        fn undo_moves_op_to_redo_stack() {
            let mut app = setup_test_app();

            app.world_mut().write_message(ApplyEdit {
                entity: TEST_ENTITY,
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

            app.world_mut().write_message(Undo);
            app.update();

            let history = app.world().resource::<EditHistory>();
            assert!(!history.can_undo());
            assert!(history.can_redo());

            assert!(history.redo_stack.len() == 1);
            let edit_op = history.redo_stack.get(0).unwrap();
            assert_eq!(edit_op.entity, TEST_ENTITY);
            assert_eq!(edit_op.component_type, TypeId::of::<TestComponent>());
            assert_eq!(edit_op.field_path, "value".to_string());
            assert_eq!(
                edit_op.old_value.as_ref().try_downcast_ref::<f32>(),
                Some(&42.0f32)
            );
            assert_eq!(
                edit_op.new_value.as_ref().try_downcast_ref::<f32>(),
                Some(&50.0f32)
            );
        }

        #[test]
        fn redo_reapplies_value() {
            let mut app = setup_test_app();

            app.world_mut().write_message(ApplyEdit {
                entity: TEST_ENTITY,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(50.0f32),
            });
            app.update();

            app.world_mut().write_message(Undo);
            app.update();

            {
                let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                    panic!("Failed to get Server");
                };

                let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                    panic!("Failed to get CanonicalScene");
                };

                let component = canonical
                    .get_component::<TestComponent>(&assets, TEST_ENTITY)
                    .unwrap();
                assert_eq!(component.value, 42.0);
            }

            app.world_mut().write_message(Redo);
            app.update();

            {
                let Some(assets) = app.world().get_resource_ref::<Assets<DynamicScene>>() else {
                    panic!("Failed to get Server");
                };

                let Some(canonical) = app.world().get_resource_ref::<CanonicalScene>() else {
                    panic!("Failed to get CanonicalScene");
                };

                let component = canonical
                    .get_component::<TestComponent>(&assets, TEST_ENTITY)
                    .unwrap();
                assert_eq!(component.value, 50.0);
            }
        }

        #[test]
        fn redo_moves_op_back_to_undo_stack() {
            let mut app = setup_test_app();

            app.world_mut().write_message(ApplyEdit {
                entity: TEST_ENTITY,
                component_type: TypeId::of::<TestComponent>(),
                field_path: "value".to_string(),
                new_value: Box::new(50.0f32),
            });
            app.update();

            let history = app.world().resource::<EditHistory>();
            assert!(history.can_undo());
            assert!(!history.can_redo());

            app.world_mut().write_message(Undo);
            app.update();

            let history = app.world().resource::<EditHistory>();
            assert!(!history.can_undo());
            assert!(history.can_redo());

            app.world_mut().write_message(Redo);
            app.update();

            let history = app.world().resource::<EditHistory>();
            assert!(history.can_undo());
            assert!(!history.can_redo());

            assert!(history.undo_stack.len() == 1);
            let edit_op = history.undo_stack.get(0).unwrap();
            assert_eq!(edit_op.entity, TEST_ENTITY);
            assert_eq!(edit_op.component_type, TypeId::of::<TestComponent>());
            assert_eq!(edit_op.field_path, "value".to_string());
            assert_eq!(
                edit_op.old_value.as_ref().try_downcast_ref::<f32>(),
                Some(&42.0f32)
            );
            assert_eq!(
                edit_op.new_value.as_ref().try_downcast_ref::<f32>(),
                Some(&50.0f32)
            );
        }
    }
}
