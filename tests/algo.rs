extern crate aflak_backend;
use aflak_backend::*;

use std::borrow::Cow;

fn plus1(input: Vec<Cow<TypeContent>>) -> Vec<TypeContent> {
    if let TypeContent::Integer(i) = *input[0] {
        vec![TypeContent::Integer(i + 1)]
    } else {
        panic!("Expected integer!")
    }
}


#[test]
fn test_plus1() {
    let plus1transform = Transformation {
        input: vec![Type::Integer],
        output: vec![Type::Integer],
        algorithm: plus1,
    };

    let mut caller = plus1transform.start();
    caller.feed(&TypeContent::Integer(1));
    let mut ret = caller.call();
    assert_eq!(ret.next(), Some(TypeContent::Integer(2)));
}
