use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, Value};
#[derive(Clone, Debug)]
pub struct Image {
    attributes: Vec<Attribute>,
    bsn: bool,
}

impl Image {
    const KEYS: [&'static str; 1] = ["src"];

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

    pub fn ok(self) -> Option<Self> {
        if self.attributes.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl ToTokens for Image {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = Value::new(&self.attributes[0].value);
        if let Some(name) = name.clean_block() {
            if self.bsn {
                // Braced so `bsn!` reads the value as an opaque Rust expression.
                tokens.extend(quote! {
                    bevy::ui::widget::ImageNode {
                        image: {#name}
                    }
                })
            } else {
                tokens.extend(quote! {
                    bevy::ui::widget::ImageNode::new(#name)
                })
            }
        }
    }
}
