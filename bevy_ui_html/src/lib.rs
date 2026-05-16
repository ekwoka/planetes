use bevy::prelude::*;
pub use bevy_ui_html_macro::{HtmlComponent, html};

/// All parsed UI properties passed to [`HtmlComponent::build`].
///
/// `node` contains every layout/styling attribute recognised by the `html!`
/// macro (padding, margin, width, border-radius, …).  Any attribute whose key
/// is not recognised by a standard builder and whose value is a plain string
/// literal is collected into `extra_attrs` so the implementation can act on
/// bespoke, component-specific data.
pub struct HtmlProps {
    pub node: Node,
    /// Unknown string-literal attributes, in source order.
    pub extra_attrs: &'static [(&'static str, &'static str)],
}

/// Trait for types that can be used as custom tags in the `html!` macro.
///
/// # Example
///
/// ```ignore
/// #[derive(Component)]
/// struct PrimaryButton { variant: &'static str }
///
/// impl HtmlComponent for PrimaryButton {
///     type Bundle = (PrimaryButton, Node, BackgroundColor);
///
///     fn build(props: HtmlProps) -> Self::Bundle {
///         let variant = props.extra_attrs.iter()
///             .find(|(k, _)| *k == "variant")
///             .map(|(_, v)| *v)
///             .unwrap_or("default");
///         let color = if variant == "danger" { Color::RED } else { Color::BLUE };
///         (PrimaryButton { variant }, props.node, BackgroundColor(color))
///     }
/// }
///
/// // html! { <PrimaryButton variant="danger" padding="8px">"Click"</PrimaryButton> }
/// // expands to:
/// // (PrimaryButton::build(HtmlProps { node: Node { padding: px(8.0).all(), ... }, extra_attrs: &[("variant", "danger")] }), children)
/// ```
pub trait HtmlComponent {
    type Bundle: Bundle;
    fn build(props: HtmlProps) -> Self::Bundle;
}

impl HtmlComponent for Button {
    type Bundle = (Button, Node);
    fn build(props: HtmlProps) -> Self::Bundle {
        (Button, props.node)
    }
}
