use std::{any::TypeId, iter::once};

use bevy::{
    ecs::component::ComponentId,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusedInput, InputFocus},
    prelude::*,
    reflect::{EnumInfo, StructInfo, TupleStructInfo, Type, TypeInfo},
};
use planetes_input::prelude::*;

use crate::{
    ReflectEditorView, ReflectPlanetesComponent, editor_ui::Capitalize, nodes::accordion,
    prelude::*,
};
pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            update_entity_viewer,
            update_component_editor,
            highlight_selected_input,
        )
            .chain(),
    );
}

pub fn view() -> impl Bundle {
    html! {
        <EntityViewer
            padding="8px"
            flex-grow="1"
            flex-shrink="1"
            display="flex"
            flex-direction="col"
            row-gap="8px"
            width="100%"
            height="100%"
        >
            <div padding="2px">
               "Entity Viewer"
            </div>
            <EntityEditor
                padding="2px"
                flex-grow="1"
                flex-shrink="1"
                display="flex"
                flex-direction="col"
                row-gap="8px"
            >
              "No Entity Selected"
            </EntityEditor>
        </EntityViewer>
    }
}

pub fn update_entity_viewer(
    mut commands: Commands,
    entity_viewer: Single<(Entity, &Viewing), (Changed<Viewing>, With<EntityEditor>)>,
    names: Query<&Name>,
    registry: Res<AppTypeRegistry>,
    world: &World,
    assets: Res<AssetServer>,
) {
    let (editor, &Viewing(target)) = *entity_viewer;

    let registry = registry.read();
    let allowed_types = registry
        .iter_with_data::<ReflectPlanetesComponent>()
        .map(|(type_reg, _)| type_reg.type_id())
        .collect::<Vec<_>>();

    let Some(components) = world.inspect_entity(target).ok() else {
        return;
    };

    let components = components
        .filter_map(|component| {
            component.type_id().and_then(|type_id| {
                if allowed_types.contains(&type_id)
                    && component.name() != "bevy_ecs::name::Name".into()
                {
                    Some((
                        component.id(),
                        format!("{}", component.name().shortname()),
                        type_id,
                    ))
                } else {
                    None
                }
            })
        })
        .map(|(id, name, type_id)| {
            accordion::view(
                name,
                SpawnIter(once(component_editor(id, type_id))),
                assets.clone(),
            )
        })
        .collect::<Vec<_>>();

    commands
        .entity(editor)
        .despawn_children()
        .with_children(move |parent| {
            parent.spawn(html! {
                <div
                  display="flex"
                  flex-direction="row"
                  align-items={AlignItems::Center}
                  oninput={|event: On<FocusedInput<KeyboardInput>>, inputs: Query<&InputField<String>>, mut names: Query<&mut Name>, target: Single<Entity, With<ViewedBy>>, focused: Res<InputFocus>| {
                    if event.input.logical_key != Key::Enter {
                        return;
                    }
                    info!("Typed in name input: {:?}", focused);
                    if let Some(focused) = focused.0.inspect(|_| {
                        info!("Focused Input Exists");
                    }) &&
                    let Ok(field) = inputs.get(focused).inspect_err(|_| {
                        info!("Failed to get field");
                    }) && let Ok(mut name) = names.get_mut(*target).inspect_err(|_| {
                        info!("Failed to get name");
                    }) {
                        info!("updated Name");
                        *name = Name::new(field.value.clone());
                    }
                  }}
                >
                    <span>"Selected: "</span>
                    {input_field::<String>(if let Ok(name) = names.get(target) {
                        format!("{name}")
                    } else {
                        format!("{target}")
                    })}
                </div>
            });

            parent.spawn(html! {
                <div
                   display="flex"
                   flex-direction="col"
                   flex-grow="1"
                   row-gap="4px">
                   <iter>
                    {components.into_iter()}
                   </iter>
                </div>
            });
        });
}

pub fn input_field<T: Validable>(value: T) -> impl Bundle {
    html! {
        <div
            border="1px"
            border-radius="4px"
            border-color={Color::linear_rgb(0.3, 0.3, 0.3)}
            padding-top="1px"
            padding-bottom="1px"
            padding-left="3px"
            padding-right="3px"
            >
            <div
                components={InputField::<T>::new(value)}
                min-height="14px"
                min-width="36px"
            />
        </div>
    }
}

pub fn highlight_selected_input(
    mut commands: Commands,
    inputs: Query<(Entity, &ChildOf), With<InputField<String>>>,
    focused: Res<InputFocus>,
) {
    if !focused.is_changed() {
        return;
    }
    for (input, child_of) in inputs.iter() {
        if let Some(focused_entity) = focused.0
            && focused_entity == input
        {
            commands
                .entity(child_of.0)
                .insert(BorderColor::from(Color::linear_rgb(0.7, 0.7, 0.7)));
        } else {
            commands
                .entity(child_of.0)
                .insert(BorderColor::from(Color::linear_rgb(0.3, 0.3, 0.3)));
        }
    }
}

