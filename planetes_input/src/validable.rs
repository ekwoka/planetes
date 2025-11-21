use bevy::prelude::*;

/// A trait for types that can be validated from a string input.
///
/// Types implementing this trait can be used with `ValidatedInputField`.
pub trait Validable: Send + Sync + Default + PartialEq + Clone + ToString + 'static {
    /// Attempts to validate and convert a string into this type.
    ///
    /// # Arguments
    ///
    /// * `text` - The input string to validate and convert.
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` if the input is valid and can be converted to this type.
    /// * `Err(String)` with an error message if the input is invalid.
    fn validate(text: &str) -> Result<Self, String>;
}

impl Validable for String {
    fn validate(text: &str) -> Result<Self, String> {
        Ok(text.to_string())
    }
}

#[derive(Component, Default)]
pub enum Validation {
    #[default]
    Valid,
    Invalid(String),
}

macro_rules! impl_validable_for_numeric {
    ($($t:ty),*) => {
        $(
            impl Validable for $t {
                fn validate(text: &str) -> Result<Self, String> {
                    text.parse().map_err(|_| format!("Invalid {} number", stringify!($t)))
                }
            }
        )*
    };
}

impl_validable_for_numeric!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64);
