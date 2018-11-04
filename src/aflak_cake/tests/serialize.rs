extern crate aflak_cake;
#[macro_use]
extern crate variant_name_derive;
extern crate variant_name;
#[macro_use]
extern crate lazy_static;

extern crate ron;
#[macro_use]
extern crate serde;

mod support;
use support::*;

use aflak_cake::{DeserTransform, SerialTransform};
use ron::de::from_str;
use ron::ser::to_string;

#[test]
fn test_plus1() {
    let plus1transform = get_plus1_transform();

    let s = to_string(&SerialTransform::new(&plus1transform)).unwrap();
    assert_eq!("Function(\"plus1\")", s);

    let plus1_deser: DeserTransform<AlgoIO, E> = from_str(&s).unwrap();
    let plus1transform_back = plus1_deser.into().unwrap();

    // Check that plus1transform_back behaves as plus1transform
    let mut caller = plus1transform_back.start();
    caller.feed_ref(&AlgoIO::Integer(1));
    let mut ret = caller.call();
    assert_eq!(ret.next(), Some(Ok(AlgoIO::Integer(2))));

    let const1 = get_get1_transform();

    let s = to_string(&SerialTransform::new(&const1)).unwrap();
    assert_eq!("Constant([Integer(1),])", s);

    let const1_deser: DeserTransform<AlgoIO, E> = from_str(&s).unwrap();
    let const1_back = const1_deser.into().unwrap();
    // Check that const1_back behaves as const1
    let caller = const1_back.start();
    let mut ret = caller.call();
    assert_eq!(ret.next(), Some(Ok(AlgoIO::Integer(1))));
}
