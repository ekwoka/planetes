use bevy::prelude::*;
pub use bevy_ui_html_macro::{HtmlComponent, html};

/// A struct grouping all UI properties parsed from the `html!` macro attributes.
///
/// `node` carries every layout/styling attribute (padding, margin, width, …).
/// The remaining fields hold the parsed value when the corresponding CSS-like
/// attribute is present, or the component's `Default` when it is absent.
///
/// Any attribute whose key is not recognised by a standard builder and whose
/// value is a plain string literal is passed separately as `additional_attributes`
/// to [`HtmlComponent::build`].
#[derive(Bundle)]
pub struct HtmlBundle {
    pub node: Node,
    pub background_color: BackgroundColor,
    pub border_color: BorderColor,
    pub text_font: TextFont,
    pub text_color: TextColor,
    pub text_layout: TextLayout,
}

/// Trait for types that can be used as custom tags in the `html!` macro.
///
/// The implementor is the tag expression itself — unit structs, enum unit
/// variants, and tuple-struct instances all work as `self`.
///
/// # Example
///
/// ```ignore
/// #[derive(Component)]
/// struct PrimaryButton { variant: &'static str }
///
/// impl HtmlComponent for PrimaryButton {
///     fn build(self, props: HtmlBundle, additional_attributes: &'static [(&'static str, &'static str)]) -> impl Bundle {
///         let variant = additional_attributes.iter()
///             .find(|(k, _)| *k == "variant")
///             .map(|(_, v)| *v)
///             .unwrap_or("default");
///         let HtmlBundle { node, background_color, .. } = props;
///         (PrimaryButton { variant }, node, background_color)
///     }
/// }
///
/// // html! { <{PrimaryButton::default()} variant="danger" padding="8px">"Click"</...> }
/// ```
pub trait HtmlComponent {
    fn build(
        self,
        props: HtmlBundle,
        additional_attributes: &'static [(&'static str, &'static str)],
    ) -> impl Bundle;
}

impl HtmlComponent for Button {
    fn build(self, props: HtmlBundle, _: &'static [(&'static str, &'static str)]) -> impl Bundle {
        (self, props)
    }
}

impl<F, B> HtmlComponent for F
where
    F: FnOnce(HtmlBundle, &'static [(&'static str, &'static str)]) -> B,
    B: Bundle,
{
    fn build(
        self,
        props: HtmlBundle,
        additional_attributes: &'static [(&'static str, &'static str)],
    ) -> impl Bundle {
        self(props, additional_attributes)
    }
}
