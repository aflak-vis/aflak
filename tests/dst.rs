#![feature(slice_patterns)]

#[macro_use]
extern crate serde_derive;

mod support;
use support::*;

fn get_all_transforms() -> [Transformation<'static, AlgoContent>; 3] {
    [
        get_plus1_transform(),
        get_minus1_transform(),
        get_get1_transform(),
    ]
}

#[test]
fn test_make_dst_and_iterate_dependencies() {
    let [plus1, minus1, get1] = get_all_transforms();

    // An error points from a box's input to a box's output  `OUT -> INT`
    // We build the dst as follows (all functions are trivial and only have 1 output or 0/1 input):
    // a, get1 -------------------> c, plus1 -> d, plus1 -> OUT1
    // \-> b, minus1 -> OUT2        \-> e, plus1
    let mut dst = DST::new();
    let a = dst.add_transform(&get1);
    let b = dst.add_transform(&minus1);
    let c = dst.add_transform(&plus1);
    let d = dst.add_transform(&plus1);
    let e = dst.add_transform(&plus1);
    let out1 = dst.attach_output(Output::new(d, 0)).unwrap();
    let out2 = dst.attach_output(Output::new(b, 0)).unwrap();
    dst.connect(Output::new(a, 0), Input::new(c, 0)).unwrap();
    dst.connect(Output::new(a, 0), Input::new(b, 0)).unwrap();
    dst.connect(Output::new(c, 0), Input::new(e, 0)).unwrap();
    dst.connect(Output::new(c, 0), Input::new(d, 0)).unwrap();

    assert_eq!("TransformIdx(5)", format!("{:?}", e));
    assert_eq!("OutputId(1)", format!("{:?}", out1));

    let mut deps = dst.dependencies(&out1).unwrap();
    assert_eq!(deps.next().unwrap().transform_idx(), a);
    assert_eq!(deps.next().unwrap().transform_idx(), c);
    assert_eq!(deps.next().unwrap().transform_idx(), d);
    assert_eq!(deps.next().as_ref().map(Dependency::transform_idx), None);

    let mut deps = dst.dependencies(&out2).unwrap();
    assert_eq!(deps.next().unwrap().transform_idx(), a);
    assert_eq!(deps.next().unwrap().transform_idx(), b);
    assert_eq!(deps.next().as_ref().map(Dependency::transform_idx), None);
}
