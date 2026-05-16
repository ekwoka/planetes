use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, Observer, PushSomeTokens, Value};

#[derive(Clone, Debug)]
pub struct Radio {
    attributes: Vec<Attribute>,
}

impl Radio {
    const KEYS: [&'static str; 2] = ["components", "label"];
}

impl From<&Vec<Attribute>> for Radio {
    fn from(attributes: &Vec<Attribute>) -> Self {
        Self {
            attributes: attributes
                .iter()
                .filter(|attr| {
                    Self::KEYS.contains(&attr.key.as_str()) || attr.key.starts_with("on")
                })
                .cloned()
                .collect(),
        }
    }
}

impl ToTokens for Radio {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut children = self
            .attributes
            .iter()
            .filter(|attr| attr.key == "label")
            .map(|attr| {
                let value = attr.value.clone();
                quote! {
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new(#value))
                }
            })
            .collect::<Vec<_>>();
        children.push_some(Observer::from(&self.attributes).ok());
        let components = self
            .attributes
            .iter()
            .find(|attr| attr.key == "components")
            .and_then(|attr| Value::new(&attr.value).clean_block())
            .unwrap_or_else(|| quote! { () });
        tokens.extend(quote! {
            ::bevy::feathers::controls::radio(
                #components,
                (
                    #(#children),*
                )
            )
        });
    }
}
