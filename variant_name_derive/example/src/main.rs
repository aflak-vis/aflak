#[macro_use]
extern crate variant_name_derive;
extern crate variant_name;

use variant_name::VariantName;

#[derive(VariantName)]
enum EnumTest {
    A,
    B(usize),
    C(usize, usize),
    D{ _a: usize, _b: usize },
}

#[derive(VariantName)]
struct StructTest;

fn main() {
    println!("{}", EnumTest::A.variant_name());
    println!("{}", EnumTest::B(2).variant_name());
    println!("{}", EnumTest::C(3,1).variant_name());
    println!("{}", EnumTest::D{_a: 1, _b: 3}.variant_name());
    println!("{}", StructTest.variant_name());

    println!("{:?}", EnumTest::variant_names());
    println!("{:?}", StructTest::variant_names());
}
