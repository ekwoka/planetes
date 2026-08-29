use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, Value};

#[derive(Clone, Debug)]
pub struct TextFont {
    attributes: Vec<Attribute>,
    bsn: bool,
}

impl TextFont {
    const KEYS: [&'static str; 1] = ["font-size"];

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

impl TextFont {
    /// Returns tokens for the plain `bevy::text::TextFont { … }` struct,
    /// without any `Propagate` wrapping. Used when populating `HtmlBundle`
    /// for custom component `build()` calls.
    pub fn plain_tokens(&self) -> TokenStream {
        let fields = self
            .attributes
            .iter()
            .filter_map(|attr| match attr.key.as_str() {
                "font-size" => Value::new(&attr.value).parse_as_float().map(|value| {
                    quote! { font_size: bevy::text::FontSize::Px(#value) }
                }),
                _ => None,
            });
        quote! {
            bevy::text::TextFont {
                #(#fields,)*
                ..Default::default()
            }
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
                        font_size: bevy::text::FontSize::Px(#value)
                    }
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        if self.bsn {
            if cfg!(feature = "propagate") {
                tokens.extend(quote! {
                        template(|_| Ok(
                            bevy::app::Propagate(
                                bevy::text::TextFont {
                                    #(#fields,)*
                                    ..Default::default()
                                }
                            )
                        ))
                        bevy::text::TextFont {
                            #(#fields),*
                        }
                })
            } else {
                tokens.extend(quote! {
                    bevy::text::TextFont {
                        #(#fields),*
                    }
                });
            }
        } else if cfg!(feature = "propagate") {
            tokens.extend(quote! {
                    bevy::app::Propagate(bevy::text::TextFont {
                    #(#fields,)*
                    ..Default::default()
                })
            })
        } else {
            tokens.extend(quote! {
                bevy::text::TextFont {
                    #(#fields,)*
                    ..Default::default()
                }
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextLayout {
    attributes: Vec<Attribute>,
    bsn: bool,
}

impl TextLayout {
    const KEYS: [&'static str; 2] = ["justify", "linebreak"];

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

impl ToTokens for TextLayout {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let bsn = self.bsn;
        let fields = self.attributes.iter().map(|attr| {
            let key = syn::Ident::new(&attr.key, attr.span);
            let value = Value::new(&attr.value).clean_block();
            if bsn {
                // Braced so `bsn!` reads the value as an opaque Rust expression rather than
                // trying to build an enum variant patch out of it.
                quote! {
                    #key: {#value}
                }
            } else {
                quote! {
                    #key: #value
                }
            }
        });

        if self.bsn {
            tokens.extend(quote! {
                bevy::text::TextLayout {
                    #(#fields),*
                }
            });
        } else {
            tokens.extend(quote! {
                bevy::text::TextLayout {
                    #(#fields,)*
                    ..Default::default()
                }
            });
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextColor {
    attributes: Vec<Attribute>,
    bsn: bool,
}

impl TextColor {
    const KEYS: [&'static str; 1] = ["text-color"];

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

impl TextColor {
    /// Returns tokens for the plain `bevy::text::TextColor(…)` value,
    /// without any `Propagate` wrapping. Used when populating `HtmlBundle`
    /// for custom component `build()` calls.
    pub fn plain_tokens(&self) -> TokenStream {
        let color = Value::new(&self.attributes[0].value);
        if let Some(color) = color.parse_as_color() {
            quote! { bevy::text::TextColor(#color) }
        } else {
            quote! { bevy::text::TextColor::default() }
        }
    }
}

impl ToTokens for TextColor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let color = Value::new(&self.attributes[0].value);

        if let Some(color) = color.parse_as_color() {
            if self.bsn {
                if cfg!(feature = "propagate") {
                    tokens.extend(quote! {
                        template(|_| Ok(
                            bevy::app::Propagate(bevy::text::TextColor(#color))
                        ))
                        bevy::text::TextColor(#color)
                    })
                } else {
                    tokens.extend(quote! {
                        bevy::text::TextColor(#color)
                    });
                }
            } else if cfg!(feature = "propagate") {
                tokens.extend(quote! {
                        bevy::app::Propagate(bevy::text::TextColor(#color))
                })
            } else {
                tokens.extend(quote! {
                    bevy::text::TextColor(#color)
                })
            }
        }
    }
}
