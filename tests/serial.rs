#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod support;
use support::*;

#[test]
fn serialize_transformation() {
    let plus1transform = get_plus1_transform();
    let json = serde_json::to_string(&plus1transform).unwrap();
    assert_eq!(
        json,
        "{\"name\":\"+1\",\"input\":[\"Integer\"],\"output\":[\"Integer\"]}"
    );
}
