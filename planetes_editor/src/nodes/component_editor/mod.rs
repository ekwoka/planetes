//! Renders the UI for editing components
pub mod component_ui;
pub use component_ui::*;

use std::any::TypeId;

use crate::{
    atoms::{button, check_box, input_field},
    editor_ui::Capitalize,
    nodes::entity_viewer::{UpdateEntityViewer, Viewing},
    prelude::*,
};
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusedInput, InputFocus},
    prelude::*,
    reflect::{EnumInfo, OpaqueInfo, StructInfo, TupleStructInfo, TypeInfo},
};
use planetes_input::prelude::*;
use planetes_scene_state::CanonicalScene;

pub fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, update_component_editor);
}

/// Converts a basic Component Editor into a full Component Editor
pub fn update_component_editor(
    mut commands: Commands,
    target: Single<&Viewing>,
    editors: Query<(Entity, &ComponentEditor), Changed<ComponentEditor>>,
    scenes: Res<Assets<DynamicScene>>,
    canonical_scene: Res<CanonicalScene>,
    registry: Res<AppTypeRegistry>,
) {
    let registry = registry.read();
    for (editor, &ComponentEditor(type_id)) in editors {
        let Some(type_info) = registry.get_type_info(type_id) else {
            continue;
        };

        let Some(reflected) = canonical_scene.get_component_by_id(&scenes, target.0, type_id)
        else {
            continue;
        };

        if let Some(editor_view) = registry.get_type_data::<ReflectEditorView>(type_id) {
            let Some(reflected) = reflected.try_as_reflect() else {
                warn!("Not reflectable");
                continue;
            };
            let Some(component_with_view) = editor_view.get(&*reflected) else {
                warn!("Not Editor View");
                continue;
            };
            commands
                .entity(editor)
                .despawn_children()
                .with_children(|parent| {
                    component_with_view.add_to_parent(parent);
                });
        } else {
            commands
                .entity(editor)
                .despawn_children()
                .with_child(full(type_info.clone(), reflected.to_dynamic()));
        };
    }
}

/// Indicates the root of a component editor panel
#[derive(Component)]
pub struct ComponentEditor(pub TypeId);

/// Tracks the path from the root of the component to the current input
#[derive(Component)]
pub struct Path(pub String);

/// Renders a base Component Editor without content
pub fn base(type_id: TypeId) -> impl Bundle {
    html! {
        <div
            padding-left="2px"
            flex-grow="0"
            flex-shrink="1"
            display="flex"
            flex-direction="col"
            row-gap="4px"
            width="100%"
            components={ComponentEditor(type_id)}>
            <span linebreak={LineBreak::WordBoundary}>
              "Hello World"
            </span>
        </div>
    }
}

#[derive(Component)]
pub struct RemoveComponentButton(pub TypeId);

/// Builds out the full Component Editor with content
pub fn full(type_info: TypeInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
    html! {
        <div display="flex" flex-direction="column" row-gap="2px" width="100%" onenter={handle_commit}>
            <div onClick={handle_remove_component} components={RemoveComponentButton(type_info.type_id())}>{button::render("Remove X")}</div>
            <with>
            {
                match type_info {
                    TypeInfo::Struct(info) => {
                        if info.field_len() != 0 {
                            parent.spawn(struct_component(info, reflect))
                        } else {
                            parent.spawn(unit_component())
                        }
                    },
                    TypeInfo::TupleStruct(info) => {
                        parent.spawn(tuple_struct_component(info, reflect))
                    }
                    TypeInfo::Enum(info) => {
                        parent.spawn(enum_component(info, reflect))
                    }
                    _ => parent.spawn(unknown_component()),
                };
            }
            </with>
        </div>
    }
}

pub fn handle_remove_component(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    buttons: Query<&RemoveComponentButton>,
    editing: Single<&Viewing>,
    world: &World,
) {
    if let Ok(button) = buttons.get(event.entity) {
        let entity = editing.0;
        info!("Removing component: {:?} from {:?}", button.0, entity);
        if let Some(component_id) = world.components().get_id(button.0) {
            commands.entity(entity).remove_by_id(component_id);
            commands.trigger(UpdateEntityViewer(entity));
        } else {
            error!("Component to remove is not a valid component");
        }
    }
}

