use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, Observer, PushSomeTokens, Value};

#[derive(Clone, Debug)]
pub struct Radio {
    attributes: Vec<Attribute>,
    bsn: bool,
}

impl Radio {
    const KEYS: [&'static str; 2] = ["components", "label"];

    pub fn new(attributes: &[Attribute], bsn: bool) -> Self {
        Self {
            attributes: attributes
                .iter()
                .filter(|attr| {
                    Self::KEYS.contains(&attr.key.as_str()) || attr.key.starts_with("on")
                })
                .cloned()
                .collect(),
            bsn,
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
                if self.bsn {
                    quote! {
                        bevy::ui::widget::Text(#value)
                    }
                } else {
                    quote! {
                        bevy::ecs::spawn::Spawn(bevy::ui::widget::Text::new(#value))
                    }
                }
            })
            .collect::<Vec<_>>();
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
                @bevy::feathers::controls::FeathersRadio {
                    @caption: bsn_list![
                        #(#children),*
                    ]
                }
                #observer
            });
        } else {
            children.push_some(observer);
            tokens.extend(quote! {
                bevy::feathers::controls::radio_bundle(
                    #components,
                    (
                        #(#children),*
                    )
                )
            });
        }
    }
}
