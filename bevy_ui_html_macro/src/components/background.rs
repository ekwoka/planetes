use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, Value};
#[derive(Clone, Debug)]
pub struct BackgroundColor {
    attributes: Vec<Attribute>,
}

impl BackgroundColor {
    const KEYS: [&'static str; 1] = ["background-color"];

    pub fn ok(self) -> Option<Self> {
        if self.attributes.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl From<&Vec<Attribute>> for BackgroundColor {
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

impl ToTokens for BackgroundColor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let color = Value::new(&self.attributes[0].value);

        if let Some(color) = color.parse_as_color() {
            tokens.extend(quote! {
                bevy::ui::BackgroundColor(#color)
            })
        }
    }
}
