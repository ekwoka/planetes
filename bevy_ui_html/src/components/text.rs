use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, Value};

#[derive(Clone, Debug)]
pub struct TextFont {
    attributes: Vec<Attribute>,
}

impl TextFont {
    const KEYS: [&'static str; 1] = ["font-size"];

    pub fn ok(self) -> Option<Self> {
        if self.attributes.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl From<&Vec<Attribute>> for TextFont {
    fn from(attributes: &Vec<Attribute>) -> Self {
        Self {
            attributes: attributes
                .iter()
                .filter(|attr| Self::KEYS.contains(&attr.key.as_str()))
                .cloned()
                .collect(),
        }
    }
}

impl ToTokens for TextFont {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let fields = self
            .attributes
            .iter()
            .filter_map(|attr| match attr.key.as_str() {
                "font-size" => Value::new(&attr.value).parse_as_float().map(|value| {
                    quote! {
                        font_size: #value
                    }
                }),
                _ => None,
            });

        if cfg!(feature = "propagate") {
            tokens.extend(quote! {
                    ::bevy::app::Propagate(::bevy::text::TextFont {
                    #(#fields,)*
                    ..Default::default()
                })
            })
        } else {
            tokens.extend(quote! {
                ::bevy::text::TextFont {
                    #(#fields,)*
                    ..Default::default()
                }
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextLayout {
    attributes: Vec<Attribute>,
}

impl TextLayout {
    const KEYS: [&'static str; 2] = ["justify", "linebreak"];

    pub fn ok(self) -> Option<Self> {
        if self.attributes.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl From<&Vec<Attribute>> for TextLayout {
    fn from(attributes: &Vec<Attribute>) -> Self {
        Self {
            attributes: attributes
                .iter()
                .filter(|attr| Self::KEYS.contains(&attr.key.as_str()))
                .cloned()
                .collect(),
        }
    }
}

impl ToTokens for TextLayout {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let fields = self.attributes.iter().map(|attr| {
            let key = syn::Ident::new(&attr.key, attr.span);
            let value = Value::new(&attr.value).clean_block();
            quote! {
                #key: #value
            }
        });

        if cfg!(feature = "propagate") {
            tokens.extend(quote! {
                    ::bevy::app::Propagate(::bevy::text::TextLayout {
                    #(#fields,)*
                    ..Default::default()
                })
            })
        } else {
            tokens.extend(quote! {
                ::bevy::text::TextLayout {
                    #(#fields,)*
                    ..Default::default()
                }
            })
        }
    }
}
