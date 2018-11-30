#[macro_use]
extern crate variant_name_derive;
extern crate variant_name;
#[macro_use]
extern crate lazy_static;
extern crate aflak_cake;
extern crate futures;

#[macro_use]
extern crate serde;
extern crate ron;

mod support;
use support::*;

use aflak_cake::Cache;
use futures::{future::Future, Async};
use ron::de;
use ron::ser;

fn get_all_transforms() -> [Transform<AlgoIO, E>; 4] {
    [
        get_plus1_transform(),
        get_minus1_transform(),
        get_get1_transform(),
        get_get_image_transform(),
    ]
}

macro_rules! assert_output_eq {
    ($dst: expr, $output: expr, $expected_value: expr, $cache: expr) => {{
        let mut promise = $dst.compute($output, $cache);
        let out = loop {
            match promise.poll() {
                Ok(Async::Ready(r)) => break r,
                Ok(Async::NotReady) => ::std::thread::yield_now(),
                Err(e) => panic!("Fails: {}", e),
            }
        };
        assert_eq!(**out, $expected_value);
    }};
}

#[test]
fn test_make_dst_and_iterate_dependencies() {
    let [plus1, minus1, get1, _image] = get_all_transforms();

    // An arrow points from a box's input to a box's output  `OUT -> INT`
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

    // Serialize and unserialize DST
    let s = ser::to_string(&dst).unwrap();
    let dst: DST<AlgoIO, E> = de::from_str(&s).unwrap();

    let mut cache = Cache::new();

    assert_output_eq!(dst, out1, AlgoIO::Integer(3), &mut cache);
    assert_output_eq!(dst, out2, AlgoIO::Integer(0), &mut cache);
}

#[test]
fn test_connect_wrong_types() {
    let [plus1, _minus1, _get1, image] = get_all_transforms();

    // An arrow points from a box's input to a box's output  `OUT -> INT`
    // We build the dst as follows (all functions are trivial and only have 1 output or 0/1 input):
    // a, image
    // \-> b, plus1 -> OUT1
    let mut dst = DST::new();
    let a = dst.add_transform(&image);
    let b = dst.add_transform(&plus1);
    let _out1 = dst.attach_output(Output::new(b, 0)).unwrap();
    if let Err(DSTError::IncompatibleTypes(_)) = dst.connect(Output::new(a, 0), Input::new(b, 0)) {
        assert!(true);
    } else {
        assert!(false, "IncompatibleTypes expected!");
    }
}

#[test]
fn test_cache_reset() {
    let [plus1, minus1, get1] = if let &[plus1, minus1, get1, _image] = *TRANSFORMATIONS_REF {
        [plus1, minus1, get1]
    } else {
        unreachable!()
    };

    // a, get1 -------------------> c, plus1 -> d, plus1 -> OUT1
    // \-> b, minus1 -> OUT2        \-> e, plus1
    let mut dst = DST::new();
    let a = dst.add_transform(&get1);
    let b = dst.add_transform(&minus1);
    let c = dst.add_transform(&plus1);
    let d = dst.add_transform(&plus1);
    let e = dst.add_transform(&plus1);
    let out1 = dst.attach_output(Output::new(d, 0)).unwrap();
    let _out2 = dst.attach_output(Output::new(b, 0)).unwrap();

    dst.connect(Output::new(a, 0), Input::new(c, 0)).unwrap();
    dst.connect(Output::new(a, 0), Input::new(b, 0)).unwrap();
    dst.connect(Output::new(c, 0), Input::new(e, 0)).unwrap();
    dst.connect(Output::new(c, 0), Input::new(d, 0)).unwrap();

    let mut cache = Cache::new();

    assert_output_eq!(dst, out1, AlgoIO::Integer(3), &mut cache);
    // Connect b's output to c's input
    dst.connect(Output::new(b, 0), Input::new(c, 0)).unwrap();
    assert_output_eq!(dst, out1, AlgoIO::Integer(2), &mut cache);
}

#[test]
fn test_remove_node() {
    let [plus1, minus1, get1, _image] = get_all_transforms();

    // a, get1 -------------------> c, plus1 -> d, plus1 -> OUT1
    // \-> b, minus1 -> OUT2        \-> e, plus1

    let mut dst = DST::new();
    let a = dst.add_transform(&get1);
    let b = dst.add_transform(&minus1);
    let c = dst.add_transform(&plus1);
    let d = dst.add_transform(&plus1);
    let e = dst.add_transform(&plus1);

    let d_out0 = Output::new(d, 0);
    let b_out0 = Output::new(b, 0);

    let out1 = dst.attach_output(d_out0).unwrap();
    let out2 = dst.attach_output(b_out0).unwrap();

    let a_out0 = Output::new(a, 0);
    let b_in0 = Input::new(b, 0);

    dst.connect(a_out0, Input::new(c, 0)).unwrap();
    dst.connect(a_out0, b_in0).unwrap();
    dst.connect(Output::new(c, 0), Input::new(e, 0)).unwrap();
    dst.connect(Output::new(c, 0), Input::new(d, 0)).unwrap();

    // We will remove "c"
    // After removal, the graph will be come as below
    //
    // a, get1                         d, plus1 -> OUT1
    // \-> b, minus1 -> OUT2           e, plus1

    dst.remove_transform(c);
    let mut links: Vec<_> = dst.links_iter().collect();
    links.sort();
    assert_eq!(links, {
        let mut vec = vec![
            (&a_out0, aflak_cake::InputSlot::Transform(b_in0)),
            (&d_out0, aflak_cake::InputSlot::Output(out1)),
            (&b_out0, aflak_cake::InputSlot::Output(out2)),
        ];
        vec.sort();
        vec
    });
}
