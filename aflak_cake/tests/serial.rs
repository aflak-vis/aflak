#[macro_use]
extern crate aflak_cake;
#[macro_use]
extern crate variant_name_derive;
extern crate variant_name;
#[macro_use]
extern crate lazy_static;
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
        "{\"name\":\"plus1\",\"input\":[\"Integer\"],\"output\":[\"Integer\"]}"
    );
}

#[test]
fn deserialize_transformation() {
    let deserialized: Transformation<AlgoIO, !> = serde_json::from_str(
        "{\"name\":\"plus1\",\"input\":[\"Integer\"],\"output\":[\"Integer\"]}",
    ).unwrap();
    let plus1transform = get_plus1_transform();
    assert_eq!(deserialized.name, plus1transform.name);
    assert_eq!(deserialized.input, plus1transform.input);
    assert_eq!(deserialized.output, plus1transform.output);
}
