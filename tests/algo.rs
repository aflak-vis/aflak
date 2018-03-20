#[macro_use]
extern crate serde_derive;

mod support;
use support::*;

#[test]
fn test_plus1() {
    let plus1transform = get_plus1_transform();

    let mut caller = plus1transform.start();
    caller.feed(&AlgoContent::Integer(1));
    let mut ret = caller.call();
    assert_eq!(ret.next(), Some(AlgoContent::Integer(2)));
}
