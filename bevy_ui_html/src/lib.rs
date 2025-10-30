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
                let num =
                    syn::LitFloat::new(&format!("{}.0", vmin_value), proc_macro2::Span::call_site());
                Some(quote! { ::bevy_ui::vmin(#num) })
            } else if let Some(vmax_value) = value_str.strip_suffix("vmax") {
                let num =
                    syn::LitFloat::new(&format!("{}.0", vmax_value), proc_macro2::Span::call_site());
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
                attributes.get(&key).and_then(|v| Self::parse_css_value(v))
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
}
