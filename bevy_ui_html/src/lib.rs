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
//! Any other tag name is treated as a custom component and included in the output tuple as another component.
//! This enables marker components for queries and styling hooks.
//!
//! ```ignore
//! #[derive(Component)]
//! struct MenuButton;
//!
//! html! {
//!     <MenuButton padding="8px" border-radius="4px">
//!         "Click Me"
//!     </MenuButton>
//! }
//! ```
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

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use rstml::{
    node::{Node, NodeName},
    parse2,
};
use syn::spanned::Spanned;

mod color;
mod components;
mod value;

use components::*;
use value::*;

#[derive(Clone, Debug)]
enum HtmlNode {
    Text(TextNode),
    Element(ElementNode),
    Inline(InlineNode),
    Block(BlockNode),
    Iter(IterNode),
    With(WithNode),
}

#[derive(Clone, Debug)]
struct TextNode {
    value: String,
}

#[derive(Clone, Debug)]
struct Attribute {
    key: String,
    value: syn::Expr,
    span: proc_macro2::Span,
}

#[derive(Clone, Debug)]
struct ChildNode(HtmlNode);

#[derive(Clone, Debug)]
struct ElementNode {
    tag_name: NodeName,
    children: Vec<ChildNode>,
    attributes: Vec<Attribute>,
}

#[derive(Clone, Debug)]
struct InlineNode {
    children: Vec<HtmlNode>,
    attributes: Vec<Attribute>,
}

#[derive(Clone, Debug)]
struct BlockNode {
    block: rstml::node::NodeBlock,
}

#[derive(Clone, Debug)]
struct IterNode {
    block: Box<HtmlNode>,
}

#[derive(Clone, Debug)]
struct WithNode {
    block: Box<HtmlNode>,
}

impl From<Node> for HtmlNode {
    fn from(node: Node) -> Self {
        match node {
            Node::Text(text) => Self::Text(TextNode {
                value: text.value_string(),
            }),
            Node::Element(element) => {
                let tag_name = element.open_tag.name.to_string();

                // Handle special transparent iter tag
                if tag_name == "iter" {
                    // For iter tags, extract children and create an Iter node
                    let block = element
                        .children
                        .into_iter()
                        .filter_map(|child| match child {
                            Node::Block(_) => Some(Self::from(child)),
                            _ => None,
                        })
                        .next()
                        .expect("iter tag must have exactly one Block child");
                    return Self::Iter(IterNode {
                        block: Box::new(block),
                    });
                }

                if tag_name == "with" {
                    let block = element
                        .children
                        .into_iter()
                        .filter_map(|child| match child {
                            Node::Block(_) => Some(Self::from(child)),
                            _ => None,
                        })
                        .next()
                        .expect("iter tag must have exactly one Block child");
                    return Self::With(WithNode {
                        block: Box::new(block),
                    });
                }

                let children = element
                    .children
                    .into_iter()
                    .filter_map(|child| match child {
                        Node::Text(_) | Node::Element(_) | Node::Block(_) => {
                            Some(Self::from(child))
                        }
                        _ => None,
                    });

                let attributes: Vec<Attribute> = element
                    .open_tag
                    .attributes
                    .into_iter()
                    .filter_map(|attr| {
                        if let rstml::node::NodeAttribute::Attribute(attr) = attr {
                            let key = attr.key.to_string();
                            if let Some(value_expr) = attr.value() {
                                return Some(Attribute {
                                    key,
                                    value: value_expr.clone(),
                                    span: match attr.key {
                                        NodeName::Path(path) => path.span(),
                                        NodeName::Punctuated(path) => path.span(),
                                        NodeName::Block(block) => block.span(),
                                    }
                                    .span(),
                                });
                            }
                            None
                        } else {
                            None
                        }
                    })
                    .collect();
                if tag_name == "span" {
                    Self::Inline(InlineNode {
                        children: children.collect(),
                        attributes,
                    })
                } else {
                    Self::Element(ElementNode {
                        tag_name: element.open_tag.name,
                        children: children.map(ChildNode).collect(),
                        attributes,
                    })
                }
            }
            Node::Block(block) => Self::Block(BlockNode { block }),
            _ => todo!("Unsupported node type"),
        }
    }
}

