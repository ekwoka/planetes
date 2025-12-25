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
