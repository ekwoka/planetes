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
                #[cfg(feature = "bsn")]
                tokens.extend(quote! {
                    (#node)
                });
                #[cfg(not(feature = "bsn"))]
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
        #[cfg(feature = "bsn")]
        tokens.extend(quote! {
            bevy::ui::widget::Text(#value)
        });
        #[cfg(not(feature = "bsn"))]
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

    // All attribute keys consumed by the standard component builders plus
    // reserved special keys.  Anything not in this list (and not observer
    // "on*" attrs) with a string-literal value becomes an `extra_attr`.
    const KNOWN_KEYS: &'static [&'static str] = &[
        // NodeComponent (47 keys, see node.rs)
        "padding",
        "padding-top",
        "padding-left",
        "padding-bottom",
        "padding-right",
        "margin",
        "margin-top",
        "margin-left",
        "margin-bottom",
        "margin-right",
        "border",
        "border-top",
        "border-left",
        "border-bottom",
        "border-right",
        "top",
        "left",
        "bottom",
        "right",
        "width",
        "height",
        "min-width",
        "min-height",
        "max-width",
        "max-height",
        "row-gap",
        "column-gap",
        "display",
        "position",
        "position-type",
        "flex-direction",
        "flex-wrap",
        "align-items",
        "justify-items",
        "align-self",
        "justify-self",
        "align-content",
        "justify-content",
        "box-sizing",
        "grid-auto-flow",
        "flex-grow",
        "flex-shrink",
        "scrollbar-width",
        "aspect-ratio",
        "overflow",
        "overflow-clip-margin",
        "border-radius",
        // Other builders
        "src",
        "border-color",
        "background-color",
        "font-size",
        "justify",
        "linebreak",
        "text-color",
        "name",
        "components",
    ];

    fn extra_attrs(attributes: &[Attribute]) -> Vec<TokenStream> {
        attributes
            .iter()
            .filter(|attr| {
                !Self::KNOWN_KEYS.contains(&attr.key.as_str()) && !attr.key.starts_with("on")
            })
            .filter_map(|attr| {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &attr.value
                {
                    let key = syn::LitStr::new(&attr.key, attr.span);
                    let val = syn::LitStr::new(&lit_str.value(), lit_str.span());
                    Some(quote! { (#key, #val) })
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Emit a `NodeName` as a value expression, stripping the outer braces from a
/// single-statement block so that `{ expr }` becomes `expr`.  This prevents
/// the `unused_braces` lint when the result is used as a function argument.
/// Multi-statement blocks and non-block names are emitted as-is.
fn tag_name_as_expr(name: &NodeName) -> TokenStream {
    if let NodeName::Block(block) = name
        && block.stmts.len() == 1
    {
        block.stmts[0].to_token_stream()
    } else {
        name.to_token_stream()
    }
}

impl ToTokens for ElementNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let tag_names = vec![
            "div",
            "img",
            #[cfg(feature = "feathers")]
            "button",
            #[cfg(feature = "feathers")]
            "input",
        ];
        let mut components = Vec::<TokenStream>::new();
        components.push_some(Name::from(&self.attributes).ok());
        let is_custom = !tag_names.contains(&self.tag_name.to_string().as_str());
        if is_custom {
            let tag_name = &self.tag_name;
            let tag_expr = tag_name_as_expr(tag_name);
            let node = NodeComponent::from(&self.attributes);
            let extra = Self::extra_attrs(&self.attributes);

            let background_color = match BackgroundColor::from(&self.attributes).ok() {
                Some(bc) => quote! { #bc },
                None => quote! { ::bevy::ui::BackgroundColor::default() },
            };
            let border_color = match BorderColor::from(&self.attributes).ok() {
                Some(bc) => quote! { #bc },
                None => quote! { ::bevy::ui::BorderColor::default() },
            };
            let text_font = match TextFont::from(&self.attributes).ok() {
                Some(tf) => tf.plain_tokens(),
                None => quote! { ::bevy::text::TextFont::default() },
            };
            let text_color = match TextColor::from(&self.attributes).ok() {
                Some(tc) => tc.plain_tokens(),
                None => quote! { ::bevy::text::TextColor::default() },
            };
            let text_layout = match TextLayout::from(&self.attributes).ok() {
                Some(tl) => quote! { #tl },
                None => quote! { ::bevy::text::TextLayout::default() },
            };

            components.push(quote! {
                <_ as ::bevy_ui_html::HtmlComponent>::build(
                    #tag_expr,
                    ::bevy_ui_html::HtmlBundle {
                        node: #node,
                        background_color: #background_color,
                        border_color: #border_color,
                        text_font: #text_font,
                        text_color: #text_color,
                        text_layout: #text_layout,
                    },
                    &[#(#extra),*]
                )
            });
        }
        #[cfg(feature = "feathers")]
        {
            if self.tag_name.to_string() == "button" {
                let button =
                    feathers::Button::from(&self.attributes).with_children(self.children.clone());
                tokens.extend(quote! {
                    #button
                });
                return;
            }
            if self.tag_name.to_string() == "input" {
                match self
                    .attributes
                    .iter()
                    .filter(|attr| attr.key == "type")
                    .map(|attr| attr.value.to_token_stream().to_string())
                    .next()
                {
                    Some(kind) => match kind.as_str() {
                        "\"checkbox\"" => {
                            let checkbox = feathers::Checkbox::from(&self.attributes);
                            tokens.extend(quote! {
                                #checkbox
                            });
                            return;
                        }
                        "\"radio\"" => {
                            let radio = feathers::Radio::from(&self.attributes);
                            tokens.extend(quote! {
                                #radio
                            });
                            return;
                        }
                        _ => {}
                    },
                    None => {}
                }
            }
        }
        if !is_custom {
            components.push_some(NodeComponent::from(&self.attributes).ok());
            components.push_some(Image::from(&self.attributes).ok());
            components.push_some(BorderColor::from(&self.attributes).ok());
            components.push_some(BackgroundColor::from(&self.attributes).ok());
            components.push_some(TextFont::from(&self.attributes).ok());
            components.push_some(TextColor::from(&self.attributes).ok());
            components.push_some(TextLayout::from(&self.attributes).ok());
        }
        components.push_some(
            Self::get_attr(&self.attributes, "components")
                .and_then(|value| Value::new(value).clean_block()),
        );
        #[cfg(feature = "bsn")]
        components.push_some(Observer::from(&self.attributes).ok());
        let mut children = self
            .children
            .iter()
            .map(|child| child.to_token_stream())
            .collect::<Vec<_>>();
        #[cfg(not(feature = "bsn"))]
        children.push_some(Observer::from(&self.attributes).ok());
        if !children.is_empty() {
            #[cfg(feature = "bsn")]
            components.push(quote! {
                Children[
                    #(#children),*
                ]
            });
            #[cfg(not(feature = "bsn"))]
            components.push(quote! {
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    #(#children),*
                ))
            });
        }
        #[cfg(feature = "bsn")]
        tokens.extend(quote! {
            #(#components)*
        });
        #[cfg(not(feature = "bsn"))]
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
        components.push_some(TextColor::from(&self.attributes).ok());
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

    #[cfg(feature = "bsn")]
    let output = quote! {
            ::bevy::scene::bsn! {
                #output
            }
    };

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

/// Derive macro that implements [`::bevy_ui_html::HtmlComponent`] for a
/// type as a simple marker: the type itself plus the parsed `Node` are
/// returned as the bundle, and `extra_attrs` are ignored.
///
/// # Example
///
/// ```ignore
/// #[derive(Component, HtmlComponent)]
/// struct MenuButton;
///
/// // html! { <MenuButton padding="8px">"Click"</MenuButton> }
/// // spawns an entity with MenuButton + Node { padding: px(8.0).all() } + Children
/// ```
#[proc_macro_derive(HtmlComponent)]
pub fn derive_html_component(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    quote! {
        impl #impl_generics ::bevy_ui_html::HtmlComponent for #name #ty_generics #where_clause {
            fn build(self, props: ::bevy_ui_html::HtmlBundle, _: &'static [(&'static str, &'static str)]) -> impl ::bevy::ecs::bundle::Bundle {
                let ::bevy_ui_html::HtmlBundle {
                    node, background_color, border_color, text_font, text_color, text_layout
                } = props;
                (self, node, background_color, border_color, text_font, text_color, text_layout)
            }
        }
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "bsn"))]
    mod legacy {
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
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
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
                        border_radius: ::bevy::ui::BorderRadius::all(::bevy::ui::px(2.0)),
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
                    <_ as ::bevy_ui_html::HtmlComponent>::build(
                        MenuButton,
                        ::bevy_ui_html::HtmlBundle {
                            node: ::bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all(),
                                border_radius: ::bevy::ui::BorderRadius::all(::bevy::ui::px(2.0)),
                                ..Default::default()

                            },
                            background_color: ::bevy::ui::BackgroundColor::default(),
                            border_color: ::bevy::ui::BorderColor::default(),
                            text_font: ::bevy::text::TextFont::default(),
                            text_color: ::bevy::text::TextColor::default(),
                            text_layout: ::bevy::text::TextLayout::default(),
                        },
                        &[]
                    ),
                    <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Menu"))
                    ))
                )
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        mod html_component {
            use super::*;

            #[test]
            fn self_closing_custom_tag_no_tuple() {
                let input = quote! {
                    <MyComponent />
                };
                let expected = quote! {
                    <_ as ::bevy_ui_html::HtmlComponent>::build(
                        MyComponent,
                        ::bevy_ui_html::HtmlBundle {
                            node: ::bevy::ui::Node::default(),
                            background_color: ::bevy::ui::BackgroundColor::default(),
                            border_color: ::bevy::ui::BorderColor::default(),
                            text_font: ::bevy::text::TextFont::default(),
                            text_color: ::bevy::text::TextColor::default(),
                            text_layout: ::bevy::text::TextLayout::default(),
                        },
                        &[]
                    )
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn extra_string_attrs_forwarded_to_additional_attributes() {
                let input = quote! {
                    <MyComponent padding="4px" variant="primary" />
                };
                let expected = quote! {
                    <_ as ::bevy_ui_html::HtmlComponent>::build(
                        MyComponent,
                        ::bevy_ui_html::HtmlBundle {
                            node: ::bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all(),
                                ..Default::default()
                            },
                            background_color: ::bevy::ui::BackgroundColor::default(),
                            border_color: ::bevy::ui::BorderColor::default(),
                            text_font: ::bevy::text::TextFont::default(),
                            text_color: ::bevy::text::TextColor::default(),
                            text_layout: ::bevy::text::TextLayout::default(),
                        },
                        &[("variant", "primary")]
                    )
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn multiple_extra_attrs_all_forwarded() {
                let input = quote! {
                    <MyComponent variant="primary" size="large" disabled="true" />
                };
                let expected = quote! {
                    <_ as ::bevy_ui_html::HtmlComponent>::build(
                        MyComponent,
                        ::bevy_ui_html::HtmlBundle {
                            node: ::bevy::ui::Node::default(),
                            background_color: ::bevy::ui::BackgroundColor::default(),
                            border_color: ::bevy::ui::BorderColor::default(),
                            text_font: ::bevy::text::TextFont::default(),
                            text_color: ::bevy::text::TextColor::default(),
                            text_layout: ::bevy::text::TextLayout::default(),
                        },
                        &[("variant", "primary"), ("size", "large"), ("disabled", "true")]
                    )
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn standard_component_attrs_passed_inside_html_bundle() {
                let input = quote! {
                    <MyButton padding="4px" background-color="black" variant="primary">
                        "text"
                    </MyButton>
                };
                let expected = quote! {
                    (
                        <_ as ::bevy_ui_html::HtmlComponent>::build(
                            MyButton,
                            ::bevy_ui_html::HtmlBundle {
                                node: ::bevy::ui::Node {
                                    padding: ::bevy::ui::px(4.0).all(),
                                    ..Default::default()
                                },
                                background_color: ::bevy::ui::BackgroundColor(::bevy::color::Color::BLACK),
                                border_color: ::bevy::ui::BorderColor::default(),
                                text_font: ::bevy::text::TextFont::default(),
                                text_color: ::bevy::text::TextColor::default(),
                                text_layout: ::bevy::text::TextLayout::default(),
                            },
                            &[("variant", "primary")]
                        ),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("text"))
                        ))
                    )
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn rust_expression_extra_attrs_not_in_additional_attributes() {
                // Rust-expression values on unknown attrs are silently dropped from additional_attributes
                // since they can't be represented as &'static str
                let input = quote! {
                    <MyComponent variant={some_var} />
                };
                let expected = quote! {
                    <_ as ::bevy_ui_html::HtmlComponent>::build(
                        MyComponent,
                        ::bevy_ui_html::HtmlBundle {
                            node: ::bevy::ui::Node::default(),
                            background_color: ::bevy::ui::BackgroundColor::default(),
                            border_color: ::bevy::ui::BorderColor::default(),
                            text_font: ::bevy::text::TextFont::default(),
                            text_color: ::bevy::text::TextColor::default(),
                            text_layout: ::bevy::text::TextLayout::default(),
                        },
                        &[]
                    )
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }
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

            #[test]
            fn allows_string_colors_on_color_attributes() {
                let input = quote! {
                    <div
                    padding="4px"
                    border-color="srgb(170 170 170)"
                    text-color="srgb(170 170 170)"
                    background-color="srgb(170 170 170)">
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
                        ::bevy::ui::BackgroundColor(::bevy::color::Color::srgb_u8(170, 170, 170)),
                        ::bevy::app::Propagate(::bevy::text::TextColor(::bevy::color::Color::srgb_u8(170, 170, 170))),
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
            fn allows_position_strings() {
                let input = quote! {
                    <div position="absolute">
                        <div position="relative"/>
                        <div position-type="absolute"/>
                        <div position-type="relative"/>
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node {
                            position_type: ::bevy::ui::PositionType::Absolute,
                            ..Default::default()
                        },
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(::bevy::ui::Node {
                                position_type: ::bevy::ui::PositionType::Relative,
                                ..Default::default()
                            }),
                            ::bevy::ecs::spawn::Spawn(::bevy::ui::Node {
                                position_type: ::bevy::ui::PositionType::Absolute,
                                ..Default::default()
                            }),
                            ::bevy::ecs::spawn::Spawn(::bevy::ui::Node {
                                position_type: ::bevy::ui::PositionType::Relative,
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
                                font_size: ::bevy::text::FontSize::Px(10.0),
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

                #[test]
                fn supports_text_color() {
                    let input = quote! {
                        <div
                            padding="4px"
                            text-color="rgb(170 170 170)">
                            "Menu"
                        </div>
                    };
                    let expected = quote! {
                        (
                            ::bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all(),
                                ..Default::default()
                            },
                            ::bevy::text::TextColor(::bevy::color::Color::linear_rgb(0.6666667, 0.6666667, 0.6666667)),
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
                                font_size: ::bevy::text::FontSize::Px(10.0),
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

                #[test]
                fn supports_text_color() {
                    let input = quote! {
                        <div
                            padding="4px"
                            text-color="srgb(170 170 170)">
                            "Menu"
                        </div>
                    };
                    let expected = quote! {
                        (
                            ::bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all(),
                                ..Default::default()
                            },
                            ::bevy::app::Propagate(::bevy::text::TextColor(::bevy::color::Color::srgb_u8(170, 170, 170))),
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

        #[cfg(feature = "feathers")]
        mod feathers_elements {
            use super::*;

            #[test]
            fn renders_button() {
                let input = quote! {
                    <div>
                        <button
                            variant="normal"
                            corners="rounded">"Hello"</button>
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node::default(),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(
                                ::bevy::feathers::controls::button_bundle(
                                    ::bevy::feathers::controls::ButtonBundleProps {
                                        variant: ::bevy::feathers::controls::ButtonVariant::Normal,
                                        corners: ::bevy::feathers::rounded_corners::RoundedCorners::All,
                                        ..Default::default()
                                    },
                                    (),
                                    (::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello")))
                                )
                            )
                        ))
                    )
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn spawns_observer() {
                let input = quote! {
                    <div>
                        <button
                            variant="primary"
                            corners="top left"
                            onActivate={|event: On<Activate>| { info!("{:?}",event.entity); }}>
                            "Hello"
                        </button>
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node::default(),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(
                                ::bevy::feathers::controls::button_bundle(
                                    ::bevy::feathers::controls::ButtonBundleProps {
                                        variant: ::bevy::feathers::controls::ButtonVariant::Primary,
                                        corners: ::bevy::feathers::rounded_corners::RoundedCorners::TopLeft,
                                        ..Default::default()
                                    },
                                    (),
                                    (
                                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello")),
                                        ::bevy::ecs::spawn::SpawnWith(|parent: &mut ::bevy::ecs::relationship::RelatedSpawner<::bevy::ecs::hierarchy::ChildOf>| {
                                            let entity = parent.target_entity();
                                            parent.spawn(
                                                ::bevy::ecs::observer::Observer::new(
                                                    |event: On<Activate>| {
                                                        info!("{:?}",event.entity);
                                                    }
                                                )
                                                .with_entity(entity)
                                            );
                                        })
                                    )
                                )
                            )
                        ))
                    )
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn button_supports_overrides() {
                let input = quote! {
                    <div>
                        <button
                            variant="normal"
                            corners="rounded"
                            components={Testing::new()}>"Hello"</button>
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node::default(),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(
                                ::bevy::feathers::controls::button_bundle(
                                    ::bevy::feathers::controls::ButtonBundleProps {
                                        variant: ::bevy::feathers::controls::ButtonVariant::Normal,
                                        corners: ::bevy::feathers::rounded_corners::RoundedCorners::All,
                                        ..Default::default()
                                    },
                                    Testing::new(),
                                    (::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello")))
                                )
                            )
                        ))
                    )
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn renders_checkbox() {
                let input = quote! {
                    <div>
                        <input
                            type="checkbox"
                            label="Hello"
                            onChange={|event: On<ValueChange<bool>>| {
                                println!("Hello Changed {}", event.value)
                            }} />
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node::default(),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(
                                ::bevy::feathers::controls::checkbox_bundle(
                                    (),
                                    (
                                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello")),
                                        ::bevy::ecs::spawn::SpawnWith(|parent: &mut ::bevy::ecs::relationship::RelatedSpawner<::bevy::ecs::hierarchy::ChildOf>| {
                                            let entity = parent.target_entity();
                                            parent.spawn(
                                                ::bevy::ecs::observer::Observer::new(
                                                    |event: On< ValueChange<bool> >| {
                                                        println!("Hello Changed {}", event.value)
                                                    }
                                                )
                                                .with_entity(entity)
                                            );
                                        })
                                    )
                                )
                            )
                        ))
                    )
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn renders_radio() {
                let input = quote! {
                    <div>
                        <input
                            type="radio"
                            label="Hello"
                            onChange={|event: On<ValueChange<bool>>| {
                                println!("Hello True {}", event.value)
                            }}
                            components={TestComponent} />
                    </div>
                };
                let expected = quote! {
                    (
                        ::bevy::ui::Node::default(),
                        <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                            ::bevy::ecs::spawn::Spawn(
                                ::bevy::feathers::controls::radio_bundle(
                                    TestComponent,
                                    (
                                        ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello")),
                                        ::bevy::ecs::spawn::SpawnWith(|parent: &mut ::bevy::ecs::relationship::RelatedSpawner<::bevy::ecs::hierarchy::ChildOf>| {
                                            let entity = parent.target_entity();
                                            parent.spawn(
                                                ::bevy::ecs::observer::Observer::new(
                                                    |event: On< ValueChange<bool> >| {
                                                        println!("Hello True {}", event.value)
                                                    }
                                                )
                                                .with_entity(entity)
                                            );
                                        })
                                    )
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

    #[cfg(feature = "bsn")]
    mod bsn {
        use super::*;
        #[test]
        fn single_div_with_text() {
            let input = quote! {
                <div>"Hello"</div>
            };
            let output = quote! {
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        (bevy::ui::widget::Text("Hello"))
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        (
                            bevy::ui::Node
                            Children[(bevy::ui::widget::Text("Hello"))]
                        ),
                        (
                            bevy::ui::Node
                            Children[(bevy::ui::widget::Text("World"))]
                        )
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        (bevy::ui::widget::Text("Hello")),(bevy::ui::widget::Text("World"))
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node {
                        padding: ::bevy::ui::px(10.0).all().with_bottom(::bevy::ui::percent(20.0)),
                        margin: ::bevy::ui::vw(5.0).top().with_right(::bevy::ui::vmax(20.0)).with_bottom(::bevy::ui::vmin(15.0)).with_left(::bevy::ui::vh(10.0))
                    }
                    Children[(
                        bevy::ui::widget::Text("Hello")
                    )]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node {
                        padding: px(10.0).all().with_bottom(percent(20.0)),
                        margin: vw(5.0).top().with_right(vmax(20.0)).with_bottom(vmin(15.0)).with_left(vh(10.0))
                    }
                    Children[(
                        bevy::ui::widget::Text("Hello")
                    )]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node {
                        left: ::bevy::ui::px(5.0),
                        top: ::bevy::ui::px(10.0),
                        width: ::bevy::ui::px(100.0),
                        height: ::bevy::ui::px(50.0),
                        min_width: ::bevy::ui::px(10.0),
                        max_width: ::bevy::ui::px(200.0)
                    }
                    Children[(
                        bevy::ui::widget::Text("Test")
                    )]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        flex_grow: 1.0,
                        flex_shrink: 0.5
                    }
                    Children[(
                        bevy::ui::widget::Text("Flex")
                    )]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node {
                        border: ::bevy::ui::px(2.0).all().with_top(::bevy::ui::px(5.0)),
                        row_gap: ::bevy::ui::px(10.0),
                        column_gap: ::bevy::ui::px(15.0)
                    }
                    Children[(
                        bevy::ui::widget::Text("Borders")
                    )]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node {
                        width: ::bevy::ui::percent(100.0),
                        position_type: PositionType::Absolute,
                        aspect_ratio: Some(1.77)
                    }
                    Children[(
                        bevy::ui::widget::Text("Aspect")
                    )]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        (Text::new("Hello"))
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        (if show { Text::new("Visible") } else { Text::new("Hidden") })
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        (::bevy::scene::bsn!{ bevy::ui::widget::Text("Nested") })
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        (bevy::ui::widget::Text("Static text")),
                        (dynamic_content),
                        (MyComponent::new())
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        ::bevy::ecs::spawn::SpawnIter(
                            items.map(|item| {
                                html! {
                                    <div>{item.name}</div>
                                }
                            })
                        )
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all(),
                        border_radius: ::bevy::ui::BorderRadius::all(::bevy::ui::px(2.0))
                    }
                    Children[(
                        bevy::ui::widget::Text("Menu")
                    )]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all()
                    }
                    ::bevy::ui::BorderColor::all(Color::linear_rgb(0.7, 0.7, 0.7))
                    Children[(
                        bevy::ui::widget::Text("Menu")
                    )]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node {
                        padding: ::bevy::ui::px(4.0).all()
                    }
                    ::bevy::ui::BackgroundColor(Color::linear_rgb(0.7, 0.7, 0.7))
                    Children[(
                        bevy::ui::widget::Text("Menu")
                    )]
                }
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
                ::bevy::scene::bsn!{
                    <_ as ::bevy_ui_html::HtmlComponent>::build(
                        MenuButton,
                        ::bevy_ui_html::HtmlBundle {
                            node: bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all(),
                                border_radius: ::bevy::ui::BorderRadius::all(::bevy::ui::px(2.0))
                            },
                            background_color: ::bevy::ui::BackgroundColor::default(),
                            border_color: ::bevy::ui::BorderColor::default(),
                            text_font: ::bevy::text::TextFont::default(),
                            text_color: ::bevy::text::TextColor::default(),
                            text_layout: ::bevy::text::TextLayout::default(),
                        },
                        &[]
                    )
                    Children[(
                        bevy::ui::widget::Text("Menu")
                    )]
                }
            };

            let result = html_inner(input);
            assert_eq!(result.to_string(), expected.to_string());
        }

        mod html_component {
            use super::*;

            #[test]
            fn self_closing_custom_tag_no_tuple() {
                let input = quote! {
                    <MyComponent />
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        <_ as ::bevy_ui_html::HtmlComponent>::build(
                            MyComponent,
                            ::bevy_ui_html::HtmlBundle {
                                node: bevy::ui::Node,
                                background_color: ::bevy::ui::BackgroundColor::default(),
                                border_color: ::bevy::ui::BorderColor::default(),
                                text_font: ::bevy::text::TextFont::default(),
                                text_color: ::bevy::text::TextColor::default(),
                                text_layout: ::bevy::text::TextLayout::default(),
                            },
                            &[]
                        )
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn extra_string_attrs_forwarded_to_additional_attributes() {
                let input = quote! {
                    <MyComponent padding="4px" variant="primary" />
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        <_ as ::bevy_ui_html::HtmlComponent>::build(
                            MyComponent,
                            ::bevy_ui_html::HtmlBundle {
                                node: bevy::ui::Node {
                                    padding: ::bevy::ui::px(4.0).all()
                                },
                                background_color: ::bevy::ui::BackgroundColor::default(),
                                border_color: ::bevy::ui::BorderColor::default(),
                                text_font: ::bevy::text::TextFont::default(),
                                text_color: ::bevy::text::TextColor::default(),
                                text_layout: ::bevy::text::TextLayout::default(),
                            },
                            &[("variant", "primary")]
                        )
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn multiple_extra_attrs_all_forwarded() {
                let input = quote! {
                    <MyComponent variant="primary" size="large" disabled="true" />
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        <_ as ::bevy_ui_html::HtmlComponent>::build(
                            MyComponent,
                            ::bevy_ui_html::HtmlBundle {
                                node: bevy::ui::Node,
                                background_color: ::bevy::ui::BackgroundColor::default(),
                                border_color: ::bevy::ui::BorderColor::default(),
                                text_font: ::bevy::text::TextFont::default(),
                                text_color: ::bevy::text::TextColor::default(),
                                text_layout: ::bevy::text::TextLayout::default(),
                            },
                            &[("variant", "primary"), ("size", "large"), ("disabled", "true")]
                        )
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn standard_component_attrs_passed_inside_html_bundle() {
                let input = quote! {
                    <MyButton padding="4px" background-color="black" variant="primary">
                        "text"
                    </MyButton>
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        <_ as ::bevy_ui_html::HtmlComponent>::build(
                            MyButton,
                            ::bevy_ui_html::HtmlBundle {
                                node: bevy::ui::Node {
                                    padding: ::bevy::ui::px(4.0).all()
                                },
                                background_color: ::bevy::ui::BackgroundColor(::bevy::color::Color::BLACK),
                                border_color: ::bevy::ui::BorderColor::default(),
                                text_font: ::bevy::text::TextFont::default(),
                                text_color: ::bevy::text::TextColor::default(),
                                text_layout: ::bevy::text::TextLayout::default(),
                            },
                            &[("variant", "primary")]
                        )
                        Children[(
                            bevy::ui::widget::Text("text")
                        )]
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn rust_expression_extra_attrs_not_in_additional_attributes() {
                // Rust-expression values on unknown attrs are silently dropped from additional_attributes
                // since they can't be represented as &'static str
                let input = quote! {
                    <MyComponent variant={some_var} />
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        <_ as ::bevy_ui_html::HtmlComponent>::build(
                            MyComponent,
                            ::bevy_ui_html::HtmlBundle {
                                node: bevy::ui::Node,
                                background_color: ::bevy::ui::BackgroundColor::default(),
                                border_color: ::bevy::ui::BorderColor::default(),
                                text_font: ::bevy::text::TextFont::default(),
                                text_color: ::bevy::text::TextColor::default(),
                                text_layout: ::bevy::text::TextLayout::default(),
                            },
                            &[]
                        )
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        (
                            bevy::ui::Node
                            Children[(Text::new("Menu"))]
                        ),
                        (
                            bevy::ui::Node
                            Children[({
                                let thing = Text::new("Thing");
                                thing
                            })]
                        )
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        (::bevy::ui::widget::Text::new("Hello")),
                        (::bevy::ui::widget::Text::new({let thing = true; if thing { "World" } else { "Mom" }}))
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    Children[
                        ::bevy::ecs::spawn::SpawnWith(move |parent: &mut ::bevy::ecs::relationship::RelatedSpawner<::bevy::ecs::hierarchy::ChildOf>| {
                            if true {
                                parent.spawn(html! { <div>"Hello World"</div>});
                            } else {
                                parent.spawn(html! { <div>"Hello Mom"</div>});
                            }
                        })
                    ]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    (Checkable, Checked)
                    Children[(bevy::ui::widget::Text("Hello"))]
                }
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
                ::bevy::scene::bsn!{
                    ::bevy::ecs::name::Name::new("hello")
                    bevy::ui::Node
                    Children[(bevy::ui::widget::Text("World"))]
                }
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
                ::bevy::scene::bsn!{
                    bevy::ui::Node
                    ::bevy::ui::widget::ImageNode::new(
                        asset_server
                            .load("embedded://planetes_editor/assets/filled_triangle.png")
                    )
                }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(Color::linear_rgb(0.7, 0.7, 0.7))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
                };

                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn allows_string_colors_on_color_attributes() {
                let input = quote! {
                    <div
                    padding="4px"
                    border-color="srgb(170 170 170)"
                    text-color="srgb(170 170 170)"
                    background-color="srgb(170 170 170)">
                    "Menu"
                    </div>
                };
                #[cfg(feature = "propagate")]
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::srgb_u8(170, 170, 170))
                        ::bevy::ui::BackgroundColor(::bevy::color::Color::srgb_u8(170, 170, 170))
                        bevy::app::Propagate(::bevy::text::TextColor(::bevy::color::Color::srgb_u8(170, 170, 170)))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
                };
                #[cfg(not(feature = "propagate"))]
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::srgb_u8(170, 170, 170))
                        ::bevy::ui::BackgroundColor(::bevy::color::Color::srgb_u8(170, 170, 170))
                        bevy::text::TextColor(::bevy::color::Color::srgb_u8(170, 170, 170))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                        ::bevy::scene::bsn!{
                            bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all()
                            }
                            ::bevy::ui::BorderColor::all(::bevy::color::Color::BLACK)
                            Children[(
                                bevy::ui::widget::Text("Menu")
                            )]
                        }
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
                        ::bevy::scene::bsn!{
                            bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all()
                            }
                            ::bevy::ui::BorderColor::all(::bevy::color::Color::WHITE)
                            Children[(
                                bevy::ui::widget::Text("Menu")
                            )]
                        }
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
                        ::bevy::scene::bsn!{
                            bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all()
                            }
                            ::bevy::ui::BorderColor::all(::bevy::color::Color::NONE)
                            Children[(
                                bevy::ui::widget::Text("Menu")
                            )]
                        }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::linear_rgb(0.6666667, 0.6666667, 0.6666667))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::linear_rgba(0.6666667, 0.6666667, 0.6666667, 0.5))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::srgb(0.7, 0.7, 0.7))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::srgb_u8(170, 170, 170))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::srgba(0.7, 0.7, 0.7, 0.5))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::srgba_u8(170, 170, 170, 127))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::hsl(170.0, 0.7, 0.7))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::hsla(170.0, 0.7, 0.7, 0.5))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::hsv(170.0, 0.7, 0.7))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::hsva(170.0, 0.7, 0.7, 0.5))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::hwb(170.0, 0.7, 0.7))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::hwba(170.0, 0.7, 0.7, 0.5))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::lab(170.0, 0.7, 0.7))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::laba(170.0, 0.7, 0.7, 0.5))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::lch(170.0, 0.7, 0.7))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::lcha(170.0, 0.7, 0.7, 0.5))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::oklab(170.0, 0.7, 0.7))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::oklaba(170.0, 0.7, 0.7, 0.5))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::oklch(170.0, 0.7, 0.7))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::oklcha(170.0, 0.7, 0.7, 0.5))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::xyz(170.0, 0.7, 0.7))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            padding: ::bevy::ui::px(4.0).all()
                        }
                        ::bevy::ui::BorderColor::all(::bevy::color::Color::xyza(170.0, 0.7, 0.7, 0.5))
                        Children[(
                            bevy::ui::widget::Text("Menu")
                        )]
                    }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            display: Display::Flex
                        }
                        Children[
                            (bevy::ui::Node {
                                display: ::bevy::ui::Display::None
                            }),
                            (bevy::ui::Node {
                                display: ::bevy::ui::Display::None
                            }),
                            (bevy::ui::Node {
                                display: ::bevy::ui::Display::Flex
                            }),
                            (bevy::ui::Node {
                                display: ::bevy::ui::Display::Grid
                            }),
                            (bevy::ui::Node {
                                display: ::bevy::ui::Display::Block
                            })
                        ]
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn allows_position_strings() {
                let input = quote! {
                    <div position="absolute">
                        <div position="relative"/>
                        <div position-type="absolute"/>
                        <div position-type="relative"/>
                    </div>
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        bevy::ui::Node {
                            position_type: ::bevy::ui::PositionType::Absolute
                        }
                        Children[
                            (bevy::ui::Node {
                                position_type: ::bevy::ui::PositionType::Relative
                            }),
                            (bevy::ui::Node {
                                position_type: ::bevy::ui::PositionType::Absolute
                            }),
                            (bevy::ui::Node {
                                position_type: ::bevy::ui::PositionType::Relative
                            })
                        ]
                    }
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
                        ::bevy::scene::bsn!{
                            bevy::ui::Node {
                                flex_direction: ::bevy::ui::FlexDirection::#ident
                            }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node
                        on(|_event: On<Pointer<Click> >,
                            mut commands: Commands,
                            text: Single<Entity, With<Text> >| {
                                commands.entity(*text).insert(Text::new("Hi, Mom!"));
                            })
                        Children[
                            (bevy::ui::widget::Text("Hello, World!"))
                        ]
                    }
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
                        ::bevy::scene::bsn!{
                            bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all()
                            }
                            ::bevy::text::TextFont {
                                font_size: ::bevy::text::FontSize::Px(10.0),

                            }
                            Children[(bevy::ui::widget::Text("Menu"))]
                        }
                    };

                    let result = html_inner(input);
                    assert_eq!(result.to_string(), expected.to_string());
                }

                #[test]
                fn supports_text_color() {
                    let input = quote! {
                        <div
                            padding="4px"
                            text-color="rgb(170 170 170)">
                            "Menu"
                        </div>
                    };
                    let expected = quote! {
                        ::bevy::scene::bsn!{
                            bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all()
                            }
                            ::bevy::text::TextColor(::bevy::color::Color::linear_rgb(0.6666667, 0.6666667, 0.6666667))
                            Children[(bevy::ui::widget::Text("Menu"))]
                        }
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
                        ::bevy::scene::bsn!{
                            bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all()
                            }
                            bevy::app::Propagate(::bevy::text::TextFont {
                                font_size: ::bevy::text::FontSize::Px(10.0),
                                ..Default::default()
                            })
                            Children[(bevy::ui::widget::Text("Menu"))]
                        }
                    };

                    let result = html_inner(input);
                    assert_eq!(result.to_string(), expected.to_string());
                }

                #[test]
                fn supports_text_color() {
                    let input = quote! {
                        <div
                            padding="4px"
                            text-color="srgb(170 170 170)">
                            "Menu"
                        </div>
                    };
                    let expected = quote! {
                        ::bevy::scene::bsn!{
                            bevy::ui::Node {
                                padding: ::bevy::ui::px(4.0).all()
                            }
                            bevy::app::Propagate(::bevy::text::TextColor(::bevy::color::Color::srgb_u8(170, 170, 170)))
                            Children[(bevy::ui::widget::Text("Menu"))]
                        }
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
                    ::bevy::scene::bsn!{
                        bevy::ui::Node
                        bevy::text::TextLayout {
                            justify: Justify::Left
                        }
                        Children[
                            ((
                                bevy::ui::widget::Text("Hello"),
                                bevy::text::TextLayout {
                                    linebreak: LineBreak::NoWrap
                                }
                            ))
                        ]
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }
        }

        #[cfg(feature = "feathers")]
        mod feathers_elements {
            use super::*;

            #[test]
            fn renders_button() {
                let input = quote! {
                    <div>
                        <button
                            variant="normal"
                            corners="rounded">"Hello"</button>
                    </div>
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        bevy::ui::Node
                        Children[
                            (::bevy::feathers::controls::button_bundle(
                                ::bevy::feathers::controls::ButtonBundleProps {
                                    variant: ::bevy::feathers::controls::ButtonVariant::Normal,
                                    corners: ::bevy::feathers::rounded_corners::RoundedCorners::All,

                                },
                                (),
                                ((bevy::ui::widget::Text("Hello")))
                            ))
                        ]
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn spawns_observer() {
                let input = quote! {
                    <div>
                        <button
                            variant="primary"
                            corners="top left"
                            onActivate={|event: On<Activate>| { info!("{:?}",event.entity); }}>
                            "Hello"
                        </button>
                    </div>
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        bevy::ui::Node
                        Children[
                            (::bevy::feathers::controls::button_bundle(
                                ::bevy::feathers::controls::ButtonBundleProps {
                                    variant: ::bevy::feathers::controls::ButtonVariant::Primary,
                                    corners: ::bevy::feathers::rounded_corners::RoundedCorners::TopLeft,

                                },
                                (),
                                (
                                    (bevy::ui::widget::Text("Hello")),
                                    ::bevy::ecs::spawn::SpawnWith(|parent: &mut ::bevy::ecs::relationship::RelatedSpawner<::bevy::ecs::hierarchy::ChildOf>| {
                                        let entity = parent.target_entity();
                                        parent.spawn(
                                            ::bevy::ecs::observer::Observer::new(
                                                |event: On<Activate>| {
                                                    info!("{:?}",event.entity);
                                                }
                                            )
                                            .with_entity(entity)
                                        );
                                    })
                                )
                            ))
                        ]
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn button_supports_overrides() {
                let input = quote! {
                    <div>
                        <button
                            variant="normal"
                            corners="rounded"
                            components={Testing::new()}>"Hello"</button>
                    </div>
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        bevy::ui::Node
                        Children[
                            (::bevy::feathers::controls::button_bundle(
                                ::bevy::feathers::controls::ButtonBundleProps {
                                    variant: ::bevy::feathers::controls::ButtonVariant::Normal,
                                    corners: ::bevy::feathers::rounded_corners::RoundedCorners::All,

                                },
                                Testing::new(),
                                ((bevy::ui::widget::Text("Hello")))
                            ))
                        ]
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn renders_checkbox() {
                let input = quote! {
                    <div>
                        <input
                            type="checkbox"
                            label="Hello"
                            onChange={|event: On<ValueChange<bool>>| {
                                println!("Hello Changed {}", event.value)
                            }} />
                    </div>
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        bevy::ui::Node
                        Children[
                            (::bevy::feathers::controls::checkbox_bundle(
                                (),
                                (
                                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello")),
                                    ::bevy::ecs::spawn::SpawnWith(|parent: &mut ::bevy::ecs::relationship::RelatedSpawner<::bevy::ecs::hierarchy::ChildOf>| {
                                        let entity = parent.target_entity();
                                        parent.spawn(
                                            ::bevy::ecs::observer::Observer::new(
                                                |event: On< ValueChange<bool> >| {
                                                    println!("Hello Changed {}", event.value)
                                                }
                                            )
                                            .with_entity(entity)
                                        );
                                    })
                                )
                            ))
                        ]
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }

            #[test]
            fn renders_radio() {
                let input = quote! {
                    <div>
                        <input
                            type="radio"
                            label="Hello"
                            onChange={|event: On<ValueChange<bool>>| {
                                println!("Hello True {}", event.value)
                            }}
                            components={TestComponent} />
                    </div>
                };
                let expected = quote! {
                    ::bevy::scene::bsn!{
                        bevy::ui::Node
                        Children[
                            (::bevy::feathers::controls::radio_bundle(
                                TestComponent,
                                (
                                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("Hello")),
                                    ::bevy::ecs::spawn::SpawnWith(|parent: &mut ::bevy::ecs::relationship::RelatedSpawner<::bevy::ecs::hierarchy::ChildOf>| {
                                        let entity = parent.target_entity();
                                        parent.spawn(
                                            ::bevy::ecs::observer::Observer::new(
                                                |event: On< ValueChange<bool> >| {
                                                    println!("Hello True {}", event.value)
                                                }
                                            )
                                            .with_entity(entity)
                                        );
                                    })
                                )
                            ))
                        ]
                    }
                };
                let result = html_inner(input);
                assert_eq!(result.to_string(), expected.to_string());
            }
        }
    }
}