#[derive(Component)]
pub struct ComponentEditor((ComponentId, TypeId));

pub fn component_editor(id: ComponentId, type_id: TypeId) -> impl Bundle {
    html! {
        <div
            padding-left="2px"
            flex-grow="0"
            flex-shrink="1"
            display="flex"
            flex-direction="col"
            row-gap="4px"
            width="100%"
            components={ComponentEditor((id, type_id))}>
            <span linebreak={LineBreak::WordBoundary}>
                {format!("{id:?}")}
            </span>
        </div>
    }
}

pub fn update_component_editor(
    mut commands: Commands,
    target: Single<Entity, With<ViewedBy>>,
    editors: Query<(Entity, &ComponentEditor), Changed<ComponentEditor>>,
    registry: Res<AppTypeRegistry>,
    world: &World,
) {
    let registry = registry.read();
    for (editor, &ComponentEditor((id, type_id))) in editors {
        let Some(registration) = registry.get(type_id) else {
            continue;
        };
        let Some(component_data) = registration.data::<ReflectComponent>() else {
            continue;
        };
        let Some(type_info) = registry.get_type_info(type_id) else {
            continue;
        };
        let Ok(entity) = world.get_entity(*target) else {
            continue;
        };
        let Some(reflected) = component_data.reflect(entity) else {
            continue;
        };
        let reflected_editor_view = registration
            .data::<ReflectEditorView>()
            .and_then(|editor_view| editor_view.get(reflected));

        let reflected = reflected.to_dynamic();

        commands
            .entity(editor)
            .despawn_children()
            .with_children(|parent| {
                parent.spawn((
                    Text::new(format!("{id:?}")),
                    TextLayout::new_with_linebreak(LineBreak::WordBoundary),
                ));
                if let Some(editor_view) = reflected_editor_view {
                    editor_view.add_to_parent(parent);
                } else {
                    parent.spawn(spawn_component_editor(type_info.clone(), reflected));
                }
            });
    }
}

fn spawn_component_editor(type_info: TypeInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
    html! {
        <div width="100%" onenter={move |event: On<FocusedInput<KeyboardInput>>, inputs: Query<&InputField<String>>, target: Query<&ComponentEditor>, ancestors: Query<&ChildOf>, focused: Res<InputFocus>| {
          if event.input.logical_key != Key::Enter || event.input.state != ButtonState::Pressed {
              return;
          }
          let Some(ComponentEditor((id, type_id))) = ancestors.iter_ancestors(event.focused_entity).find_map(|entity| target.get(entity).ok()) else {
              return;
          };
          info!("Applying Data to component {id:?} for type: {type_id:?}");
        }}>
            <with>
            {
                match type_info {
                    TypeInfo::Struct(info) => {
                        if info.field_len() != 0 {
                            parent.spawn(struct_editor(info, reflect))
                        } else {
                            parent.spawn(unit_struct())
                        }
                    },
                    TypeInfo::TupleStruct(info) => {
                        parent.spawn(tuple_struct_editor(info, reflect))
                    }
                    TypeInfo::Enum(info) => {
                        parent.spawn(enum_editor(info, reflect))
                    }
                    _ => parent.spawn(unknown_struct()),
                };
            }
            </with>
        </div>
    }
}

fn unit_struct() -> impl Bundle {
    html! {
        <span linebreak={LineBreak::WordBoundary}>"Unit Struct"</span>
    }
}

fn unknown_struct() -> impl Bundle {
    html! {
        <span linebreak={LineBreak::WordBoundary}>"Unknown Struct"</span>
    }
}

fn struct_editor(info: StructInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
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

fn tuple_struct_editor(info: TupleStructInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
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
                                          parent.spawn(Text::new(format!("{}:", info.type_path())));
                                           parent.spawn(Text::new(format!("{:?}", value)));
                                      }
                                      other => {
                                          parent.spawn(Text::new("Unknown Type"));
                                          parent.spawn(Text::new(format!("{:?}", other)));
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

fn enum_editor(info: EnumInfo, reflect: Box<dyn PartialReflect>) -> impl Bundle {
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
        .map(|field| format!("{:?}", field))
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
            html! {
                <div
                   display="flex"
                   flex-direction="row"
                   align-items={AlignItems::Center}
                   column-gap="4px">
                   <span>{field.name().capitalize_words().to_string()}</span>
                   {input_field::<String>(format!("{value:?}"))}
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

#[derive(Component)]
pub struct EntityViewer;

#[derive(Component)]
pub struct EntityEditor;

#[derive(Component)]
#[relationship(relationship_target = ViewedBy)]
pub struct Viewing(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = Viewing)]
pub struct ViewedBy(Entity);
