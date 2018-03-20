mod support;
use support::*;

use std::borrow::Cow;

fn plus1(input: Vec<Cow<AlgoContent>>) -> Vec<AlgoContent> {
    if let AlgoContent::Integer(i) = *input[0] {
        vec![AlgoContent::Integer(i + 1)]
    } else {
        panic!("Expected integer!")
    }
}

#[test]
fn test_plus1() {
    let plus1transform = Transformation {
        name: "+1",
        input: vec![AlgoType::Integer],
        output: vec![AlgoType::Integer],
        algorithm: plus1,
    };

    let mut caller = plus1transform.start();
    caller.feed(&AlgoContent::Integer(1));
    let mut ret = caller.call();
    assert_eq!(ret.next(), Some(AlgoContent::Integer(2)));
}
