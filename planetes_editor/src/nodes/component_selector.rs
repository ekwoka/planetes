use bevy::{app::Propagate, prelude::*, reflect::TypeInfo};
use bevy_ui_html::html;

pub fn plugin(app: &mut App) {
    app.add_observer(handle_open_add_component);
}
#[derive(Event)]
pub struct OpenAddComponent {
    pub entity: Entity,
}

pub fn handle_open_add_component(
    _event: On<OpenAddComponent>,
    mut commands: Commands,
    registry: Res<AppTypeRegistry>,
) {
    let mut all_components = collect_default_components(&registry);
    all_components
        .sort_by_cached_key(|info| info.type_path_table().crate_name().unwrap_or("Unknown"));
    commands.spawn(html! {
        <div
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
                border-radius="4px">
                <div padding-bottom="8px">"Components Here"</div>
                <iter>
                    {all_components.into_iter().take(25).map(|info| {
                        html!{
                            <div display="flex" justify-content="space-between" column-gap="12px">
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
    });
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
