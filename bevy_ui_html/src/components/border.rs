use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, ExprBlock};

use crate::{Attribute, Value};

#[derive(Clone, Debug)]
pub struct BorderRadius {
    attributes: Vec<Attribute>,
}

impl BorderRadius {
    const KEYS: [&'static str; 1] = ["border-radius"];

    pub fn ok(self) -> Option<Self> {
        if self.attributes.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl From<&Vec<Attribute>> for BorderRadius {
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

impl ToTokens for BorderRadius {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let radius = Value::new(&self.attributes[0].value);

        if let Some(radius) = radius.parse_as_css_value() {
            tokens.extend(quote! {
                ::bevy::ui::BorderRadius::all(#radius)
            })
        };
    }
}

#[derive(Clone, Debug)]
pub struct BorderColor {
    attributes: Vec<Attribute>,
}

impl BorderColor {
    const KEYS: [&'static str; 1] = ["border-color"];

    pub fn ok(self) -> Option<Self> {
        if self.attributes.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl From<&Vec<Attribute>> for BorderColor {
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

impl ToTokens for BorderColor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let color = &self.attributes[0].value;

        if let Expr::Block(ExprBlock { block, .. }) = color {
            if block.stmts.len() == 1 {
                let stmt = &block.stmts[0];
                tokens.extend(quote! {
                    ::bevy::ui::BorderColor::all(#stmt)
                })
            } else {
                tokens.extend(quote! {
                    ::bevy::ui::BorderColor::all(#block)
                })
            }
        }
    }
}