/// Commits the editing of an [InputField] to the [CanonicalScene]
fn handle_commit(
    event: On<FocusedInput<KeyboardInput>>,
    inputs: Query<&InputValue>,
    component_editor: Query<&ComponentEditor>,
    target: Single<&Viewing>,
    path_segments: Query<&Path>,
    ancestors: Query<&ChildOf>,
    focused: Res<InputFocus>,
    mut scenes: ResMut<Assets<DynamicScene>>,
    canonical_scene: Res<CanonicalScene>,
) {
    if event.input.logical_key != Key::Enter || event.input.state != ButtonState::Pressed {
        return;
    }
    let Some(focused_entity) = focused.0 else {
        warn!("No focused entity");
        return;
    };
    let Some(input_field) = inputs.get(focused_entity).ok() else {
        warn!("No input field found");
        return;
    };
    let Some(ComponentEditor(type_id)) = ancestors
        .iter_ancestors(focused_entity)
        .find_map(|entity| component_editor.get(entity).ok())
    else {
        warn!("No component editor found");
        return;
    };

    let Some(reflected_component) =
        canonical_scene.get_component_mut_by_id(&mut scenes, target.0, *type_id)
    else {
        warn!("No component found");
        return;
    };
    let mut paths = ancestors
        .iter_ancestors(focused_entity)
        .filter_map(|entity| path_segments.get(entity).ok())
        .map(|path| path.0.clone())
        .collect::<Vec<_>>();

    paths.reverse();

    let path = paths.join("");

    info!("Applying Data on path: {path:?} to component for type: {type_id:?}");

    let Ok(reflected_value) = path
        .reflect_element_mut(reflected_component)
        .inspect_err(|error| {
            warn!("Failed to reflect element at path: {path:?}: {error}");
        })
    else {
        return;
    };
    let Some(value) = input_field.value() else {
        return;
    };
    reflected_value.apply(value);
}

/// Renders the editor for a Unit Component
fn unit_component() -> impl Bundle {
    html! {
        <span linebreak={LineBreak::WordBoundary}>"Unit Struct"</span>
    }
}

/// Renders the editor for an Unknown Component
fn unknown_component() -> impl Bundle {
    html! {
        <span linebreak={LineBreak::WordBoundary}>"Unknown Struct"</span>
    }
}

/// Renders the editor for a Struct Component
fn struct_component(info: StructInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
    let struct_data = reflect.reflect_owned().into_struct().unwrap();
    let fields = info.iter().cloned().collect::<Vec<_>>();
    html! {
        <div
            width="100%"
            display="flex"
            flex-direction="col"
            row-gap="4px">
            <iter>
            {
               fields.into_iter().map(move |field| {
                   let name = format!("{}: ", field.name().capitalize_words());
                   let value = struct_data
                       .field(field.name())
                       .map(|partial| partial.to_dynamic());
                   html! {
                       <div
                          display="flex"
                          flex-direction="row"
                          align-items={AlignItems::Center}
                          column-gap="4px"
                          components={Path(format!(".{}", field.name()))}>
                          <div flex-grow="1">
                            <span>{name}</span>
                          </div>
                          <with>
                          {
                              match value {
                                  None => {
                                      parent.spawn(Text::new("Unknown Field"));
                                  }
                                  Some(value) => match value.get_represented_type_info() {
                                      Some(TypeInfo::TupleStruct(info)) => {
                                          parent.spawn(reflected_tuple_struct(info, value));
                                      }
                                      Some(TypeInfo::Struct(info)) => {
                                          parent.spawn(reflected_struct(info, value));
                                      }
                                      Some(TypeInfo::Tuple(_)) => {
                                          parent.spawn(Text::new("Unknown Tuple"));
                                      }
                                      Some(TypeInfo::List(_)) => {
                                          parent.spawn(Text::new("Unknown List"));
                                      }
                                      Some(TypeInfo::Opaque(info)) => {
                                          parent.spawn(reflected_opaque(info, value));
                                      }
                                      _ => {
                                          parent.spawn(Text::new("Unknown Type"));
                                      }
                                  },
                              };
                          }
                          </with>
                       </div>
                   }
               })
           }
           </iter>
        </div>
    }
}

/// Renders the editor for a Tuple Struct Component
fn tuple_struct_component(info: TupleStructInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
    let struct_data = reflect.reflect_owned().into_tuple_struct().unwrap();
    let fields = info.iter().cloned().collect::<Vec<_>>();
    html! {
        <div
            width="100%"
            display="flex"
            flex-direction="col"
            row-gap="4px">
            <iter>
            {
               fields.into_iter().map(move |field| {
                   let name = format!("{}: ", field.index());
                   let value = struct_data
                       .field(field.index())
                       .map(|partial| partial.to_dynamic());
                   html! {
                       <div
                          display="flex"
                          flex-direction="row"
                          column-gap="4px">
                          <div flex-grow="1">
                            <span>{name}</span>
                          </div>
                          <with>
                          {
                              match value {
                                  None => {
                                      parent.spawn(Text::new("Unknown Field"));
                                  }
                                  Some(value) => match value.get_represented_type_info() {
                                      Some(TypeInfo::TupleStruct(info)) => {
                                          parent.spawn(reflected_tuple_struct(info, value));
                                      }
                                      Some(TypeInfo::Struct(info)) => {
                                          parent.spawn(reflected_struct(info, value));
                                      }
                                      Some(TypeInfo::Tuple(_)) => {
                                          parent.spawn(Text::new("Unknown Tuple"));
                                      }
                                      Some(TypeInfo::List(_)) => {
                                          parent.spawn(Text::new("Unknown List"));
                                      }
                                      Some(TypeInfo::Opaque(info)) => {
                                          parent.spawn(reflected_opaque(info, value));
                                      }
                                      other => {
                                          parent.spawn(Text::new("Unknown Type"));
                                          parent.spawn(Text::new(format!("{other:?}")));
                                      }
                                  },
                              };
                          }
                          </with>
                       </div>
                   }
               })
           }
           </iter>
        </div>
    }
}

