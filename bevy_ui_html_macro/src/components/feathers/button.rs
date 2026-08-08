use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, ChildNode, Observer, PushSomeTokens, Value};

#[derive(Clone, Debug)]
pub struct Button {
    attributes: Vec<Attribute>,
    children: Option<Vec<ChildNode>>,
    bsn: bool,
}

impl Button {
    const KEYS: [&'static str; 3] = ["variant", "corners", "components"];

    pub fn new(attributes: &[Attribute], bsn: bool) -> Self {
        Self {
            attributes: attributes
                .iter()
                .filter(|attr| {
                    Self::KEYS.contains(&attr.key.as_str()) || attr.key.starts_with("on")
                })
                .cloned()
                .collect(),
            children: None,
            bsn,
        }
    }

    pub fn with_children(self, children: Vec<ChildNode>) -> Self {
        Self {
            children: Some(children),
            ..self
        }
    }
}

impl ToTokens for Button {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut children = self
            .children
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(|child| child.to_token_stream())
            .collect::<Vec<_>>();
        let props = ButtonProps::new(&self.attributes, self.bsn);
        let components = self
            .attributes
            .iter()
            .find(|attr| attr.key == "components")
            .and_then(|attr| Value::new(&attr.value).clean_block())
            .unwrap_or_else(|| {
                if self.bsn {
                    quote! {}
                } else {
                    quote! { () }
                }
            });
        let observer = Observer::new(&self.attributes, self.bsn).ok();
        if self.bsn {
            tokens.extend(quote! {
                #components
                @::bevy::feathers::controls::FeathersButton {
                    #props,
                    @caption: bsn_list![
                        #(#children),*
                    ]
                }
                #observer
            });
        } else {
            children.push_some(observer);
            tokens.extend(quote! {
                ::bevy::feathers::controls::button_bundle(
                    #props,
                    #components,
                    (
                        #(#children),*
                    )
                )
            });
        }
    }
}

#[derive(Clone, Debug)]
pub struct ButtonProps {
    attributes: Vec<Attribute>,
    bsn: bool,
}

impl ButtonProps {
    const KEYS: [&'static str; 2] = ["variant", "corners"];

    pub fn new(attributes: &[Attribute], bsn: bool) -> Self {
        Self {
            attributes: attributes
                .iter()
                .filter(|attr| Self::KEYS.contains(&attr.key.as_str()))
                .cloned()
                .collect(),
            bsn,
        }
    }
}

impl ToTokens for ButtonProps {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if self.attributes.is_empty() {
            tokens.extend(quote! { ::bevy::feathers::controls::ButtonBundleProps::default() })
        } else {
            let fields = self
                .attributes
                .iter()
                .map(|attr| {
                    let field = attr.key.clone();
                    let value = attr.value.clone();
                    let value_tokens = if let syn::Expr::Block(expr_block) = value {
                        let stmts = &expr_block.block.stmts;
                        quote! { #(#stmts)* }
                    } else {
                        quote! { #value }
                    };

                    // Try to parse as a string literal (CSS-style values)
                    let value_str = value_tokens.to_string().trim_matches('"').to_string();
                    let value = match field.as_str() {
                        "variant" => {
                            let value = match value_str.as_str() {
                                "normal" | "Normal" => {
                                    syn::Ident::new("Normal", proc_macro2::Span::call_site())
                                }
                                "primary" | "Primary" => {
                                    syn::Ident::new("Primary", proc_macro2::Span::call_site())
                                }
                                &_ => unreachable!(),
                            };
                            quote! {
                                ::bevy::feathers::controls::ButtonVariant::#value
                            }
                        }
                        "corners" => {
                            let value = match value_str.as_str() {
                                "all" | "All" | "rounded" | "Rounded" => {
                                    syn::Ident::new("All", proc_macro2::Span::call_site())
                                }
                                "top" | "Top" => {
                                    syn::Ident::new("Top", proc_macro2::Span::call_site())
                                }
                                "bottom" | "Bottom" => {
                                    syn::Ident::new("Bottom", proc_macro2::Span::call_site())
                                }
                                "left" | "Left" => {
                                    syn::Ident::new("Left", proc_macro2::Span::call_site())
                                }
                                "right" | "Right" => {
                                    syn::Ident::new("Right", proc_macro2::Span::call_site())
                                }
                                "topleft" | "TopLeft" | "top left" => {
                                    syn::Ident::new("TopLeft", proc_macro2::Span::call_site())
                                }
                                "topright" | "TopRight" | "top right" => {
                                    syn::Ident::new("TopRight", proc_macro2::Span::call_site())
                                }
                                "bottomleft" | "BottomLeft" | "bottom left" => {
                                    syn::Ident::new("BottomLeft", proc_macro2::Span::call_site())
                                }
                                "bottomright" | "BottomRight" | "bottom right" => {
                                    syn::Ident::new("BottomRight", proc_macro2::Span::call_site())
                                }
                                "none" | "None" => {
                                    syn::Ident::new("None", proc_macro2::Span::call_site())
                                }
                                &_ => unreachable!(),
                            };
                            quote! {
                                ::bevy::feathers::rounded_corners::RoundedCorners::#value
                            }
                        }
                        &_ => unreachable!(),
                    };
                    let field_name = syn::Ident::new(&field, proc_macro2::Span::call_site());
                    quote! {
                        #field_name: #value
                    }
                })
                .collect::<Vec<_>>();

            if self.bsn {
                tokens.extend(quote! {
                    #(@#fields),*
                });
            } else {
                tokens.extend(quote! {
                    ::bevy::feathers::controls::ButtonBundleProps {
                        #(#fields,)*
                        ..Default::default()
                    }
                });
            }
        }
    }
}
