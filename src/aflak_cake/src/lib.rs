//! # aflak - Computational mAKE
//!
//! A crate to manage a graph of interdependent functions.
extern crate rayon;

extern crate boow;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate variant_name;

mod dst;
mod export;
mod macros;
mod transform;

pub use dst::*;
pub use export::*;
pub use macros::{GuardRef, Macro, MacroEvaluationError, MacroHandle};
pub use transform::*;

pub use self::variant_name::VariantName;

/// Trait to define a default value for each variant of an enumeration.
pub trait DefaultFor: VariantName {
    fn default_for(variant_name: &str) -> Self;
}

/// Trait to discriminate editable variants from constant variants of an
/// enumeration.
///
/// Especially used for a node editor.
pub trait EditableVariants: VariantName {
    /// Get list of editable variants.
    fn editable_variants() -> &'static [&'static str];
    /// Check if given variant is editable or not.
    fn editable(variant_name: &str) -> bool {
        Self::editable_variants().contains(&variant_name)
    }
}

/// Make it easier to define a function used for a transform. Used internally
/// by [`cake_transform`]. You probably want to directly use [`cake_transform`].
#[doc(hidden)]
#[macro_export]
macro_rules! cake_fn {
    // Special case where no argument is provided
    ($fn_name: ident<$enum_name: ident, $err_type: ty>() $fn_block: block) => {
        fn $fn_name(
            _: Vec<::std::borrow::Cow<$enum_name>>,
        ) -> Vec<Result<$enum_name, $err_type>> {
            $fn_block
        }
    };
    // Standard case
    ($fn_name: ident<$enum_name: ident, $err_type: ty>($($x: ident: $x_type: ident),*) $fn_block: block) => {
        fn $fn_name(
            input: Vec<::std::borrow::Cow<$enum_name>>,
        ) -> Vec<Result<$enum_name, $err_type>> {
            #[allow(non_camel_case_types)]
            enum Args { $($x,)* }
            if let ($(&$enum_name::$x_type(ref $x), )*) = ($(&*input[Args::$x as usize], )*) {
                $fn_block
            } else {
                panic!("Unexpected argument!")
            }
        }
    };
}

/// Create a new transform from a rust function.
///
/// # Example
///
/// ```rust
/// #[macro_use] extern crate variant_name_derive;
/// #[macro_use] extern crate aflak_cake;
/// use aflak_cake::*;
///
/// #[derive(Clone, PartialEq, Debug, VariantName)]
/// pub enum AlgoIO {
///     Integer(u64),
///     Image2d(Vec<Vec<f64>>),
/// }
///
/// pub enum E {}
///
/// let plus_one_trans = cake_transform!(
///     "Long description of the transform",
/// // key identifying transformation   Input arguments with default value (optional)
/// //   \  In/Out types /- Error type  /        _ Output type(s)
/// //    \       /     / /------------/        /
///     plus1<AlgoIO, E>(i: Integer = 0) -> Integer {
///     // Define the body of the transformation.
///     // Must return a Vec<Result<AlgoIO, !>>!
///     vec![Ok(AlgoIO::Integer(i + 1))]
/// });
/// ```
#[macro_export]
macro_rules! cake_transform {
    ($description: expr, $fn_name: ident<$enum_name: ident, $err_type: ty>($($x: ident: $x_type: ident $(= $x_default_val: expr), *),*) -> $($out_type: ident),* $fn_block: block) => {{
        cake_fn!{$fn_name<$enum_name, $err_type>($($x: $x_type),*) $fn_block}
        use std::borrow::Cow;

        $crate::Transformation {
            name: stringify!($fn_name),
            description: Cow::Borrowed($description),
            input: vec![$((stringify!($x_type), {
                cake_some_first_value!($( $enum_name::$x_type($x_default_val) ),*)
            }), )*],
            output: vec![$(stringify!($out_type), )*],
            algorithm: $crate::Algorithm::Function($fn_name),
        }
    }};
}

/// Make a constant.
///
/// Subject for deprecation.
/// You'd probably better use [`Transformation::new_constant`].
#[macro_export]
macro_rules! cake_constant {
    ($const_name: ident, $($x: expr),*) => {{
        use std::borrow::Cow;
        use $crate::VariantName;

        let constant = vec![$($x, )*];
        $crate::Transformation {
            name: stringify!($const_name),
            description: Cow::Borrowed(
                concat!("Constant variable of type '", stringify!($const_name), "'")
            ),
            input: vec![],
            output: constant.iter().map(|c| c.variant_name()).collect(),
            algorithm: $crate::Algorithm::Constant(constant),
        }
    }};
}

/// Helper macro for internal use.
#[doc(hidden)]
#[macro_export]
macro_rules! cake_some_first_value {
    () => {
        None
    };
    ($x:expr) => {
        Some($x)
    };
    ($x:expr, $($xs:expr)+) => {
        compile_error!("Only zero or one value is expected.")
    };
}
