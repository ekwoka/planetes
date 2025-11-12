use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use rstml::{
    node::{Node, NodeName},
    parse2,
};

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

                if tag_name == "span" {
                    Self::Inline(InlineNode {
                        children: children.collect(),
                    })
                } else {
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
                                    });
                                }
                                None
                            } else {
                                None
                            }
                        })
                        .collect();
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
    fn kebab_to_snake(s: &str) -> String {
        s.replace('-', "_")
    }

    fn get_attr<'a>(attributes: &'a [Attribute], kebab_name: &str) -> Option<&'a syn::Expr> {
        attributes
            .iter()
            .find(|attr| attr.key == kebab_name)
            .map(|attr| &attr.value)
    }

    fn parse_css_value(value: &syn::Expr) -> Option<TokenStream> {
        // Handle block expressions - extract the inner content
        let value_tokens = if let syn::Expr::Block(expr_block) = value {
            let stmts = &expr_block.block.stmts;
            quote! { #(#stmts)* }
        } else {
            quote! { #value }
        };

        // Try to parse as a string literal (CSS-style values)
        let value_str = value_tokens.to_string();

        // Check if this is a quoted string (CSS-style value)
        if value_str.starts_with('"') && value_str.ends_with('"') {
            // Remove quotes
            let value_str = value_str.trim_matches('"');

            if let Some(px_value) = value_str.strip_suffix("px") {
                let num =
                    syn::LitFloat::new(&format!("{}.0", px_value), proc_macro2::Span::call_site());
                Some(quote! { ::bevy::ui::px(#num) })
            } else if let Some(percent_value) = value_str.strip_suffix("%") {
                let num = syn::LitFloat::new(
                    &format!("{}.0", percent_value),
                    proc_macro2::Span::call_site(),
                );
                Some(quote! { ::bevy::ui::percent(#num) })
            } else if let Some(vw_value) = value_str.strip_suffix("vw") {
                let num =
                    syn::LitFloat::new(&format!("{}.0", vw_value), proc_macro2::Span::call_site());
                Some(quote! { ::bevy::ui::vw(#num) })
            } else if let Some(vh_value) = value_str.strip_suffix("vh") {
                let num =
                    syn::LitFloat::new(&format!("{}.0", vh_value), proc_macro2::Span::call_site());
                Some(quote! { ::bevy::ui::vh(#num) })
            } else if let Some(vmin_value) = value_str.strip_suffix("vmin") {
                let num = syn::LitFloat::new(
                    &format!("{}.0", vmin_value),
                    proc_macro2::Span::call_site(),
                );
                Some(quote! { ::bevy::ui::vmin(#num) })
            } else if let Some(vmax_value) = value_str.strip_suffix("vmax") {
                let num = syn::LitFloat::new(
                    &format!("{}.0", vmax_value),
                    proc_macro2::Span::call_site(),
                );
                Some(quote! { ::bevy::ui::vmax(#num) })
            } else {
                Some(quote! { ::bevy::ui::Val::from(#value_tokens) })
            }
        } else {
            // Not a CSS-style value, assume it's a Rust expression (e.g., px(10.0))
            // Use the value directly without prepending namespace
            Some(value_tokens)
        }
    }

    fn parse_enum_value(value: &syn::Expr) -> Option<TokenStream> {
        // Handle block expressions - extract the inner content
        let value_tokens = if let syn::Expr::Block(expr_block) = value {
            let stmts = &expr_block.block.stmts;
            quote! { #(#stmts)* }
        } else {
            quote! { #value }
        };

        // Check if this is a quoted string (CSS-style enum value)
        let value_str = value_tokens.to_string();
        if value_str.starts_with('"') && value_str.ends_with('"') {
            // Remove quotes and use the string as-is (CSS-style)
            let value_str = value_str.trim_matches('"');
            // Return as an identifier path
            if let Ok(path) = syn::parse_str::<syn::Path>(value_str) {
                return Some(quote! { #path });
            }
            None
        } else {
            // Not a CSS-style value, assume it's a Rust expression
            Some(value_tokens)
        }
    }

    fn parse_numeric_value(value: &syn::Expr) -> Option<TokenStream> {
        // Handle block expressions - extract the inner content
        let value_tokens = if let syn::Expr::Block(expr_block) = value {
            let stmts = &expr_block.block.stmts;
            quote! { #(#stmts)* }
        } else {
            quote! { #value }
        };

        let value_str = value_tokens.to_string();
        // Check if this is a quoted string
        if value_str.starts_with('"') && value_str.ends_with('"') {
            let value_str = value_str.trim_matches('"');
            // Try to parse as a number
            if let Ok(num) = value_str.parse::<f32>() {
                let lit = syn::LitFloat::new(
                    format!("{num:#?}").as_str(),
                    proc_macro2::Span::call_site(),
                );
                return Some(quote! { #lit });
            }
            None
        } else {
            // Not a CSS-style value, assume it's a Rust expression
            Some(value_tokens)
        }
    }

    fn build_spacing_chain(attributes: &[Attribute], property: &str) -> Option<TokenStream> {
        // Define directions in priority order: all -> top -> right -> bottom -> left
        let directions = [
            ("", "all"),
            ("-top", "top"),
            ("-right", "right"),
            ("-bottom", "bottom"),
            ("-left", "left"),
        ];

        // Collect parsed values for each direction
        let values: Vec<_> = directions
            .iter()
            .map(|(suffix, _)| {
                let key = format!("{}{}", property, suffix);
                Self::get_attr(attributes, &key).and_then(Self::parse_css_value)
            })
            .collect();

        // Find the first available direction and start the chain
        let (start_idx, mut chain) = values.iter().enumerate().find_map(|(idx, val)| {
            val.as_ref().map(|v| {
                let method = syn::Ident::new(directions[idx].1, proc_macro2::Span::call_site());
                (idx, quote! { #v.#method() })
            })
        })?;

        // Chain the remaining directions using with_X() methods
        for (idx, val) in values.iter().enumerate() {
            if idx != start_idx
                && let Some(v) = val
            {
                let method_name = format!("with_{}", directions[idx].1);
                let method = syn::Ident::new(&method_name, proc_macro2::Span::call_site());
                chain = quote! { #chain.#method(#v) };
            }
        }

        Some(chain)
    }
}

impl ToTokens for ElementNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut components = Vec::<TokenStream>::new();
        components.push_some(Name::from(&self.attributes).ok());
        if self.tag_name.to_string() != "div" {
            let tag_name = &self.tag_name;
            components.push(quote! {
                #tag_name
            })
        }
        let children = &self.children;
        if self.attributes.is_empty() {
            components.push(quote! { ::bevy::ui::Node::default() });
        } else {
            let mut fields = Vec::new();

            // Process padding attributes
            if let Some(padding_tokens) = Self::build_spacing_chain(&self.attributes, "padding") {
                fields.push(quote! {
                    padding: #padding_tokens
                });
            }

            // Process margin attributes
            if let Some(margin_tokens) = Self::build_spacing_chain(&self.attributes, "margin") {
                fields.push(quote! {
                    margin: #margin_tokens
                });
            }

            // Process border attributes
            if let Some(border_tokens) = Self::build_spacing_chain(&self.attributes, "border") {
                fields.push(quote! {
                    border: #border_tokens
                });
            }

            // Process simple Val properties (positioning)
            for prop in ["left", "right", "top", "bottom"] {
                if let Some(value) =
                    Self::get_attr(&self.attributes, prop).and_then(Self::parse_css_value)
                {
                    let field = syn::Ident::new(prop, proc_macro2::Span::call_site());
                    fields.push(quote! {
                        #field: #value
                    });
                }
            }

            // Process sizing properties
            for prop in [
                "width",
                "height",
                "min-width",
                "min-height",
                "max-width",
                "max-height",
            ] {
                if let Some(value) =
                    Self::get_attr(&self.attributes, prop).and_then(Self::parse_css_value)
                {
                    let field_name = Self::kebab_to_snake(prop);
                    let field = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
                    fields.push(quote! {
                        #field: #value
                    });
                }
            }

            // Process gap properties
            for prop in ["row-gap", "column-gap"] {
                if let Some(value) =
                    Self::get_attr(&self.attributes, prop).and_then(Self::parse_css_value)
                {
                    let field_name = Self::kebab_to_snake(prop);
                    let field = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
                    fields.push(quote! {
                        #field: #value
                    });
                }
            }

            // Process flex-basis
            if let Some(value) =
                Self::get_attr(&self.attributes, "flex-basis").and_then(Self::parse_css_value)
            {
                fields.push(quote! {
                    flex_basis: #value
                });
            }

            // Process enum properties
            for prop in [
                "display",
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
            ] {
                if let Some(value) =
                    Self::get_attr(&self.attributes, prop).and_then(Self::parse_enum_value)
                {
                    let field_name = Self::kebab_to_snake(prop);
                    let field = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
                    fields.push(quote! {
                        #field: #value
                    });
                }
            }

            // Process numeric properties (f32)
            for prop in ["flex-grow", "flex-shrink", "scrollbar-width"] {
                if let Some(value) =
                    Self::get_attr(&self.attributes, prop).and_then(Self::parse_numeric_value)
                {
                    let field_name = Self::kebab_to_snake(prop);
                    let field = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
                    fields.push(quote! {
                        #field: #value
                    });
                }
            }

            // Process aspect-ratio (Option<f32>)
            if let Some(value) = Self::get_attr(&self.attributes, "aspect-ratio")
                && let Some(parsed) = Self::parse_numeric_value(value)
            {
                fields.push(quote! {
                    aspect_ratio: Some(#parsed)
                });
            }

            // Process overflow (special struct)
            if let Some(value) =
                Self::get_attr(&self.attributes, "overflow").and_then(Self::parse_enum_value)
            {
                fields.push(quote! {
                    overflow: #value
                });
            }

            // Process overflow-clip-margin (special struct)
            if let Some(value) = Self::get_attr(&self.attributes, "overflow-clip-margin")
                .and_then(Self::parse_enum_value)
            {
                fields.push(quote! {
                    overflow_clip_margin: #value
                });
            }

            components.push(quote! {
                ::bevy::ui::Node {
                    #(#fields,)*
                    ..Default::default()
                }
            });
        };

        components.push_some(BorderRadius::from(&self.attributes).ok());
        components.push_some(BorderColor::from(&self.attributes).ok());
        components.push_some(BackgroundColor::from(&self.attributes).ok());
        components.push_some(TextFont::from(&self.attributes).ok());
        components.push_some(
            Self::get_attr(&self.attributes, "components")
                .and_then(|value| Value::new(value).clean_block()),
        );

        tokens.extend(quote! {
            (
                #(#components,)*
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    #(#children),*
                ))
            )
        });
    }
}

impl ToTokens for InlineNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let children = &self
            .children
            .iter()
            .map(|child| match child {
                HtmlNode::Block(block) => quote! {
                    ::bevy::ui::widget::Text::new(#block)
                },
                _ => quote! {
                    #child
                },
            })
            .collect::<Vec<_>>();
        tokens.extend(quote! {
            #(#children),*
        });
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

    #[cfg(not(feature = "propagate"))]
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

    #[cfg(feature = "propagate")]
    #[test]
    fn supports_text_font_with_propagate() {
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
                ::bevy::ui::Node { ..Default::default() },
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
                ::bevy::ui::Node { ..Default::default() },
                <::bevy::ecs::hierarchy::Children as ::bevy::ecs::spawn::SpawnRelated>::spawn((
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new("World"))
                ))
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), expected.to_string());
    }
}
