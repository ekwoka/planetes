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
                #[cfg(feature = "bsn")]
                return quote! {
                    ::bevy::ui::widget::Text(#value)
                };
                #[cfg(not(feature = "bsn"))]
                quote! {
                    ::bevy::ecs::spawn::Spawn(::bevy::ui::widget::Text::new(#value))
                }
            })
            .collect::<Vec<_>>();
        let components = self
            .attributes
            .iter()
            .find(|attr| attr.key == "components")
            .and_then(|attr| Value::new(&attr.value).clean_block())
            .unwrap_or_else(|| {
                #[cfg(feature = "bsn")]
                return quote! {};
                #[cfg(not(feature = "bsn"))]
                return quote! { () };
            });
        let observer = Observer::from(&self.attributes).ok();
        #[cfg(not(feature = "bsn"))]
        children.push_some(observer);
        #[cfg(feature = "bsn")]
        tokens.extend(quote! {
            #components
            @::bevy::feathers::controls::FeathersRadio {
                @caption: bsn_list![
                    #(#children),*
                ]
            }
            #observer
        });
        #[cfg(not(feature = "bsn"))]
        tokens.extend(quote! {
            ::bevy::feathers::controls::radio_bundle(
                #components,
                (
                    #(#children),*
                )
            )
        });
    }
}
