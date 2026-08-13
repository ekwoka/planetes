use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{Attribute, BorderRadius, Value};
#[derive(Clone, Debug)]
pub struct NodeComponent {
    attributes: Vec<Attribute>,
    bsn: bool,
}

impl NodeComponent {
    const KEYS: [&'static str; 47] = [
        "padding",
        "padding-top",
        "padding-left",
        "padding-bottom",
        "padding-right",
        "margin",
        "margin-top",
        "margin-left",
        "margin-bottom",
        "margin-right",
        "border",
        "border-top",
        "border-left",
        "border-bottom",
        "border-right",
        "top",
        "left",
        "bottom",
        "right",
        "width",
        "height",
        "min-width",
        "min-height",
        "max-width",
        "max-height",
        "row-gap",
        "column-gap",
        "display",
        "position",
        "position-type",
        "flex-direction",
        "flex-wrap",
        "align-items",
        "justify-items",
        "align-self",
        "justify-self",
        "align-content",
        "justify-content",
        "box-sizing",
        "grid-auto-flow",
        "flex-grow",
        "flex-shrink",
        "scrollbar-width",
        "aspect-ratio",
        "overflow",
        "overflow-clip-margin",
        "border-radius",
    ];

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
        Some(self)
    }

    fn kebab_to_snake(s: &str) -> String {
        s.replace('-', "_")
    }

    fn get_attr<'a>(attributes: &'a [Attribute], kebab_name: &str) -> Option<&'a syn::Expr> {
        attributes
            .iter()
            .find(|attr| attr.key == kebab_name)
            .map(|attr| &attr.value)
    }

    fn parse_css_value(value: &syn::Expr) -> Option<TokenStream> {
        // Handle block expressions - extract the inner content
        let value_tokens = if let syn::Expr::Block(expr_block) = value {
            let stmts = &expr_block.block.stmts;
            quote! { #(#stmts)* }
        } else {
            quote! { #value }
        };

        // Try to parse as a string literal (CSS-style values)
        let value_str = value_tokens.to_string();

        // Check if this is a quoted string (CSS-style value)
        if value_str.starts_with('"') && value_str.ends_with('"') {
            // Remove quotes
            let value_str = value_str.trim_matches('"');

            if let Some(px_value) = value_str.strip_suffix("px") {
                let num =
                    syn::LitFloat::new(&format!("{px_value}.0"), proc_macro2::Span::call_site());
                Some(quote! { ::bevy::ui::px(#num) })
            } else if let Some(percent_value) = value_str.strip_suffix("%") {
                let num = syn::LitFloat::new(
                    &format!("{percent_value}.0"),
                    proc_macro2::Span::call_site(),
                );
                Some(quote! { ::bevy::ui::percent(#num) })
            } else if let Some(vw_value) = value_str.strip_suffix("vw") {
                let num =
                    syn::LitFloat::new(&format!("{vw_value}.0"), proc_macro2::Span::call_site());
                Some(quote! { ::bevy::ui::vw(#num) })
            } else if let Some(vh_value) = value_str.strip_suffix("vh") {
                let num =
                    syn::LitFloat::new(&format!("{vh_value}.0"), proc_macro2::Span::call_site());
                Some(quote! { ::bevy::ui::vh(#num) })
            } else if let Some(vmin_value) = value_str.strip_suffix("vmin") {
                let num =
                    syn::LitFloat::new(&format!("{vmin_value}.0"), proc_macro2::Span::call_site());
                Some(quote! { ::bevy::ui::vmin(#num) })
            } else if let Some(vmax_value) = value_str.strip_suffix("vmax") {
                let num =
                    syn::LitFloat::new(&format!("{vmax_value}.0"), proc_macro2::Span::call_site());
                Some(quote! { ::bevy::ui::vmax(#num) })
            } else {
                Some(quote! { ::bevy::ui::Val::from(#value_tokens) })
            }
        } else {
            // Not a CSS-style value, assume it's a Rust expression (e.g., px(10.0))
            // Use the value directly without prepending namespace
            Some(value_tokens)
        }
    }

    fn parse_enum_value(value: &syn::Expr) -> Option<TokenStream> {
        // Handle block expressions - extract the inner content
        let value_tokens = if let syn::Expr::Block(expr_block) = value {
            let stmts = &expr_block.block.stmts;
            quote! { #(#stmts)* }
        } else {
            quote! { #value }
        };

        // Check if this is a quoted string (CSS-style enum value)
        let value_str = value_tokens.to_string();
        if value_str.starts_with('"') && value_str.ends_with('"') {
            // Remove quotes and use the string as-is (CSS-style)
            let value_str = value_str.trim_matches('"');
            // Return as an identifier path
            if let Ok(path) = syn::parse_str::<syn::Path>(value_str) {
                return Some(quote! { #path });
            }
            None
        } else {
            // Not a CSS-style value, assume it's a Rust expression
            Some(value_tokens)
        }
    }

    fn parse_numeric_value(value: &syn::Expr) -> Option<TokenStream> {
        // Handle block expressions - extract the inner content
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

    fn build_spacing_chain(attributes: &[Attribute], property: &str) -> Option<TokenStream> {
        // Define directions in priority order: all -> top -> right -> bottom -> left
        let directions = [
            ("", "all"),
            ("-top", "top"),
            ("-right", "right"),
            ("-bottom", "bottom"),
            ("-left", "left"),
        ];

        // Collect parsed values for each direction
        let values: Vec<_> = directions
            .iter()
            .map(|(suffix, _)| {
                let key = format!("{property}{suffix}");
                Self::get_attr(attributes, &key).and_then(Self::parse_css_value)
            })
            .collect();

        // Find the first available direction and start the chain
        let (start_idx, mut chain) = values.iter().enumerate().find_map(|(idx, val)| {
            val.as_ref().map(|v| {
                let method = syn::Ident::new(directions[idx].1, proc_macro2::Span::call_site());
                (idx, quote! { #v.#method() })
            })
        })?;

        // Chain the remaining directions using with_X() methods
        for (idx, val) in values.iter().enumerate() {
            if idx != start_idx
                && let Some(v) = val
            {
                let method_name = format!("with_{}", directions[idx].1);
                let method = syn::Ident::new(&method_name, proc_macro2::Span::call_site());
                chain = quote! { #chain.#method(#v) };
            }
        }

        Some(chain)
    }
}

impl ToTokens for NodeComponent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if self.attributes.is_empty() {
            if self.bsn {
                tokens.extend(quote! { bevy::ui::Node });
            } else {
                tokens.extend(quote! { ::bevy::ui::Node::default() });
            }
        } else {
            let mut fields = Vec::new();

            // Process padding attributes
            if let Some(padding_tokens) = Self::build_spacing_chain(&self.attributes, "padding") {
                fields.push(quote! {
                    padding: { #padding_tokens }
                });
            }

            // Process margin attributes
            if let Some(margin_tokens) = Self::build_spacing_chain(&self.attributes, "margin") {
                fields.push(quote! {
                    margin: { #margin_tokens }
                });
            }

            // Process border attributes
            if let Some(border_tokens) = Self::build_spacing_chain(&self.attributes, "border") {
                fields.push(quote! {
                    border: { #border_tokens }
                });
            }

            // Process BorderRadius attributes
            if let Some(border_radius) = BorderRadius::from(&self.attributes).ok() {
                fields.push(quote! {
                    border_radius: { #border_radius }
                });
            }

            // Process simple Val properties (positioning)
            for prop in ["left", "right", "top", "bottom"] {
                if let Some(value) =
                    Self::get_attr(&self.attributes, prop).and_then(Self::parse_css_value)
                {
                    let field = syn::Ident::new(prop, proc_macro2::Span::call_site());
                    fields.push(quote! {
                        #field: #value
                    });
                }
            }

            // Process sizing properties
            for prop in [
                "width",
                "height",
                "min-width",
                "min-height",
                "max-width",
                "max-height",
            ] {
                if let Some(value) =
                    Self::get_attr(&self.attributes, prop).and_then(Self::parse_css_value)
                {
                    let field_name = Self::kebab_to_snake(prop);
                    let field = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
                    fields.push(quote! {
                        #field: #value
                    });
                }
            }

            // Process gap properties
            for prop in ["row-gap", "column-gap"] {
                if let Some(value) =
                    Self::get_attr(&self.attributes, prop).and_then(Self::parse_css_value)
                {
                    let field_name = Self::kebab_to_snake(prop);
                    let field = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
                    fields.push(quote! {
                        #field: #value
                    });
                }
            }

            // Process flex-basis
            if let Some(value) =
                Self::get_attr(&self.attributes, "flex-basis").and_then(Self::parse_css_value)
            {
                fields.push(quote! {
                    flex_basis: #value
                });
            }

            if let Some(value) = Self::get_attr(&self.attributes, "display")
                .map(Value::new)
                .and_then(|val| val.as_display())
            {
                fields.push(quote! {
                    display: #value
                });
            }

            if let Some(value) = Self::get_attr(&self.attributes, "flex-direction")
                .map(Value::new)
                .and_then(|val| val.as_flex_direction())
            {
                fields.push(quote! {
                    flex_direction: #value
                });
            }

            if let Some(value) = Self::get_attr(&self.attributes, "justify-content")
                .map(Value::new)
                .and_then(|val| val.as_justify_content())
            {
                fields.push(quote! {
                    justify_content: #value
                });
            }

            if let Some(position) = Self::get_attr(&self.attributes, "position")
                .or(Self::get_attr(&self.attributes, "position-type"))
                .map(Value::new)
                .and_then(|val| val.as_position_type())
            {
                fields.push(quote! {
                    position_type: #position
                });
            }

            // Process enum properties
            for prop in [
                "flex-wrap",
                "align-items",
                "justify-items",
                "align-self",
                "justify-self",
                "align-content",
                "box-sizing",
                "grid-auto-flow",
            ] {
                if let Some(value) =
                    Self::get_attr(&self.attributes, prop).and_then(Self::parse_enum_value)
                {
                    let field_name = Self::kebab_to_snake(prop);
                    let field = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
                    fields.push(quote! {
                        #field: #value
                    });
                }
            }

            // Process numeric properties (f32)
            for prop in ["flex-grow", "flex-shrink", "scrollbar-width"] {
                if let Some(value) =
                    Self::get_attr(&self.attributes, prop).and_then(Self::parse_numeric_value)
                {
                    let field_name = Self::kebab_to_snake(prop);
                    let field = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
                    fields.push(quote! {
                        #field: #value
                    });
                }
            }

            // Process aspect-ratio (Option<f32>)
            if let Some(value) = Self::get_attr(&self.attributes, "aspect-ratio")
                && let Some(parsed) = Self::parse_numeric_value(value)
            {
                fields.push(quote! {
                    aspect_ratio: Some(#parsed)
                });
            }

            // Process overflow (special struct)
            if let Some(value) =
                Self::get_attr(&self.attributes, "overflow").and_then(Self::parse_enum_value)
            {
                fields.push(quote! {
                    overflow: #value
                });
            }

            // Process overflow-clip-margin (special struct)
            if let Some(value) = Self::get_attr(&self.attributes, "overflow-clip-margin")
                .and_then(Self::parse_enum_value)
            {
                fields.push(quote! {
                    overflow_clip_margin: #value
                });
            }
            if self.bsn {
                tokens.extend(quote! {
                    bevy::ui::Node {
                        #(#fields),*
                    }
                });
            } else {
                tokens.extend(quote! {
                    ::bevy::ui::Node {
                        #(#fields,)*
                        ..Default::default()
                    }
                });
            }
        }
    }
}
