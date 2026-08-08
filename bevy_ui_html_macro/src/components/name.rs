use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, Value};
#[derive(Clone, Debug)]
pub struct Name {
    attributes: Vec<Attribute>,
    bsn: bool,
}

impl Name {
    const KEYS: [&'static str; 1] = ["name"];

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

impl ToTokens for Name {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = Value::new(&self.attributes[0].value);
        if self.bsn {
            if let Some(name) = name.as_ident() {
                let hash = quote! { # };
                tokens.extend(quote! {
                    #hash #name
                })
            } else if let Some(name) = name.clean_block() {
                tokens.extend(quote! {
                    bevy::ecs::name::Name(#name)
                })
            }
        } else if let Some(name) = name.clean_block() {
            tokens.extend(quote! {
                ::bevy::ecs::name::Name::new(#name)
            })
        }
    }
}
