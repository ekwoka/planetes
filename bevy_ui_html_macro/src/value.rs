use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, ExprBlock, ExprLit, Ident, Lit, Path, parse_quote_spanned, spanned::Spanned};

use crate::color::to_color;

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
                            &format!("{px_value}.0"),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::px(#num) })
                    } else if let Some(percent_value) = value_string.strip_suffix("%") {
                        let num = syn::LitFloat::new(
                            &format!("{percent_value}.0"),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::percent(#num) })
                    } else if let Some(vw_value) = value_string.strip_suffix("vw") {
                        let num = syn::LitFloat::new(
                            &format!("{vw_value}.0"),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::vw(#num) })
                    } else if let Some(vh_value) = value_string.strip_suffix("vh") {
                        let num = syn::LitFloat::new(
                            &format!("{vh_value}.0"),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::vh(#num) })
                    } else if let Some(vmin_value) = value_string.strip_suffix("vmin") {
                        let num = syn::LitFloat::new(
                            &format!("{vmin_value}.0"),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::vmin(#num) })
                    } else if let Some(vmax_value) = value_string.strip_suffix("vmax") {
                        let num = syn::LitFloat::new(
                            &format!("{vmax_value}.0"),
                            proc_macro2::Span::call_site(),
                        );
                        Some(quote! { ::bevy::ui::vmax(#num) })
                    } else {
                        let tokens = self.0.to_token_stream();
                        Some(quote! { ::bevy::ui::Val::from(#tokens) })
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

    pub fn parse_as_float(&self) -> Option<TokenStream> {
        let value = self.0.clone();
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
            // Try to parse as a number
            if let Ok(num) = value_str.parse::<f32>() {
                let lit = syn::LitFloat::new(
                    format!("{num:#?}").as_str(),
                    proc_macro2::Span::call_site(),
                );
                return Some(quote! { #lit });
            }
            None
        } else {
            // Not a CSS-style value, assume it's a Rust expression
            Some(value_tokens)
        }
    }

    pub fn as_display(&self) -> Option<TokenStream> {
        let known = match self.0.clone() {
            Expr::Block(_) => self.clean_block(),
            Expr::Lit(lit) => match lit.lit {
                Lit::Str(string) => {
                    let value = string.value();
                    let name = match value.as_str() {
                        "none" | "hidden" | "Hidden" => "None",
                        "flex" => "Flex",
                        "grid" => "Grid",
                        "block" => "Block",
                        other => other,
                    };
                    let ident = Ident::new(name, self.0.span());
                    let path: Path = parse_quote_spanned! {
                    self.0.span() => ::bevy::ui::Display::#ident
                    };
                    Some(path.to_token_stream())
                }
                _ => None,
            },
            _ => None,
        };
        known.or_else(|| Some(self.0.to_token_stream()))
    }

    pub fn as_flex_direction(&self) -> Option<TokenStream> {
        let known = match self.0.clone() {
            Expr::Block(_) => self.clean_block(),
            Expr::Lit(lit) => match lit.lit {
                Lit::Str(string) => {
                    let value = string.value();
                    let name = match value.as_str() {
                        "row" => "Row",
                        "column" | "col" => "Column",
                        "row-reverse" => "RowReverse",
                        "column-reverse" | "col-reverse" => "ColumnReverse",
                        other => other,
                    };
                    let ident = Ident::new(name, self.0.span());
                    let path: Path = parse_quote_spanned! {
                    self.0.span() => ::bevy::ui::FlexDirection::#ident
                    };
                    Some(path.to_token_stream())
                }
                _ => None,
            },
            _ => None,
        };
        known.or_else(|| Some(self.0.to_token_stream()))
    }

    pub fn as_justify_content(&self) -> Option<TokenStream> {
        let known = match self.0.clone() {
            Expr::Block(_) => self.clean_block(),
            Expr::Lit(lit) => match lit.lit {
                Lit::Str(string) => {
                    let value = string.value();
                    let name = match value.as_str() {
                        "default" => "Default",
                        "start" => "Start",
                        "end" => "End",
                        "flex-start" => "FlexStart",
                        "flex-end" => "FlexEnd",
                        "center" => "Center",
                        "stretch" => "Stretch",
                        "space-between" => "SpaceBetween",
                        "space-evently" => "SpaceEvenly",
                        "space-around" => "SpaceAround",
                        other => other,
                    };
                    let ident = Ident::new(name, self.0.span());
                    let path: Path = parse_quote_spanned! {
                    self.0.span() => ::bevy::ui::JustifyContent::#ident
                    };
                    Some(path.to_token_stream())
                }
                _ => None,
            },
            _ => None,
        };
        known.or_else(|| Some(self.0.to_token_stream()))
    }

    pub fn as_position_type(&self) -> Option<TokenStream> {
        let known = match self.0.clone() {
            Expr::Block(_) => self.clean_block(),
            Expr::Lit(lit) => match lit.lit {
                Lit::Str(string) => {
                    let value = string.value();
                    let name = match value.as_str() {
                        "absolute" => "Absolute",
                        "relative" => "Relative",
                        other => other,
                    };
                    let ident = Ident::new(name, self.0.span());
                    let path: Path = parse_quote_spanned! {
                    self.0.span() => ::bevy::ui::PositionType::#ident
                    };
                    Some(path.to_token_stream())
                }
                _ => None,
            },
            _ => None,
        };
        known.or_else(|| Some(self.0.to_token_stream()))
    }

    pub fn clean_block(&self) -> Option<TokenStream> {
        let value = &self.0;

        if let syn::Expr::Block(ExprBlock { block, .. }) = value {
            if block.stmts.len() == 1 {
                let stmt = &block.stmts[0];
                Some(stmt.to_token_stream())
            } else {
                Some(block.into_token_stream())
            }
        } else {
            Some(value.into_token_stream())
        }
    }

    pub fn parse_as_color(&self) -> Option<TokenStream> {
        let value = &self.0;
        if let syn::Expr::Block(ExprBlock { block, .. }) = value {
            if block.stmts.len() == 1 {
                let stmt = &block.stmts[0];
                Some(stmt.to_token_stream())
            } else {
                Some(block.into_token_stream())
            }
        } else {
            to_color(value)
        }
    }
}