impl ToTokens for ChildNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let node = self.0.clone();
        match node {
            HtmlNode::Iter(node) => {
                tokens.extend(quote! {
                    ::bevy::ecs::spawn::SpawnIter(#node)
                });
            }
            HtmlNode::With(node) => tokens.extend(quote! {
                ::bevy::ecs::spawn::SpawnWith(#node)
            }),
            _ => {
                tokens.extend(quote! {
                    ::bevy::ecs::spawn::Spawn(#node)
                });
            }
        }
    }
}

impl ToTokens for TextNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let value = &self.value;
        tokens.extend(quote! {
            ::bevy::ui::widget::Text::new(#value)
        });
    }
}

impl ToTokens for BlockNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let rstml::node::NodeBlock::ValidBlock(block) = &self.block
            && block.stmts.len() == 1
        {
            block.stmts[0].to_tokens(tokens)
        } else {
            // Output the block directly - it can contain any Rust expression
            // including nested macro calls, component instantiation, etc.
            self.block.to_tokens(tokens);
        }
    }
}

impl ToTokens for IterNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let block = &self.block;
        tokens.extend(quote! {
            #block
        });
    }
}

impl ToTokens for WithNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let block = &self.block;
        tokens.extend(quote! {
            move |parent: &mut ::bevy::ecs::relationship::RelatedSpawner<::bevy::ecs::hierarchy::ChildOf>| {
                #block
            }
        });
    }
}

impl ElementNode {
    fn get_attr<'a>(attributes: &'a [Attribute], kebab_name: &str) -> Option<&'a syn::Expr> {
        attributes
            .iter()
            .find(|attr| attr.key == kebab_name)
            .map(|attr| &attr.value)
    }
}

impl ToTokens for ElementNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut components = Vec::<TokenStream>::new();
        components.push_some(Name::from(&self.attributes).ok());
        if self.tag_name.to_string() != "div" && self.tag_name.to_string() != "img" {
            let tag_name = &self.tag_name;
            components.push(quote! {
                #tag_name
            })
        }
        components.push_some(NodeComponent::from(&self.attributes).ok());
        components.push_some(Image::from(&self.attributes).ok());
        components.push_some(BorderRadius::from(&self.attributes).ok());
        components.push_some(BorderColor::from(&self.attributes).ok());
        components.push_some(BackgroundColor::from(&self.attributes).ok());
        components.push_some(TextFont::from(&self.attributes).ok());
        components.push_some(TextLayout::from(&self.attributes).ok());
        components.push_some(
            Self::get_attr(&self.attributes, "components")
                .and_then(|value| Value::new(value).clean_block()),
        );
        let mut children = self
            .children
            .iter()
            .map(|child| child.to_token_stream())
            .collect::<Vec<_>>();
        children.push_some(Observer::from(&self.attributes).ok());
        if !children.is_empty() {
            components.push(quote! {
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    #(#children),*
                ))
            });
        }
        match components.len() {
            1 => {
                tokens.extend(quote! {
                    #(#components)*
                });
            }
            _ => {
                tokens.extend(quote! {
                    (
                        #(#components),*
                    )
                });
            }
        }
    }
}

impl ToTokens for InlineNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut components = Vec::<TokenStream>::new();
        components.extend(
            self.children
                .iter()
                .map(|child| match child {
                    HtmlNode::Block(block) => quote! {
                        ::bevy::ui::widget::Text::new(#block)
                    },
                    _ => quote! {
                        #child
                    },
                })
                .collect::<Vec<_>>(),
        );
        components.push_some(TextFont::from(&self.attributes).ok());
        components.push_some(TextLayout::from(&self.attributes).ok());
        if components.len() == 1 {
            tokens.extend(quote! {
                #(#components),*
            });
        } else {
            tokens.extend(quote! {
                (#(#components),*)
            });
        }
    }
}

impl ToTokens for HtmlNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            HtmlNode::Text(text) => text.to_tokens(tokens),
            HtmlNode::Element(element) => element.to_tokens(tokens),
            HtmlNode::Inline(inline) => inline.to_tokens(tokens),
            HtmlNode::Block(block) => block.to_tokens(tokens),
            HtmlNode::Iter(iter) => iter.to_tokens(tokens),
            HtmlNode::With(with) => with.to_tokens(tokens),
        }
    }
}

trait PushSomeTokens {
    fn push_some(&mut self, item: Option<impl ToTokens>);
}

