//! # aflak - Computational mAKE
//!
//! A crate to manage a graph of interdependent functions.
extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate variant_name;

mod transform;
mod dst;
mod serial;

pub use transform::*;
pub use dst::*;

/// Make it easier to define a function used for a transform
#[macro_export]
macro_rules! cake_fn {
    // Special case where no argument is provided
    ($fn_name: ident<$enum_name: ident>() $fn_block: block) => {
        fn $fn_name(
            _: Vec<::std::borrow::Cow<$enum_name>>,
        ) -> Vec<Result<$enum_name, <$enum_name as $crate::TypeContent>::Err>> {
            $fn_block
        }
    };
    // Standard case
    ($fn_name: ident<$enum_name: ident>($($x: ident: $x_type: ident),*) $fn_block: block) => {
        fn $fn_name(
            input: Vec<::std::borrow::Cow<$enum_name>>,
        ) -> Vec<Result<$enum_name, <$enum_name as $crate::TypeContent>::Err>> {
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
