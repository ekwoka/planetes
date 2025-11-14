use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, Value};
#[derive(Clone, Debug)]
pub struct Observer {
    attributes: Vec<Attribute>,
}

impl Observer {
    pub fn ok(self) -> Option<Self> {
        if self.attributes.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl From<&Vec<Attribute>> for Observer {
    fn from(attributes: &Vec<Attribute>) -> Self {
        Self {
            attributes: attributes
                .iter()
                .filter(|attr| attr.key.as_str().starts_with("on"))
                .cloned()
                .collect(),
        }
    }
}

impl ToTokens for Observer {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let observers = self
            .attributes
            .iter()
            .filter_map(|attr| Value::new(&attr.value).clean_block());
        tokens.extend(quote! {
            ::bevy::ecs::spawn::SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
                let entity = parent.target_entity();
                #(parent.spawn(::bevy::ecs::observer::Observer::new(#observers).with_entity(entity));)*
            })
        });
    }
}
