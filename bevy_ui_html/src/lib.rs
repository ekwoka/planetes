use proc_macro2::TokenStream;
use quote::quote;
use rstml::{node::Node, parse2};

pub fn html_inner(input: TokenStream) -> TokenStream {
    let node_tree = parse2(input).unwrap();
    let mut output = TokenStream::new();
    eprintln!("Node tree: {:#?}", node_tree);
    for node in node_tree.into_iter() {
        if let Node::Element(element) = node {
            let children = element
                .children
                .into_iter()
                .filter_map(|child| match child {
                    Node::Text(text) => Some(text.value_string()),
                    _ => None,
                });
            output.extend(quote! {
                (
                    ::bevy_ui::Node::default(),
                    ::bevy_ecs::hierarchy::Children::spawn(
                        #(::bevy_ecs::spawn::Spawn(::bevy_ui::TextNode::new(#children))),*
                    )
                )
            })
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
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
}
