//! # bevy_ui_html
//!
//! A procedural macro crate for writing Bevy UI using an HTML-like syntax.
//!
//! This crate provides the [`html!`] macro which transforms familiar HTML/JSX-like markup
//! into Bevy UI Bundles, making UI development more ergonomic for developers
//! coming from web backgrounds.
//!
//! ## Quick Start
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_ui_html::html;
//!
//! fn my_ui() -> impl Bundle {
//!     html! {
//!         <div padding="10px">
//!             "Hello, Bevy!"
//!         </div>
//!     }
//! }
//!
//! // expands to
//! fn my_ui() -> impl Bundle {
//!     (
//!         ::bevy::ui::Node {
//!             padding: ::bevy::ui::px(10.0).all(),
//!             ..Default::default()
//!         },
//!         <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn(
//!             (::bevy::ecs::spawn::Spawn(
//!                 ::bevy::ui::widget::Text::new("Hello, Bevy!"),
//!             )),
//!         ),
//!     )
//! }
//! ```
//!
//! > `html!` expands to fully qualified Bevy paths as in the above (eg. `::bevy::ui::Node`). For simplicity, the rest of the docs will show expansions using already in scope references (eg. `Node`) where relevant.
//!
//! ## Syntax
//!
//! The syntax is inspired by HTML and JSX (and parsed by `rstml` under the hood) to provide a relatively tried and true way to represent UI.
//! This should be quite familiar to those with web experience.
//!
//! This seemed a natural fit as already a lot of the API for Bevy's UI System is inspired by the web.
//!
//! ## Making Elements
//!
//! As with HTML, elements are defined with `<tagname>...</tagname>`.
//! They can be nested, or contain string literals, to produce `Children` relationships
//! and feature key-value pairs.
//!
//! While any valid `Ident` can be used as a tag name (see `Marker Components`), there are a few special cases.
//! ### `<div>` - Default `Node`
//!
//! This essentially is a "nothing" element. Generates a `bevy::ui::Node` as standard, along with any components indicated by the attributes.
//!
//! ```ignore
//! html! {
//!     <div>
//!         <div>"Child 1"</div>
//!         <div>"Child 2"</div>
//!     </div>
//! }
//! ```
//!
//! ### `<span>` - `Text` Element
//!
//! Renders directly as a `bevy::ui::widget::Text` component. Useful for inline text with optional styling.
//!
//! In most cases this would be equivalent to simple nesting a string literal inside a `<div>`.
//!
//! The main purpose of this is to allow for using inline Rust code to render the text. The contents of the block below are directly rendered into `Text::new(<HERE>)`
//!
//! ```ignore
//! html! {
//!     <div>
//!         <span>{
//!             format!("Hello, {}!", "World")
//!         }</span>
//!     </div>
//! }
//! ```
//!
//! Without the `<span>`, the return of the block needs to be a valid `Bundle` child.
//!
//! ### `<img>` - `ImageNode` Element
//!
//! Creates a `bevy::ui::widget::ImageNode` using the `src` attribute fed into `ImageNode::new(<SRC>)`.
//!
//! ```ignore
//! html! {
//!     <img src={asset_server.load("icon.png")} width="64px" height="64px" />
//! }
//! ```
//!
//! ### Custom Component Tags
//!
//! The [`HtmlComponent`] trait is provided to enable custom component tags. The simplest way using a simple marker struct is to use the provided derive.
//!
//! ```ignore
//! #[derive(Component, HtmlComponent)]
//! struct MenuButton;
//!
//! html! {
//!     <MenuButton padding="8px" border-radius="4px">
//!         "Click Me"
//!     </MenuButton>
//! }
//! ```
//!
//! [`HtmlComponent`] is blanket implemented for `FnOnce`, and the derive works simply on Unit Struct and Enum, though can also be derived for structs containing data, but with some quirks.
//!
//! ## Children
//!
//! The simplest way to represent the `Children/ChildOf` relationship, is nesting elements.
//!
//! ```ignore
//! html! {
//!     <div>
//!         <MenuButton padding="8px" border-radius="4px">
//!             <img src={asset_server.load("icon.png")} width="64px" height="64px" />
//!             <span>
//!                 {label.into()}
//!             </span>
//!         </MenuButton>
//!     </div>
//! }
//! ```
//!
//! These all expand to `Children::spawn((Spawn(child1), Spawn(child2), ...))`.
//!
//! However, this can have some issues. The first being how to generate children in a loop,
//! and the second being how to handle situations where the element tree could have dynamic children that would not resolve to the same opaque `Bundle`.
//!
//! Luckily, we can leverage other types that are supported by `Children::spawn`.
//!
//! ### `<iter>` - Spawning from an Iterator
//!
//! Wraps an iterator expression in `bevy::ecs::spawn::SpawnIter` for spawning multiple children.
//!
//! ```ignore
//! html! {
//!     <div>
//!         <iter>
//!             {
//!                 items.iter().map(|item| html! {
//!                     <div>{item.name.clone()}</div>
//!                 })
//!             }
//!         </iter>
//!     </div>
//! }
//! ```
//!
//! ### `<with>` - Imperative Spawning
//!
//! Wraps a block in `bevy::ecs::spawn::SpawnWith` for imperative child spawning.
//! The block receives a `parent: &mut RelatedSpawner<ChildOf>` parameter.
//!
//! ```ignore
//! html! {
//!     <div>
//!         <with>
//!             {
//!                 if show_content {
//!                     parent.spawn(html! { <div>"Visible"</div> });
//!                 }
//!             }
//!         </with>
//!     </div>
//! }
//! ```
//!
//! With these, just about any complex and dynamic UI structure could be represented.
//!
//! ## Custom Elements
//!
//! Currently, there are no custom elements directly supported with special syntax. Instead you can simply call a normal rust function as a child of an element.
//!
//! ```ignore
//! fn button(label: Into<String>) -> impl Bundle {
//!     Text::new(label.into())
//! }
//!
//! html! {
//!     <div>
//!         {button("Click me!")}
//!     </div>
//! }
//! ```
//!
//! ## Attributes
//!
//! One of the major benefits of using `html!` over writing out your own UI Bundles, is not needing to worry about which specific UI components to use for different functionality (`TextFont` vs `TextColor` vs `BorderRadius` etc).
//!
//! This is because you can put all the attributes directly on a single element, and the compiler splits them out into the appropriate components.
//!
//! ### Attribute Syntax
//!
//! Attributes are expressed with a kebab-case key and a value. For instance, if the `Node` property is `flex_direction`, it should be expressed as `flex-direction`.
//!
//! The value can be any arbitrary Rust block that returns the appropriate type for that property.
//!
//! ```ignore
//! html!{
//!     <div
//!         display={Display::Flex}
//!         flex-direction={FlexDirection::Row}>
//!     </div>
//! }
//! ```
//!
//! Luckily, many attributes also support string representations and other shorthands, allowing a more `HTML`-like experience. The above could also be represented as
//!
//! ```ignore
//! html!{
//!     <div
//!         display="flex"
//!         flex-direction="row">
//!     </div>
//! }
//! ```
//!
//! Below are all the supported attributes on an element, what they do, which component/property they map to, and what types they accept:
//!
//!
//! ### Layout Attributes
//!
//! | Attribute | Description | Component.property | Type | Notes
//! |-----------|-------------|--------------------|------|------
//! | `width` | Element Width | Node.width | `Val` |
//! | `height` | Element Height | Node.height | `Val` |
//! | `min-width` | Minimum Width | Node.min_width | `Val` |
//! | `max-width` | Maximum Width | Node.max_width | `Val` |
//! | `min-height` | Minimum Height | Node.min_height | `Val` |
//! | `max-height` | Maximum Height | Node.max_height | `Val` |
//! | `aspect-ratio` | Aspect ratio constraint (f32) |
//! | `padding` | Padding on all sides | Node.padding | `Val` | All sides
//! | `padding-[top\|right\|bottom\|left]` | Individual padding | Node.padding | `Val` | Individual sides
//! | `margin` | Margin on all sides | Node.margin | `Val` | All sides
//! | `margin-[top\|right\|bottom\|left]` | Individual sides | Node.margin | `Val` | Individual sides
//! | `border` | Border Width | Node.border | `Val` | All sides
//! | `border-[top\|right\|bottom\|left]` | Border Width | Node.border | `Val` | Individual sides
//! | `top`, `right`, `bottom`, `left` | Positioning Offset | Node.[top\|right\|bottom\|left] | `Val` |
//! | `position-type` | How to position element in layout | Node.position_type | `PositionType` |
//!
//! Directional attributes chain together:
//! ```text
//! padding="10px" padding-top="20px"
//! // becomes:
//! padding: px(10.0).all().with_top(px(20.0))
//! ```
//!
//! ### Entity Attributes
//!
//! | Attribute | Component | Description |
//! |-----------|-----------|-------------|
//! | `name` | `Name` | Entity debug name |
//! | `components` | — | Additional component tuple |
//!
//! The `components` attribute allows injecting arbitrary components:
//! ```ignore
//! html! {
//!     <div components={(Focusable, TabIndex(0))}>
//!         "Interactive"
//!     </div>
//! }
//! ```
//!
//! ### Event Observers
//!
//! Attributes starting with `on` attach `bevy::ecs::observer::Observer` components.
//!
//! ```ignore
//! html! {
//!     <div onClick={|_: On<Pointer<Click>>, mut commands: Commands| {
//!         // Handle click
//!     }}>
//!         "Clickable"
//!     </div>
//! }
//! ```
//!
//! ## Value Syntax
//!
//! ### CSS-like String Values
//!
//! String literals support common CSS units:
//!
//! | Syntax | Bevy Equivalent |
//! |--------|-----------------|
//! | `"10px"` | `px(10.0)` |
//! | `"50%"` | `percent(50.0)` |
//! | `"100vw"` | `vw(100.0)` |
//! | `"50vh"` | `vh(50.0)` |
//! | `"10vmin"` | `vmin(10.0)` |
//! | `"10vmax"` | `vmax(10.0)` |
//!
//! ### Rust Expressions
//!
//! Use braces for Rust expressions:
//! ```ignore
//! html! {
//!     <div width={Val::Percent(50.0)} padding={px(10.0)}>
//!         {format!("Count: {}", count)}
//!     </div>
//! }
//! ```
//!
//! ### Color Values
//!
//! String literals support multiple color formats:
//!
//! ```ignore
//! // Named colors
//! background-color="black"
//! background-color="white"
//!
//! // RGB (0-255 values, converted to linear)
//! background-color="rgb(255 128 0)"
//! background-color="rgba(255 128 0 / 0.5)"
//!
//! // sRGB
//! background-color="srgb(255 128 0)"
//! background-color="srgba(255 128 0 / 0.5)"
//!
//! // Other colorspaces: hsl, hsv, hwb, lab, lch, oklab, oklch, xyz
//! background-color="hsl(180 50 50)"
//!
//! // Rust expressions
//! background-color={Color::linear_rgb(1.0, 0.5, 0.0)}
//! ```
//!
//! ## Feature Flags
//!
//! ### `propagate`
//!
//! When enabled, `TextFont` components are wrapped in `Propagate<TextFont>` for
//! automatic inheritance to child text nodes. Useful with the `bevy_propagate` crate.
//!
//! ```toml
//! [dependencies]
//! bevy_ui_html = { version = "0.0.1", features = ["propagate"] }
//! ```
//!
//! ## Generated Output
//!
//! The macro generates component tuples compatible with Bevy's spawn system:
//!
//! ```ignore
//! // This:
//! html! {
//!     <div padding="10px" background-color="black">
//!         "Hello"
//!     </div>
//! }
//!
//! // Expands to:
//! (
//!     Node {
//!         padding: px(10.0).all(),
//!         ..Default::default()
//!     },
//!     BackgroundColor(Color::BLACK),
//!     Children::spawn((
//!         Spawn(Text::new("Hello"))
//!     ))
//! )
//! ```
//!
//! Children are wrapped in `Spawn`, iterators in `SpawnIter`, and imperative
//! blocks in `SpawnWith`, leveraging Bevy's spawn-related traits.

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

