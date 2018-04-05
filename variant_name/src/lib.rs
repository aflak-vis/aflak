pub trait VariantName {
    fn variant_name(&self) -> &'static str;
    fn variant_names() -> &'static [&'static str];
}
