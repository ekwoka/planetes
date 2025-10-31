use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use rstml::{node::Node, parse2};
use std::collections::HashMap;

#[derive(Debug)]
enum HtmlNode {
    Text(TextNode),
    Element(ElementNode),
    Inline(InlineNode),
}

#[derive(Debug)]
struct TextNode {
    value: String,
}

#[derive(Debug)]
struct ElementNode {
    children: Vec<HtmlNode>,
    attributes: HashMap<String, TokenStream>,
}

#[derive(Debug)]
struct InlineNode {
    children: Vec<HtmlNode>,
}

impl From<Node> for HtmlNode {
    fn from(node: Node) -> Self {
        match node {
            Node::Text(text) => Self::Text(TextNode {
                value: text.value_string(),
            }),
            Node::Element(element) => {
                let children = element
                    .children
                    .into_iter()
                    .filter_map(|child| match child {
                        Node::Text(_) | Node::Element(_) => Some(Self::from(child)),
                        _ => None,
                    })
                    .collect();

                let tag_name = element.open_tag.name.to_string();
                let attributes: HashMap<String, TokenStream> = element
                    .open_tag
                    .attributes
                    .into_iter()
                    .filter_map(|attr| {
                        if let rstml::node::NodeAttribute::Attribute(attr) = attr {
                            let key = attr.key.to_string();
                            if let Some(value_expr) = attr.value() {
                                // Try to extract string literal from the expression
                                if let syn::Expr::Lit(expr_lit) = value_expr
                                    && let syn::Lit::Str(lit_str) = &expr_lit.lit
                                {
                                    // String literal - return as TokenStream for later parsing
                                    let value = lit_str.value();
                                    return Some((key, quote! { #value }));
                                }
                                // For block expressions, extract the inner content
                                if let syn::Expr::Block(expr_block) = value_expr {
                                    let stmts = &expr_block.block.stmts;
                                    return Some((key, quote! { #(#stmts)* }));
                                }
                                // Otherwise, return the expression as a TokenStream
                                return Some((key, quote! { #value_expr }));
                            }
                            None
                        } else {
                            None
                        }
                    })
                    .collect();

                if tag_name == "span" {
                    Self::Inline(InlineNode { children })
                } else {
                    Self::Element(ElementNode {
                        children,
                        attributes,
                    })
                }
            }
            _ => todo!("Unsupported node type"),
        }
    }
}

impl ToTokens for TextNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let value = &self.value;
        tokens.extend(quote! {
            ::bevy_ui::TextNode::new(#value)
        });
    }
}

impl ElementNode {
    fn kebab_to_snake(s: &str) -> String {
        s.replace('-', "_")
    }

    fn get_attr<'a>(
        attributes: &'a HashMap<String, TokenStream>,
        kebab_name: &str,
    ) -> Option<&'a TokenStream> {
        attributes.get(kebab_name)
    }
    fn parse_css_value(value: &TokenStream) -> Option<TokenStream> {
        // Try to parse as a string literal (CSS-style values)
        let value_str = value.to_string();

        // Check if this is a quoted string (CSS-style value)
        if value_str.starts_with('"') && value_str.ends_with('"') {
            // Remove quotes
            let value_str = value_str.trim_matches('"');

            if let Some(px_value) = value_str.strip_suffix("px") {
                let num =
                    syn::LitFloat::new(&format!("{}.0", px_value), proc_macro2::Span::call_site());
                Some(quote! { ::bevy_ui::px(#num) })
            } else if let Some(percent_value) = value_str.strip_suffix("%") {
                let num = syn::LitFloat::new(
                    &format!("{}.0", percent_value),
                    proc_macro2::Span::call_site(),
                );
                Some(quote! { ::bevy_ui::percent(#num) })
            } else if let Some(vw_value) = value_str.strip_suffix("vw") {
                let num =
                    syn::LitFloat::new(&format!("{}.0", vw_value), proc_macro2::Span::call_site());
                Some(quote! { ::bevy_ui::vw(#num) })
            } else if let Some(vh_value) = value_str.strip_suffix("vh") {
                let num =
                    syn::LitFloat::new(&format!("{}.0", vh_value), proc_macro2::Span::call_site());
                Some(quote! { ::bevy_ui::vh(#num) })
            } else if let Some(vmin_value) = value_str.strip_suffix("vmin") {
                let num = syn::LitFloat::new(
                    &format!("{}.0", vmin_value),
                    proc_macro2::Span::call_site(),
                );
                Some(quote! { ::bevy_ui::vmin(#num) })
            } else if let Some(vmax_value) = value_str.strip_suffix("vmax") {
                let num = syn::LitFloat::new(
                    &format!("{}.0", vmax_value),
                    proc_macro2::Span::call_site(),
                );
                Some(quote! { ::bevy_ui::vmax(#num) })
            } else {
                None
            }
        } else {
            // Not a CSS-style value, assume it's a Rust expression (e.g., px(10.0))
            // Use the value directly without prepending namespace
            Some(value.clone())
        }
    }

    fn parse_enum_value(value: &TokenStream) -> Option<TokenStream> {
        // Check if this is a quoted string (CSS-style enum value)
        let value_str = value.to_string();
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
            Some(value.clone())
        }
    }

    fn parse_numeric_value(value: &TokenStream) -> Option<TokenStream> {
        let value_str = value.to_string();
        // Check if this is a quoted string
        if value_str.starts_with('"') && value_str.ends_with('"') {
            let value_str = value_str.trim_matches('"');
            // Try to parse as a number
            if let Ok(num) = value_str.parse::<f32>() {
                let lit = syn::LitFloat::new(&num.to_string(), proc_macro2::Span::call_site());
                return Some(quote! { #lit });
            }
            None
        } else {
            // Not a CSS-style value, assume it's a Rust expression
            Some(value.clone())
        }
    }

    fn build_spacing_chain(
        attributes: &HashMap<String, TokenStream>,
        property: &str,
    ) -> Option<TokenStream> {
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
                attributes.get(&key).and_then(Self::parse_css_value)
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
        let children = &self.children;
        let node_tokens = if self.attributes.is_empty() {
            quote! { ::bevy_ui::Node::default() }
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
            if let Some(value) = Self::get_attr(&self.attributes, "flex-basis")
                .and_then(Self::parse_css_value)
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
                if let Some(value) = Self::get_attr(&self.attributes, prop)
                    .and_then(Self::parse_numeric_value)
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
                && let Some(parsed) = Self::parse_numeric_value(value) {
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

            quote! {
                ::bevy_ui::Node {
                    #(#fields,)*
                    ..default()
                }
            }
        };

        tokens.extend(quote! {
            (
                #node_tokens,
                ::bevy_ecs::hierarchy::Children::spawn(
                    #(::bevy_ecs::spawn::Spawn(#children)),*
                )
            )
        });
    }
}

impl ToTokens for InlineNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let children = &self.children;
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
        }
    }
}

pub fn html_inner(input: TokenStream) -> TokenStream {
    let node_tree = parse2(input).unwrap();
    let mut output = TokenStream::new();
    eprintln!("Node tree: {:#?}", node_tree);

    for node in node_tree.into_iter() {
        if let Node::Element(_) = node {
            let html_node = HtmlNode::from(node);
            html_node.to_tokens(&mut output);
        }
    }

    output
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
                ::bevy_ui::Node::default(),
                ::bevy_ecs::hierarchy::Children::spawn(
                    ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("Hello"))
                )
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
                ::bevy_ui::Node::default(),
                ::bevy_ecs::hierarchy::Children::spawn(
                    ::bevy_ecs::spawn::Spawn(
                        (
                            ::bevy_ui::Node::default(),
                            ::bevy_ecs::hierarchy::Children::spawn(
                                ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("Hello"))
                            )
                        )
                    ),
                    ::bevy_ecs::spawn::Spawn(
                        (
                            ::bevy_ui::Node::default(),
                            ::bevy_ecs::hierarchy::Children::spawn(
                                ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("World"))
                            )
                        )
                    )
                )
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
                ::bevy_ui::Node::default(),
                ::bevy_ecs::hierarchy::Children::spawn(
                    ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("Hello")),
                    ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("World"))
                )
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
                ::bevy_ui::Node {
                  padding: ::bevy_ui::px(10.0).all().with_bottom(::bevy_ui::percent(20.0)),
                  margin: ::bevy_ui::vw(5.0).top().with_right(::bevy_ui::vmax(20.0)).with_bottom(::bevy_ui::vmin(15.0)).with_left(::bevy_ui::vh(10.0)),
                  ..default()
                },
                ::bevy_ecs::hierarchy::Children::spawn(
                    ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("Hello"))
                )
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
                ::bevy_ui::Node {
                  padding: px(10.0).all().with_bottom(percent(20.0)),
                  margin: vw(5.0).top().with_right(vmax(20.0)).with_bottom(vmin(15.0)).with_left(vh(10.0)),
                  ..default()
                },
                ::bevy_ecs::hierarchy::Children::spawn(
                    ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("Hello"))
                )
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
                ::bevy_ui::Node {
                  left: ::bevy_ui::px(5.0),
                  top: ::bevy_ui::px(10.0),
                  width: ::bevy_ui::px(100.0),
                  height: ::bevy_ui::px(50.0),
                  min_width: ::bevy_ui::px(10.0),
                  max_width: ::bevy_ui::px(200.0),
                  ..default()
                },
                ::bevy_ecs::hierarchy::Children::spawn(
                    ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("Test"))
                )
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
                ::bevy_ui::Node {
                  display: Display::Flex,
                  flex_direction: FlexDirection::Column,
                  align_items: AlignItems::Center,
                  justify_content: JustifyContent::SpaceBetween,
                  flex_grow: 1.0,
                  flex_shrink: 0.5,
                  ..default()
                },
                ::bevy_ecs::hierarchy::Children::spawn(
                    ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("Flex"))
                )
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
                ::bevy_ui::Node {
                  border: ::bevy_ui::px(2.0).all().with_top(::bevy_ui::px(5.0)),
                  row_gap: ::bevy_ui::px(10.0),
                  column_gap: ::bevy_ui::px(15.0),
                  ..default()
                },
                ::bevy_ecs::hierarchy::Children::spawn(
                    ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("Borders"))
                )
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
                ::bevy_ui::Node {
                  width: ::bevy_ui::percent(100.0),
                  position_type: PositionType::Absolute,
                  aspect_ratio: Some(1.77),
                  ..default()
                },
                ::bevy_ecs::hierarchy::Children::spawn(
                    ::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new("Aspect"))
                )
            )
        };
        let result = html_inner(input);
        assert_eq!(result.to_string(), output.to_string());
    }
}