/// Type Alias for the unclaimed HtmlAttributes to simplify trait
pub type HtmlAttributes = &'static [(&'static str, &'static str)];

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
///     fn build(self, props: HtmlBundle, additional_attributes: HtmlAttributes) -> impl Bundle {
///         let variant = additional_attributes.iter()
///             .find(|(k, _)| *k == "variant")
///             .map(|(_, v)| *v)
///             .unwrap_or("default");
///         let HtmlBundle { node, background_color, .. } = props;
///         (PrimaryButton { variant }, node, background_color)
///     }
/// }
///
/// // html! {
/// //   <{PrimaryButton::default()} variant="danger" padding="8px">"Click"</{PrimaryButton::default()}>
/// // }
/// ```
///
/// # `FnOnce` Components
/// `HtmlComponent` is blanket implemented for `FnOnce` if they are of the same signature as the `build` method (minus the `self` of course)
/// ```ignore
/// fn header(props: HtmlBundle, props: HtmlAttributes) -> impl Bundle {
///   let content = additional_attributes.iter()
///             .find(|(k, _)| *k == "content")
///             .map(|(_, v)| *v)
///             .unwrap_or("");
///   (props, Text::new(content.into()))
/// }
///
/// fn render_ui() -> impl Bundle {
///    html! {
///       <header content="Heading 1" />
///    }
/// }
/// ```
/// This also means closures can be used as well.
///
/// # Enum and Unit Structs
///
/// enum and unit structs are most simply implemented with the provided `derive`
///
/// ```ignore
///
/// #[derive(Component, HtmlComponent)]
/// struct Clickable;
///
/// #[derive(Component, HtmlComponent)]
/// enum Button {
///    Primary,
///    Secondary
/// }
///
/// fn render_ui() -> impl Bundle {
///    html! {
///       <div>
///          <Clickable>"Hello"</Clickable>
///          <Button::Primary>"Do Something"</Button::Primary>
///       </div>
///    }
/// }
/// ```
///
/// It is of note, that the closing tag must be the exact same tokens as the opening tag, so the above cannot be closed by simple `</Button>`. This is most important if your struct contains data. It can be beneficial to create the struct instance before returning the html to simplify the layout.
pub trait HtmlComponent {
    fn build(self, props: HtmlBundle, additional_attributes: HtmlAttributes) -> impl Bundle;
}

impl HtmlComponent for Button {
    fn build(self, props: HtmlBundle, _: HtmlAttributes) -> impl Bundle {
        (self, props)
    }
}

impl<F, B> HtmlComponent for F
where
    F: FnOnce(HtmlBundle, HtmlAttributes) -> B,
    B: Bundle,
{
    fn build(self, props: HtmlBundle, additional_attributes: HtmlAttributes) -> impl Bundle {
        self(props, additional_attributes)
    }
}