/// Renders the editor for Enum Components
fn enum_component(info: EnumInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
    let enum_data = reflect.reflect_owned().into_enum().unwrap();
    let variants = info.iter().cloned().collect::<Vec<_>>();
    html! {
        <div
            width="100%"
            display="flex"
            flex-direction="col"
            row-gap="4px">
            <iter>
            {
               variants.into_iter().map(move |variant| {
                   let name = format!("{}: ", variant.name());
                   let is_this = enum_data
                       .variant_name() == variant.name();
                   html! {
                       <div
                          display="flex"
                          flex-direction="row"
                          column-gap="4px">
                          <div flex-grow="1">
                            <span>{name}</span>
                          </div>
                          <span>{format!("{is_this:?}")}</span>
                       </div>
                   }
               })
           }
           </iter>
        </div>
    }
}

fn reflected_tuple_struct(info: &TupleStructInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
    let name = info.type_path_table().ident().unwrap_or("Unknown Ident");
    let tuple_struct = reflect
        .reflect_ref()
        .as_tuple_struct()
        .unwrap()
        .iter_fields()
        .map(|field| format!("{field:?}"))
        .collect::<Vec<String>>();
    html! {
        <div
           display="flex"
           flex-direction="row"
           column-gap="4px">
           <span>{name}</span>
           <div
              display="flex"
              flex-direction="row"
              column-gap="2px">
              <iter>
              {
                  tuple_struct.into_iter().map(|field| {
                      Text::new(field)
                  })
              }
              </iter>
            </div>
        </div>
    }
}

fn reflected_struct(info: &StructInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
    let name = info.type_path_table().ident().unwrap_or("Unknown Ident");
    let reflect_struct = reflect.reflect_owned().into_struct().unwrap();
    let children = info
        .iter()
        .zip(reflect_struct.iter_fields())
        .map(|(field, value)| {
            let type_info = field.clone().type_info();
            let value = value.to_dynamic();
            let input = type_info.and_then(|type_info| match type_info {
                TypeInfo::Opaque(info) => Some(info),
                _ => None,
            });
            html! {
                <div
                   display="flex"
                   flex-direction="row"
                   align-items={AlignItems::Center}
                   column-gap="4px"
                   components={Path(format!(".{}", field.name()))}>
                   <span>{field.name().capitalize_words().to_string()}</span>
                   <with>
                       {
                           if let Some(input_type) = input {
                               parent.spawn(reflected_opaque(input_type, value));
                           } else {
                               parent.spawn(Text::new(format!("{value:?}")));
                           }
                       }
                   </with>
                </div>
            }
        })
        .collect::<Vec<_>>();
    html! {
        <div
           display="flex"
           flex-direction="row"
           align-items={AlignItems::Center}
           column-gap="16px">
           <span>{name}</span>
            <div
                display={Display::Flex}
                flex-direction="row"
                column-gap="8px">
                <iter>
                {
                    children.into_iter()
                }
                </iter>
            </div>
        </div>
    }
}

fn reflected_opaque(input_type: &OpaqueInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
    let input_type = input_type.clone();
    let reflect = reflect.to_dynamic();
    html! {
        <div>
           <with>
               {
                    if input_type.is::<String>() {
                        parent.spawn(input_field::<String>(format!("{reflect:?}")));
                    } else if input_type.is::<f32>() {
                        parent.spawn(input_field::<f32>(reflect.as_partial_reflect().try_downcast_ref::<f32>().cloned().unwrap()));
                    } else if input_type.is::<u32>() {
                        parent.spawn(input_field::<u32>(reflect.as_partial_reflect().try_downcast_ref::<u32>().cloned().unwrap()));
                    } else if input_type.is::<u64>() {
                        parent.spawn(input_field::<u64>(reflect.as_partial_reflect().try_downcast_ref::<u64>().cloned().unwrap()));
                    } else if input_type.is::<i32>() {
                        parent.spawn(input_field::<i32>(reflect.as_partial_reflect().try_downcast_ref::<i32>().cloned().unwrap()));
                    } else if input_type.is::<bool>() {
                        parent.spawn(check_box(reflect.as_partial_reflect().try_downcast_ref::<bool>().cloned().unwrap()));
                    } else {
                       parent.spawn(Text::new(format!("{reflect:?}")));
                   }
               }
           </with>
        </div>
    }
}
