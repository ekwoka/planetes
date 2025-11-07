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

        tokens.extend(quote! {
            ::bevy::text::TextFont {
                #(#fields,)*
                ..Default::default()
            }
        })
    }
}
