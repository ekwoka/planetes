use bevy::prelude::*;

/// A trait for types that can be validated from a string input.
///
/// Types implementing this trait can be used with `ValidatedInputField`.
pub trait Validable: Send + Sync + Default + PartialEq + Clone + ToString + 'static {
    /// The characters that are accepted for this type.
    const ACCEPTED_CHARS: &'static str;
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

    /// Returns the characters that are accepted for this type.
    fn accepted_chars() -> &'static str {
        Self::ACCEPTED_CHARS
    }
}

impl Validable for String {
    const ACCEPTED_CHARS: &'static str = "";

    fn validate(text: &str) -> Result<Self, String> {
        Ok(text.to_string())
    }
}

#[derive(Component, Default, Debug, PartialEq, Eq, Clone)]
pub enum Validation {
    #[default]
    Valid,
    Invalid(String),
}

macro_rules! impl_validable_for_unsigned_numeric {
    ($($t:ty),*) => {
        $(
            impl Validable for $t {
                const ACCEPTED_CHARS: &'static str = "0123456789";
                fn validate(text: &str) -> Result<Self, String> {
                    text.parse().map_err(|_| format!("Invalid {} number", stringify!($t)))
                }
            }
        )*
    };
}

macro_rules! impl_validable_for_integer_numeric {
    ($($t:ty),*) => {
        $(
            impl Validable for $t {
                const ACCEPTED_CHARS: &'static str = "-0123456789";
                fn validate(text: &str) -> Result<Self, String> {
                    text.parse().map_err(|_| format!("Invalid {} number", stringify!($t)))
                }
            }
        )*
    };
}

macro_rules! impl_validable_for_float_numeric {
    ($($t:ty),*) => {
        $(
            impl Validable for $t {
                const ACCEPTED_CHARS: &'static str = "-.0123456789";
                fn validate(text: &str) -> Result<Self, String> {
                    text.parse().map_err(|_| format!("Invalid {} number", stringify!($t)))
                }
            }
        )*
    };
}

impl_validable_for_unsigned_numeric!(u8, u16, u32, u64, u128, isize);
impl_validable_for_integer_numeric!(i8, i16, i32, i64, i128, usize);
impl_validable_for_float_numeric!(f32, f64);
