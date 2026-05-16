use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, Value};
#[derive(Clone, Debug)]
pub struct Name {
    attributes: Vec<Attribute>,
}

impl Name {
    const KEYS: [&'static str; 1] = ["name"];

    pub fn ok(self) -> Option<Self> {
        if self.attributes.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl From<&Vec<Attribute>> for Name {
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

impl ToTokens for Name {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = Value::new(&self.attributes[0].value);
        if let Some(name) = name.clean_block() {
            tokens.extend(quote! {
                ::bevy::ecs::name::Name::new(#name)
            })
        }
    }
}