impl PushSomeTokens for Vec<TokenStream> {
    fn push_some(&mut self, item: Option<impl ToTokens>) {
        if let Some(item) = item {
            self.push(item.to_token_stream());
        }
    }
}

fn html_inner(input: TokenStream) -> TokenStream {
    let node_tree = parse2(input).unwrap();
    let mut output = TokenStream::new();

    for node in node_tree.into_iter() {
        if let Node::Element(_) = node {
            let html_node = HtmlNode::from(node);
            html_node.to_tokens(&mut output);
        }
    }

    output
}

/// Transforms HTML-like markup into Bevy UI component tuples.
///
/// This procedural macro parses an HTML/JSX-like syntax and generates component tuples
/// that can be spawned directly with Bevy's `commands.spawn()`.
///
/// # Basic Usage
///
/// ```ignore
/// use bevy::prelude::*;
/// use bevy_ui_html::html;
///
/// fn setup(mut commands: Commands) {
///     commands.spawn(html! {
///         <div padding="10px" background-color="black">
///             "Hello, World!"
///         </div>
///     });
/// }
/// ```
///
/// # Supported Elements
///
/// - **`<div>`**: Container element, generates `bevy::ui::Node` with layout attributes
/// - **`<span>`**: Text element, generates `bevy::ui::widget::Text`
/// - **`<img>`**: Image element with `src` attribute, generates `bevy::ui::widget::ImageNode`
/// - **`<iter>`**: Wraps an iterator in `bevy::ecs::spawn::SpawnIter`
/// - **`<with>`**: Wraps a closure in `bevy::ecs::spawn::SpawnWith`
/// - **PascalCase tags**: Treated as custom components (e.g., `<MenuButton>`)
///
/// # Attribute Syntax
///
/// Attributes accept either CSS-like string values or Rust expressions in braces:
///
/// ```ignore
/// html! {
///     <div
///         padding="10px"           // CSS-like string
///         width={Val::Percent(50.0)} // Rust expression
///         background-color="rgb(255 128 0)"
///     >
///         "Content"
///     </div>
/// }
/// ```
///
/// # CSS Unit Support
///
/// String values support common CSS units:
/// - `"10px"` → `px(10.0)`
/// - `"50%"` → `percent(50.0)`
/// - `"100vw"` / `"50vh"` → `vw(100.0)` / `vh(50.0)`
/// - `"10vmin"` / `"10vmax"` → `vmin(10.0)` / `vmax(10.0)`
///
/// # Color Formats
///
/// Color attributes support multiple formats:
/// - Named: `"black"`, `"white"`, `"none"`
/// - RGB: `"rgb(255 128 0)"`, `"rgba(255 128 0 / 0.5)"`
/// - sRGB: `"srgb(255 128 0)"`, `"srgba(255 128 0 / 0.5)"`
/// - Other: `"hsl(180 50 50)"`, `"hsv(...)"`', `"oklab(...)"`
///
/// # Event Observers
///
/// Attributes starting with `on` attach `bevy::ecs::observer::Observer` components:
///
/// ```ignore
/// html! {
///     <div onClick={|_: On<Pointer<Click>>| { /* handle click */ }}>
///         "Click me"
///     </div>
/// }
/// ```
///
/// # Generated Output
///
/// The macro generates component tuples compatible with Bevy's spawn system:
///
/// ```ignore
/// // html! { <div padding="10px">"Hello"</div> }
/// // expands to:
/// (
///     Node { padding: px(10.0).all(), ..Default::default() },
///     Children::spawn((Spawn(Text::new("Hello"))))
/// )
/// ```
///
/// See the [crate-level documentation](crate) for complete attribute reference.
#[proc_macro]
pub fn html(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    html_inner(input.into()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_div_with_text() {
        let input = quote! {
            <div>"Hello"</div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn single_div_with_no_children() {
        let input = quote! {
            <div/>
        };
        let expected = quote! {
            ::bevy::ui::Node::default()
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn div_with_div_children() {
        let input = quote! {
            <div>
                <div>"Hello"</div>
                <div>"World"</div>
            </div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(
                        (
                            ::bevy::ui::Node::default(),
                            <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                                ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello"))
                            ))
                        )
                    ),
                    ::bevy::ecs::spawn::Spawn(
                        (
                            ::bevy::ui::Node::default(),
                            <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                                ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("World"))
                            ))
                        )
                    )
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn div_with_span_children() {
        let input = quote! {
            <div>
                <span>"Hello"</span>
                <span>"World"</span>
            </div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello")),
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("World"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn single_div_with_attributes() {
        let input = quote! {
            <div
              padding="10px"
              padding-bottom="20%"
              margin-top="5vw"
              margin-left="10vh"
              margin-bottom="15vmin"
              margin-right="20vmax"
              >
              "Hello"
            </div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node {
                  padding: ::bevy::ui::px(10.0).all().with_bottom(::bevy::ui::percent(20.0)),
                  margin: ::bevy::ui::vw(5.0).top().with_right(::bevy::ui::vmax(20.0)).with_bottom(::bevy::ui::vmin(15.0)).with_left(::bevy::ui::vh(10.0)),
                  ..Default::default()
                },
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn single_div_with_rust_attributes() {
        let input = quote! {
            <div
              padding={px(10.0)}
              padding-bottom={percent(20.0)}
              margin-top={vw(5.0)}
              margin-left={vh(10.0)}
              margin-bottom={vmin(15.0)}
              margin-right={vmax(20.0)}
              >
              "Hello"
            </div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node {
                  padding: px(10.0).all().with_bottom(percent(20.0)),
                  margin: vw(5.0).top().with_right(vmax(20.0)).with_bottom(vmin(15.0)).with_left(vh(10.0)),
                  ..Default::default()
                },
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn div_with_sizing_and_positioning() {
        let input = quote! {
            <div
              width="100px"
              height="50px"
              min-width="10px"
              max-width="200px"
              left="5px"
              top="10px"
              >
              "Test"
            </div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node {
                  left: ::bevy::ui::px(5.0),
                  top: ::bevy::ui::px(10.0),
                  width: ::bevy::ui::px(100.0),
                  height: ::bevy::ui::px(50.0),
                  min_width: ::bevy::ui::px(10.0),
                  max_width: ::bevy::ui::px(200.0),
                  ..Default::default()
                },
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Test"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn div_with_flexbox_attributes() {
        let input = quote! {
            <div
              display={Display::Flex}
              flex-direction={FlexDirection::Column}
              flex-grow={1.0}
              flex-shrink={0.5}
              align-items={AlignItems::Center}
              justify-content={JustifyContent::SpaceBetween}
              >
              "Flex"
            </div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node {
                  display: Display::Flex,
                  flex_direction: FlexDirection::Column,
                  align_items: AlignItems::Center,
                  justify_content: JustifyContent::SpaceBetween,
                  flex_grow: 1.0,
                  flex_shrink: 0.5,
                  ..Default::default()
                },
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Flex"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn div_with_border_and_gaps() {
        let input = quote! {
            <div
              border="2px"
              border-top="5px"
              row-gap="10px"
              column-gap="15px"
              >
              "Borders"
            </div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node {
                  border: ::bevy::ui::px(2.0).all().with_top(::bevy::ui::px(5.0)),
                  row_gap: ::bevy::ui::px(10.0),
                  column_gap: ::bevy::ui::px(15.0),
                  ..Default::default()
                },
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Borders"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn div_with_aspect_ratio_and_position_type() {
        let input = quote! {
            <div
              aspect-ratio="1.77"
              position-type={PositionType::Absolute}
              width="100%"
              >
              "Aspect"
            </div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node {
                  width: ::bevy::ui::percent(100.0),
                  position_type: PositionType::Absolute,
                  aspect_ratio: Some(1.77),
                  ..Default::default()
                },
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Aspect"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn single_div_with_block_content() {
        let input = quote! {
            <div>{Text::new("Hello")}</div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(Text::new("Hello"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn div_with_complex_block_expression() {
        let input = quote! {
            <div>{if show { Text::new("Visible") } else { Text::new("Hidden") }}</div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(if show { Text::new("Visible") } else { Text::new("Hidden") })
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn div_with_nested_macro_call() {
        // This demonstrates that blocks can contain macro calls (like nested html! calls)
        let inner = html_inner(quote! { <span>"Nested"</span> });
        let input = quote! {
            <div>{#inner}</div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Nested"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn div_with_mixed_text_and_blocks() {
        let input = quote! {
            <div>
                "Static text"
                {dynamic_content}
                {MyComponent::new()}
            </div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Static text")),
                    ::bevy::ecs::spawn::Spawn(dynamic_content),
                    ::bevy::ecs::spawn::Spawn(MyComponent::new())
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn supports_iter_element() {
        let input = quote! {
            <div>
               <iter>
                   {
                        items.map(|item| {
                            html! {
                                <div>{item.name}</div>
                            }
                        })
                    }
               </iter>
            </div>
        };
        let output = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::SpawnIter(
                        items.map(|item| {
                            html! {
                                <div>{item.name}</div>
                            }
                        })
                    )
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }

    #[test]
    fn supports_border_radius() {
        let input = quote! {
            <div
               padding="4px"
               border-radius="2px">
               "Menu"
            </div>
        };
        let expected = quote! {
            (
                ::bevy::ui::Node {
                    padding: ::bevy::ui::px(4.0).all(),
                    ..Default::default()
                },
                ::bevy::ui::BorderRadius::all(::bevy::ui::px(2.0)),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                ))
            )
        };

        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn supports_border_color() {
        let input = quote! {
            <div
               padding="4px"
               border-color={Color::linear_rgb(0.7, 0.7, 0.7)}>
               "Menu"
            </div>
        };
        let expected = quote! {
            (
                ::bevy::ui::Node {
                    padding: ::bevy::ui::px(4.0).all(),
                    ..Default::default()
                },
                ::bevy::ui::BorderColor::all(Color::linear_rgb(0.7, 0.7, 0.7)),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                ))
            )
        };

        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn supports_background_color() {
        let input = quote! {
            <div
               padding="4px"
               background-color={Color::linear_rgb(0.7, 0.7, 0.7)}>
               "Menu"
            </div>
        };
        let expected = quote! {
            (
                ::bevy::ui::Node {
                    padding: ::bevy::ui::px(4.0).all(),
                    ..Default::default()
                },
                ::bevy::ui::BackgroundColor(Color::linear_rgb(0.7, 0.7, 0.7)),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                ))
            )
        };

        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn supports_unit_struct_elements() {
        let input = quote! {
            <MenuButton
               padding="4px"
               border-radius="2px">
               "Menu"
            </MenuButton>
        };
        let expected = quote! {
            (
                MenuButton,
                ::bevy::ui::Node {
                    padding: ::bevy::ui::px(4.0).all(),
                    ..Default::default()
                },
                ::bevy::ui::BorderRadius::all(::bevy::ui::px(2.0)),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                ))
            )
        };

        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn only_removes_bracket_on_single_statement_block() {
        let input = quote! {
            <div>
               <div>
               {Text::new("Menu")}
               </div>
               <div>
               {
                   let thing = Text::new("Thing");
                   thing
               }
               </div>
            </div>
        };
        let expected = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn((
                        ::bevy::ui::Node::default(),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(Text::new("Menu"))
                        ))
                    )),
                    ::bevy::ecs::spawn::Spawn((
                        ::bevy::ui::Node::default(),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn({
                                let thing = Text::new("Thing");
                                thing
                            })
                        ))
                    ))
                ))
            )
        };

        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn auto_wrap_inline_block_children() {
        let input = quote! {
            <div>
                <span>{"Hello"}</span>
                <span>{let thing = true; if thing { "World" } else { "Mom" }}</span>
            </div>
        };
        let expected = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello")),
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new({let thing = true; if thing { "World" } else { "Mom" }}))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn supports_spawning_with_children() {
        let input = quote! {
            <div>
                <with>
                    {
                        if true {
                            parent.spawn(html! { <div>"Hello World"</div>});
                        } else {
                            parent.spawn(html! { <div>"Hello Mom"</div>});
                        }
                    }
                </with>
            </div>
        };
        let expected = quote! {
            (
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::SpawnWith(move |parent: &mut ::bevy::ecs::relationship::RelatedSpawner<::bevy::ecs::hierarchy::ChildOf>| {
                        if true {
                            parent.spawn(html! { <div>"Hello World"</div>});
                        } else {
                            parent.spawn(html! { <div>"Hello Mom"</div>});
                        }
                    })
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn support_arbitrary_component_additions() {
        let input = quote! {
            <div
            components={(
                Checkable,
                Checked
            )}>"Hello"</div>
        };
        let expected = quote! {
            (
                ::bevy::ui::Node::default(),
                (Checkable, Checked),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn supports_name() {
        let input = quote! {
            <div name="hello">"World"</div>
        };
        let expected = quote! {
            (
                ::bevy::ecs::name::Name::new("hello"),
                ::bevy::ui::Node::default(),
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("World"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn supports_img_tags() {
        let input = quote! {
            <img src={asset_server
                .load("embedded://planetes_editor/assets/filled_triangle.png")} />
        };
        let expected = quote! {
            (
                ::bevy::ui::Node::default(),
                ::bevy::ui::widget::ImageNode::new(
                    asset_server
                        .load("embedded://planetes_editor/assets/filled_triangle.png")
                )
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }

    mod colors {
        use super::*;

        #[test]
        fn allows_direct_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color={Color::linear_rgb(0.7, 0.7, 0.7)}>
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(Color::linear_rgb(0.7, 0.7, 0.7)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        mod const_colors {
            use super::*;
            #[test]
            fn allows_const_black() {
                let input = quote! {
                    <div
                    padding="4px"
                    border-color="black">
                    "Menu"
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all(),
                            ..Default::default()
                        },
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::BLACK),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                        ))
                    )
                };

                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn allows_const_white() {
                let input = quote! {
                    <div
                    padding="4px"
                    border-color="white">
                    "Menu"
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all(),
                            ..Default::default()
                        },
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::WHITE),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                        ))
                    )
                };

                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn allows_const_none() {
                let input = quote! {
                    <div
                    padding="4px"
                    border-color="none">
                    "Menu"
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all(),
                            ..Default::default()
                        },
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::NONE),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                        ))
                    )
                };

                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }
        }

        #[test]
        fn allows_string_rgb_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="rgb(170 170 170)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::linear_rgb(0.6666667, 0.6666667, 0.6666667)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_rgba_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="rgba(170 170 170 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::linear_rgba(0.6666667, 0.6666667, 0.6666667, 0.5)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_srgb_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="srgb(178.5 178.5 178.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::srgb(0.7, 0.7, 0.7)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_srgbu8_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="srgb(170 170 170)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::srgb_u8(170, 170, 170)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_srgba_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="srgba(178.5 178.5 178.5 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::srgba(0.7, 0.7, 0.7, 0.5)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_srgbau8_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="srgba(170 170 170 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::srgba_u8(170, 170, 170, 127)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_hsl_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="hsl(170 0.7 0.7)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::hsl(170.0, 0.7, 0.7)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_hsla_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="hsla(170 0.7 0.7 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::hsla(170.0, 0.7, 0.7, 0.5)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_hsv_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="hsv(170 0.7 0.7)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::hsv(170.0, 0.7, 0.7)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_hsva_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="hsva(170 0.7 0.7 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::hsva(170.0, 0.7, 0.7, 0.5)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_hwb_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="hwb(170 0.7 0.7)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::hwb(170.0, 0.7, 0.7)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_hwba_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="hwba(170 0.7 0.7 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::hwba(170.0, 0.7, 0.7, 0.5)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_lab_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="lab(170 0.7 0.7)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::lab(170.0, 0.7, 0.7)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_laba_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="laba(170 0.7 0.7 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::laba(170.0, 0.7, 0.7, 0.5)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_lch_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="lch(170 0.7 0.7)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::lch(170.0, 0.7, 0.7)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_lcha_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="lcha(170 0.7 0.7 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::lcha(170.0, 0.7, 0.7, 0.5)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_oklab_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="oklab(170 0.7 0.7)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::oklab(170.0, 0.7, 0.7)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_oklaba_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="oklaba(170 0.7 0.7 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::oklaba(170.0, 0.7, 0.7, 0.5)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_oklch_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="oklch(170 0.7 0.7)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::oklch(170.0, 0.7, 0.7)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_oklcha_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="oklcha(170 0.7 0.7 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::oklcha(170.0, 0.7, 0.7, 0.5)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_xyz_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="xyz(170 0.7 0.7)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::xyz(170.0, 0.7, 0.7)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_string_xyza_colors() {
            let input = quote! {
                <div
                   padding="4px"
                   border-color="xyza(170 0.7 0.7 / 0.5)">
                   "Menu"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        ..Default::default()
                    },
                    ::bevy::ui::BorderColor::all(::bevy::color::Color::xyza(170.0, 0.7, 0.7, 0.5)),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }
    }

    mod nodes {
        use super::*;

        #[test]
        fn allows_display_strings() {
            let input = quote! {
                <div display={Display::Flex}>
                    <div display="none"/>
                    <div display="hidden"/>
                    <div display="flex"/>
                    <div display="grid"/>
                    <div display="block"/>
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node {
                        display: Display::Flex,
                        ..Default::default()
                    },
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::Node {
                            display: ::bevy::ui::Display::None,
                            ..Default::default()
                        }),
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::Node {
                            display: ::bevy::ui::Display::None,
                            ..Default::default()
                        }),
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::Node {
                            display: ::bevy::ui::Display::Flex,
                            ..Default::default()
                        }),
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::Node {
                            display: ::bevy::ui::Display::Grid,
                            ..Default::default()
                        }),
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::Node {
                            display: ::bevy::ui::Display::Block,
                            ..Default::default()
                        })
                    ))
                )
            };
            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        #[test]
        fn allows_flex_direction_strings() {
            let tests = vec![
                ("row", "Row"),
                ("column", "Column"),
                ("col", "Column"),
                ("row-reverse", "RowReverse"),
                ("column-reverse", "ColumnReverse"),
                ("col-reverse", "ColumnReverse"),
                ("Invalid", "Invalid"),
            ];
            for (input, expected) in tests {
                let input = quote! {
                    <div flex-direction=#input/>
                };
                let ident = syn::Ident::new(expected, proc_macro2::Span::call_site());
                let expected = quote! {
                    ::bevy::ui::Node {
                        flex_direction: ::bevy::ui::FlexDirection::#ident,
                        ..Default::default()
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }
        }
    }

    mod children {
        use super::*;

        #[test]
        fn adds_observers() {
            let input = quote! {
                <div onClick={|_event: On<Pointer<Click>>,
                    mut commands: Commands,
                    text: Single<Entity, With<Text>>| {
                        commands.entity(*text).insert(Text::new("Hi, Mom!"));
                    }}>
                    "Hello, World!"
                </div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node::default(),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello, World!")),
                        ::bevy::ecs::spawn::SpawnWith(|parent: &mut ::bevy::ecs::relationship::RelatedSpawner<::bevy::ecs::hierarchy::ChildOf>| {
                            let entity = parent.target_entity();
                            parent.spawn(
                                ::bevy::ecs::observer::Observer::new(
                                    |_event: On<Pointer<Click> >,
                                     mut commands: Commands,
                                     text: Single<Entity, With<Text> >| {
                                        commands.entity(*text).insert(Text::new("Hi, Mom!"));
                                    }
                                )
                                .with_entity(entity)
                            );
                        })
                    ))
                )
            };
            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }
    }

    mod text_components {
        use super::*;

        #[cfg(not(feature = "propagate"))]
        mod no_propagate {
            use super::*;
            #[test]
            fn supports_text_font() {
                let input = quote! {
                    <div
                        padding="4px"
                        font-size="10">
                        "Menu"
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all(),
                            ..Default::default()
                        },
                        ::bevy::text::TextFont {
                            font_size: 10.0,
                            ..Default::default()
                        },
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                        ))
                    )
                };

                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }
        }

        #[cfg(feature = "propagate")]
        mod propagate {
            use super::*;
            #[test]
            fn supports_text_font() {
                let input = quote! {
                    <div
                        padding="4px"
                        font-size="10">
                            "Menu"
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all(),
                            ..Default::default()
                        },
                        ::bevy::app::Propagate(::bevy::text::TextFont {
                            font_size: 10.0,
                            ..Default::default()
                        }),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                        ))
                    )
                };

                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }
        }

        #[test]
        fn supports_text_layout() {
            let input = quote! {
                <div justify={Justify::Left}><span linebreak={LineBreak::NoWrap}>"Hello"</span></div>
            };
            let expected = quote! {
                (
                    ::bevy::ui::Node::default(),
                    ::bevy::text::TextLayout {
                        justify: Justify::Left,
                        ..Default::default()
                    },
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(
                            (
                                ::bevy::ui::widget::Text::new("Hello"),
                                ::bevy::text::TextLayout {
                                    linebreak: LineBreak::NoWrap,
                                    ..Default::default()
                                }
                            )
                        )
                    ))
                )
            };
            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }
    }
}
