use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, ExprLit, Lit};

#[derive(Clone, Debug)]
pub struct Value(Expr);

impl Value {
    pub fn new(expr: &Expr) -> Self {
        Value(expr.clone())
    }
    pub fn parse_as_css_value(&self) -> Option<TokenStream> {
        match &self.0 {
            Expr::Block(block) => Some(quote! { #block }),
            Expr::Lit(ExprLit {
                lit: Lit::Str(value),
                ..
            }) => {
                let value_string = value.to_token_stream().to_string();
                if value_string.starts_with('"') && value_string.ends_with('"') {
                    // Remove quotes
                    let value_string = value_string.trim_matches('"');

                    if let Some(px_value) = value_string.strip_suffix("px") {
                        let num = syn::LitFloat::new(
                            &format!("{}.0", px_value),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::px(#num) })
                    } else if let Some(percent_value) = value_string.strip_suffix("%") {
                        let num = syn::LitFloat::new(
                            &format!("{}.0", percent_value),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::percent(#num) })
                    } else if let Some(vw_value) = value_string.strip_suffix("vw") {
                        let num = syn::LitFloat::new(
                            &format!("{}.0", vw_value),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::vw(#num) })
                    } else if let Some(vh_value) = value_string.strip_suffix("vh") {
                        let num = syn::LitFloat::new(
                            &format!("{}.0", vh_value),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::vh(#num) })
                    } else if let Some(vmin_value) = value_string.strip_suffix("vmin") {
                        let num = syn::LitFloat::new(
                            &format!("{}.0", vmin_value),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::vmin(#num) })
                    } else if let Some(vmax_value) = value_string.strip_suffix("vmax") {
                        let num = syn::LitFloat::new(
                            &format!("{}.0", vmax_value),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::vmax(#num) })
                    } else {
                        None
                    }
                } else {
                    // Not a CSS-style value, assume it's a Rust expression (e.g., px(10.0))
                    // Use the value directly without prepending namespace
                    Some(self.0.to_token_stream())
                }
            }
            _ => None,
        }

        // Check if this is a quoted string (CSS-style value)
    }
}
