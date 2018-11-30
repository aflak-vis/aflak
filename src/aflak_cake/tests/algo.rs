extern crate aflak_cake;
#[macro_use]
extern crate variant_name_derive;
extern crate variant_name;
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde;

mod support;
use support::*;

#[test]
fn test_plus1() {
    let plus1transform = get_plus1_transform();

    let mut caller = plus1transform.start();
    caller.feed(&AlgoIO::Integer(1));
    let mut ret = caller.call();
    assert_eq!(ret.next(), Some(Ok(AlgoIO::Integer(2))));
}

#[test]
fn test_autoconversions() {
    let divide_by_10 = get_divide_by_10_transform();

    let one = AlgoIO::Integer(1);
    let mut caller = divide_by_10.start();
    caller.feed(&one);
    let mut ret = caller.call();
    assert_eq!(ret.next(), Some(Ok(AlgoIO::Float(0.1))));
}
