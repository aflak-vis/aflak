//! # aflak - Computational mAKE
//!
//! A crate to manage a graph of interdependent functions.
//!
//! To define a new transformation (i.e. a node in a node graph), for example for
//! an existing project (i.e. aflak), please see the
//! [cake_transform!](macro.cake_transform.html) macro.
extern crate chashmap;
extern crate futures;
extern crate rayon;

extern crate boow;
extern crate serde;
#[macro_use]
extern crate serde_derive;
pub extern crate uuid;
extern crate variant_name;

mod cache;
mod dst;
pub mod export;
mod future;
pub mod macros;
mod timed;
mod transform;

pub use crate::cache::Cache;
pub use crate::dst::{
    compute, DSTError, Input, InputDefaultsMut, InputSlot, LinkIter, MetaTransform, Node, NodeId,
    NodeIter, Output, OutputId, TransformAndDefaults, TransformIdx, DST,
};
pub use crate::export::{DeserDST, ImportError, NamedAlgorithms, SerialDST};
pub use crate::future::Task;
pub use crate::timed::Timed;
pub use crate::transform::*;
pub use boow::Bow;
pub use futures::{future::Future, Async};

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

/// Represent how the variant with the name defined in `from` can be converted
/// to another variant whose name is defined in `into`.
pub struct ConvertibleVariant<T> {
    /// Variant name that can be converted
    pub from: &'static str,
    /// Target variant name
    pub into: &'static str,
    /// Conversion function. Cannot fail.
    pub f: fn(&T) -> T,
}

/// Trait implemented to define conversions between different variants.
///
/// To implement this trait, please define a conversation table from and into
/// the different variants of an enumeration.
pub trait ConvertibleVariants: VariantName + Sized + 'static {
    /// Definition of conversions. This table is user-defined and must be
    /// implemented.
    const CONVERTION_TABLE: &'static [ConvertibleVariant<Self>];

    /// Convert variant `from` into variant `into`. Return `None` if `from`
    /// cannot be converted into `into`.
    fn convert<'a>(
        from: &'static str,
        into: &'static str,
        value: &'a Self,
    ) -> Option<Bow<'a, Self>> {
        if from == into {
            Some(Bow::Borrowed(value))
        } else if let Some(variant) = Self::CONVERTION_TABLE
            .iter()
            .find(|variant| variant.from == from && variant.into == into)
        {
            let out = (variant.f)(value);
            Some(Bow::Owned(out))
        } else {
            None
        }
    }

    /// Check whether variant `from` can be converted into variant `into`.
    fn convertible(from: &'static str, into: &'static str) -> bool {
        if from == into {
            true
        } else if Self::CONVERTION_TABLE
            .iter()
            .find(|variant| variant.from == from && variant.into == into)
            .is_some()
        {
            true
        } else {
            false
        }
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
            _: Vec<&$enum_name>,
        ) -> Vec<Result<$enum_name, $err_type>> {
            $fn_block
        }
    };
    // Standard case
    ($fn_name: ident<$enum_name: ident, $err_type: ty>($($x: ident: $x_type: ident),*) $fn_block: block) => {
        fn $fn_name(
            input: Vec<$crate::Bow<$enum_name>>,
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
///                                       //   _______ MAJOR/MINOR/PATCH version numbers
/// let plus_one_trans = cake_transform!( //  /  /  /
///     "Long description of the transform", 1, 0, 0,
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
    ($description: expr, $major: expr, $minor: expr, $patch: expr, $fn_name: ident<$enum_name: ident, $err_type: ty>($($x: ident: $x_type: ident $(= $x_default_val: expr), *),*) -> $($out_type: ident),* $fn_block: block) => {{
        cake_fn!{$fn_name<$enum_name, $err_type>($($x: $x_type),*) $fn_block}

        $crate::Transform::from_algorithm($crate::Algorithm::Function {
                f: $fn_name,
                id: $crate::FnTransformId(stringify!($fn_name)),
                version: $crate::Version {
                    major: $major,
                    minor: $minor,
                    patch: $patch
                },
                description: $description,
                inputs: vec![$(
                    $crate::TransformInputSlot {
                        type_id: $crate::TypeId(stringify!($x_type)),
                        default: cake_some_first_value!($( $enum_name::$x_type($x_default_val) ),*),
                        name: stringify!($x),
                    }, )*],
                outputs: vec![$($crate::TypeId(stringify!($out_type)), )*],
        })
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
