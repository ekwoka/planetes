use std::any::TypeId;

use bevy::{app::Propagate, prelude::*, reflect::TypeInfo};
use bevy_ui_html::html;
use planetes_scene_state::CanonicalScene;

use crate::nodes::entity_viewer::{UpdateEntityViewer, Viewing};

pub fn plugin(app: &mut App) {
    app.add_observer(handle_open_add_component)
        .add_observer(handle_close_add_component);
}
#[derive(Event)]
pub struct OpenAddComponent {
    pub entity: Entity,
}

#[derive(Event)]
pub struct CloseAddComponent;

#[derive(Component)]
pub struct AddComponentButton(pub TypeId);

#[derive(Component)]
pub struct AddComponentModal;

pub fn handle_open_add_component(
    _event: On<OpenAddComponent>,
    mut commands: Commands,
    registry: Res<AppTypeRegistry>,
) {
    let mut all_components = collect_default_components(&registry);
    all_components
        .sort_by_cached_key(|info| info.type_path_table().crate_name().unwrap_or("Unknown"));
    commands.spawn(html! {
        <AddComponentModal
          display="flex"
          flex-direction="column"
          justify-content="center"
          align-items={AlignItems::Center}
          position-type={PositionType::Absolute}
          top="0px"
          left="0px"
          right="0px"
          bottom="0px"
          font-size="12"
          components={ZIndex(1)}
        >
            <div
                display="flex"
                flex-direction="column"
                row-gap="4px"
                background-color={Color::BLACK}
                padding="8px"
                border-radius="4px"
                max-height="70vh">
                <div padding-bottom="8px">"Components Here"</div>
                <div
                  display="flex"
                  flex-direction="column"
                  row-gap="4px"
                  onScroll={handle_scroll}
                  overflow={Overflow {
                      y: OverflowAxis::Scroll,
                      ..Default::default()
                  }}
                  scrollbar-width="12px"
                  components={ScrollPosition(Vec2::new(0.0, 10.0))}>
                    <iter>
                        {all_components.into_iter().map(|info| {
                            html!{
                                <div
                                    display="flex"
                                    justify-content="space-between"
                                    column-gap="12px"
                                    onClick={handle_add_component}
                                    components={AddComponentButton(info.type_id())}>
                                    <span>{
                                        info.type_path_table().ident().unwrap_or("Unknown")
                                    }</span>
                                    <div components={Propagate(TextColor(Color::srgb_u8(178, 178, 178)))}>
                                        <span>{
                                            info.type_path_table().crate_name().unwrap_or("unknown")
                                        }</span>
                                    </div>
                                </div>
                            }
                        })}
                    </iter>
                </div>
            </div>
        </AddComponentModal>
    });
}

pub fn handle_scroll(
    event: On<Pointer<Scroll>>,
    mut scroll_position: Query<(&ComputedNode, &mut ScrollPosition)>,
) {
    if let Ok((layout, mut scroll_pos)) = scroll_position.get_mut(event.entity) {
        scroll_pos.0.y =
            layout.scroll_position.y * layout.inverse_scale_factor + event.event.y * 10.0;
    }
}

pub fn handle_close_add_component(
    _event: On<CloseAddComponent>,
    mut commands: Commands,
    modals: Query<Entity, With<AddComponentModal>>,
) {
    for modal in modals.iter() {
        commands.entity(modal).try_despawn();
    }
}

pub fn handle_add_component(
    event: On<Pointer<Click>>,
    mut commands: Commands,
    buttons: Query<&AddComponentButton>,
    editing: Single<&Viewing>,
    registry: Res<AppTypeRegistry>,
    canonical_scene: Res<CanonicalScene>,
    mut scenes: ResMut<Assets<DynamicScene>>,
) {
    if let Ok(button) = buttons.get(event.entity) {
        let entity = editing.0;
        info!("Adding component: {:?} to {:?}", button.0, entity);
        let registry = registry.read();
        let component_default = registry.get_type_data::<ReflectDefault>(button.0).unwrap();
        commands.trigger(CloseAddComponent);
        if let Some(entity) = canonical_scene.get_entity_mut(&mut scenes, entity) {
            entity.components.push(component_default.default());
        }
        commands.trigger(UpdateEntityViewer(entity));
    }
}

pub fn collect_default_components(registry: &Res<AppTypeRegistry>) -> Vec<TypeInfo> {
    let registry = registry.read();
    let all_components = registry
        .iter_with_data::<ReflectComponent>()
        .map(|(registration, _)| registration.type_info())
        .cloned()
        .collect::<Vec<TypeInfo>>();
    all_components
        .into_iter()
        .filter(|info| {
            registry
                .get_type_data::<ReflectDefault>(info.type_id())
                .is_some()
        })
        .collect()
}
