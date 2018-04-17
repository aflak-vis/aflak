/// Trait to uniquely identify the variant of an enumeration.
///
/// Similar to [`Discriminant`] except that it can easily printed and is
/// user-defined. This trait can entirely be derived with the
/// `variant_name_derive` crate.
///
/// # Example
///
/// ```
/// #[macro_use] extern crate variant_name_derive;
/// extern crate variant_name;
///
/// use variant_name::VariantName;
///
/// #[derive(VariantName)]
/// enum EnumTest {
///     VariantA,
///     VariantB { a: usize },
///     VariantC(usize),
/// }
/// ```
///
/// [`Discriminant`]: std::mem::Discriminant
pub trait VariantName {
    /// Get identifier of variant
    fn variant_name(&self) -> &'static str;
    /// Get each identifier of all possible variants
    fn variant_names() -> &'static [&'static str];
}
