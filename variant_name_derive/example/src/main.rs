#[macro_use]
extern crate variant_name_derive;

#[derive(VariantName)]
enum EnumTest {
    A,
    B(usize),
}

#[derive(VariantName)]
struct StructTest;

trait VariantName {
    fn variant_name(&self) -> &'static str;
}

fn main() {
    println!("{}", EnumTest::A.variant_name());
    println!("{}", EnumTest::B(2).variant_name());
    println!("{}", StructTest.variant_name());
}
