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

        if value_str == "black" {
            return Some(quote! { Color::BLACK });
        }

        if value_str == "white" {
            return Some(quote! { Color::WHITE });
        }

        if value_str == "none" {
            return Some(quote! { Color::NONE });
        }

        if let Some(values) = value_str.strip_prefix("rgba") {
            let (rgb, a) = values
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_once(" / ")
                .unwrap();
            let rgb = rgb
                .split_whitespace()
                .filter_map(|value| value.parse::<f32>().ok())
                .map(|value| value / 255.0)
                .map(|value| syn::LitFloat::new(&format!("{value}"), value.span()));
            let alpha = a
                .parse::<f32>()
                .ok()
                .map(|a| syn::LitFloat::new(&format!("{a}"), value.span()));
            return Some(quote! { Color::linear_rgba(#(#rgb),*, #alpha) });
        } else if let Some(values) = value_str.strip_prefix("rgb") {
            let values = values
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_whitespace()
                .filter_map(|value| value.parse::<f32>().ok())
                .map(|value| value / 255.0)
                .map(|value| syn::LitFloat::new(&format!("{value}"), value.span()));
            return Some(quote! { Color::linear_rgb(#(#values),*) });
        } else if let Some(values) = value_str.strip_prefix("srgba") {
            let (rgb, alpha) = values
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_once(" / ")
                .unwrap();

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
                let alpha =
                    alpha.map(|value| syn::LitFloat::new(&format!("{value}"), value.span()));
                return Some(quote! { Color::srgba(#(#rgb),*, #alpha) });
            } else {
                let rgb = rgb.map(|value| syn::LitInt::new(&format!("{value}"), value.span()));
                let alpha = alpha
                    .map(|a| (a * 255.0).trunc())
                    .map(|value| syn::LitInt::new(&format!("{value}"), value.span()));
                return Some(quote! { Color::srgba_u8(#(#rgb),*, #alpha) });
            }
        } else if let Some(values) = value_str.strip_prefix("srgb") {
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
                return Some(quote! { Color::srgb(#(#values),*) });
            } else {
                let values =
                    values.map(|value| syn::LitInt::new(&format!("{value}"), value.span()));
                return Some(quote! { Color::srgb_u8(#(#values),*) });
            }
        }
        return None;
    } else {
        // Not a CSS-style value, assume it's a Rust expression
        return Some(value_tokens);
    }
}
