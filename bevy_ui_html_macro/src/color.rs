use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, spanned::Spanned};

pub fn to_color(value: &Expr) -> Option<TokenStream> {
    let value_tokens = if let syn::Expr::Block(expr_block) = value {
        let stmts = &expr_block.block.stmts;
        quote! { #(#stmts)* }
    } else {
        quote! { #value }
    };

    let value_str = value_tokens.to_string();
    // Check if this is a quoted string
    if value_str.starts_with('"') && value_str.ends_with('"') {
        let value_str = value_str.trim_matches('"');

        if value_str.contains('(') {
            let (colorspace, values) = value_str.trim_end_matches(')').split_once('(').unwrap();
            let colorspace = colorspace.trim_end_matches('a');
            match colorspace {
                "rgb" => {
                    if let Some((rgb, alpha)) = values.split_once(" / ") {
                        let rgb = rgb
                            .split_whitespace()
                            .filter_map(|value| value.parse::<f32>().ok())
                            .map(|value| value / 255.0)
                            .map(|value| syn::LitFloat::new(&format!("{value}"), value.span()));
                        let alpha = alpha
                            .parse::<f32>()
                            .ok()
                            .map(|a| syn::LitFloat::new(&format!("{a}"), value.span()));
                        return Some(quote! { bevy::color::Color::linear_rgba(#(#rgb),*, #alpha) });
                    } else {
                        let values = values
                            .split_whitespace()
                            .filter_map(|value| value.parse::<f32>().ok())
                            .map(|value| value / 255.0)
                            .map(|value| syn::LitFloat::new(&format!("{value}"), value.span()));
                        return Some(quote! { bevy::color::Color::linear_rgb(#(#values),*) });
                    }
                }
                "srgb" => {
                    if let Some((rgb, alpha)) = values.split_once(" / ") {
                        let rgb = rgb
                            .split_whitespace()
                            .filter_map(|value| value.parse::<f32>().ok());
                        let alpha = alpha.parse::<f32>().ok();
                        if rgb
                            .clone()
                            .any(|maybe_float| maybe_float.trunc() != maybe_float)
                        {
                            let rgb = rgb
                                .map(|value| value / 255.0)
                                .map(|value| syn::LitFloat::new(&format!("{value}"), value.span()));
                            let alpha = alpha
                                .map(|value| syn::LitFloat::new(&format!("{value}"), value.span()));
                            return Some(quote! { bevy::color::Color::srgba(#(#rgb),*, #alpha) });
                        } else {
                            let rgb = rgb
                                .map(|value| syn::LitInt::new(&format!("{value}"), value.span()));
                            let alpha = alpha
                                .map(|a| (a * 255.0).trunc())
                                .map(|value| syn::LitInt::new(&format!("{value}"), value.span()));
                            return Some(
                                quote! { bevy::color::Color::srgba_u8(#(#rgb),*, #alpha) },
                            );
                        }
                    } else {
                        let values = values
                            .trim_start_matches('(')
                            .trim_end_matches(')')
                            .split_whitespace()
                            .filter_map(|value| value.parse::<f32>().ok());

                        if values
                            .clone()
                            .any(|maybe_float| maybe_float.trunc() != maybe_float)
                        {
                            let values = values
                                .map(|value| value / 255.0)
                                .map(|value| syn::LitFloat::new(&format!("{value}"), value.span()));
                            return Some(quote! { bevy::color::Color::srgb(#(#values),*) });
                        } else {
                            let values = values
                                .map(|value| syn::LitInt::new(&format!("{value}"), value.span()));
                            return Some(quote! { bevy::color::Color::srgb_u8(#(#values),*) });
                        }
                    }
                }
                "hsl" | "hsv" | "hwb" | "lab" | "lch" | "oklab" | "oklch" | "xyz" => {
                    if let Some((rgb, alpha)) = values.split_once(" / ") {
                        let rgb = rgb
                            .split_whitespace()
                            .filter_map(|value| value.parse::<f32>().ok())
                            .map(|value| format!("{value}"))
                            .map(|mut float| {
                                if float.contains('.') {
                                    syn::LitFloat::new(&float, value.span())
                                } else {
                                    float.extend(".0".chars());
                                    syn::LitFloat::new(&float, value.span())
                                }
                            });
                        let alpha = alpha
                            .parse::<f32>()
                            .ok()
                            .map(|a| syn::LitFloat::new(&format!("{a}"), value.span()));
                        let colorspace = syn::Ident::new(&format!("{colorspace}a"), value.span());
                        return Some(quote! { bevy::color::Color::#colorspace(#(#rgb),*, #alpha) });
                    } else {
                        let values = values
                            .split_whitespace()
                            .filter_map(|value| value.parse::<f32>().ok())
                            .map(|value| format!("{value}"))
                            .map(|mut float| {
                                if float.contains('.') {
                                    syn::LitFloat::new(&float, value.span())
                                } else {
                                    float.extend(".0".chars());
                                    syn::LitFloat::new(&float, value.span())
                                }
                            });
                        let colorspace = syn::Ident::new(colorspace, value.span());
                        return Some(quote! { bevy::color::Color::#colorspace(#(#values),*) });
                    }
                }
                _ => {
                    return None;
                }
            };
        } else {
            return match value_str {
                "black" | "BLACK" => Some(quote! { bevy::color::Color::BLACK }),
                "white" | "WHITE" => Some(quote! { bevy::color::Color::WHITE }),
                "none" | "NONE" => Some(quote! { bevy::color::Color::NONE }),
                _ => None,
            };
        }
    } else {
        // Not a CSS-style value, assume it's a Rust expression
        return Some(value_tokens);
    }
}
