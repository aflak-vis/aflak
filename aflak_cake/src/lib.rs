//! # aflak - Computational mAKE
//!
//! A crate to manage a graph of interdependent functions.
extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate bow;
extern crate variant_name;

mod dst;
mod serial;
mod transform;

pub use dst::*;
pub use transform::*;

pub use self::variant_name::VariantName;

pub trait DefaultFor {
    fn default_for(variant_name: &'static str) -> Self;
}

/// Make it easier to define a function used for a transform
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

#[macro_export]
macro_rules! cake_transform {
    ($fn_name: ident<$enum_name: ident, $err_type: ty>($($x: ident: $x_type: ident),*) -> $($out_type: ident),* $fn_block: block) => {{
        cake_fn!{$fn_name<$enum_name, $err_type>($($x: $x_type),*) $fn_block}
        $crate::Transformation {
            name: stringify!($fn_name),
            input: vec![$(stringify!($x_type), )*],
            output: vec![$(stringify!($out_type), )*],
            algorithm: $crate::Algorithm::Function($fn_name),
        }
    }};
}

#[macro_export]
macro_rules! cake_constant {
    ($const_name: ident, $($x: expr),*) => {{
        use $crate::VariantName;
        let constant = vec![$($x, )*];
        $crate::Transformation {
            name: stringify!($const_name),
            input: vec![],
            output: constant.iter().map(|c| c.variant_name()).collect(),
            algorithm: $crate::Algorithm::Constant(constant),
        }
    }};
}
