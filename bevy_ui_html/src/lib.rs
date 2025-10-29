use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use rstml::{node::Node, parse2};

#[derive(Debug)]
enum HtmlNode {
    Text(TextNode),
    Element(ElementNode),
}

#[derive(Debug)]
struct TextNode {
    value: String,
}

#[derive(Debug)]
struct ElementNode {
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

                Self::Element(ElementNode { children })
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

impl ToTokens for ElementNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let children = &self.children;
        tokens.extend(quote! {
            (
                ::bevy_ui::Node::default(),
                ::bevy_ecs::hierarchy::Children::spawn(
                    #(::bevy_ecs::spawn::Spawn(#children)),*
                )
            )
        });
    }
}

impl ToTokens for HtmlNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            HtmlNode::Text(text) => text.to_tokens(tokens),
            HtmlNode::Element(element) => element.to_tokens(tokens),
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
}
