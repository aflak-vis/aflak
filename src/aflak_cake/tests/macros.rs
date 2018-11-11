#[macro_use]
extern crate variant_name_derive;
extern crate variant_name;
#[macro_use]
extern crate lazy_static;
extern crate aflak_cake;

#[macro_use]
extern crate serde;
extern crate ron;

mod support;
use support::*;

use std::borrow::Cow;

fn make_dst() -> DST<'static, AlgoIO, E> {
    if let &[ref plus1, ref minus1, _, _] = TRANSFORMATIONS.as_slice() {
        // An arrow points from a box's input to a box's output  `OUT -> INT`
        // We build the dst as follows (all functions are trivial and only have 1 output or 0/1 input):
        //    0 (default input) ---\
        //EMPTY                     c, plus1 -> d, plus1 -> OUT1
        // \-> b, minus1 -> OUT2     \-> e, plus1
        let mut dst = DST::new();
        let b = dst.add_transform(&minus1);
        let c = dst.add_transform(&plus1);
        let d = dst.add_transform(&plus1);
        let e = dst.add_transform(&plus1);
        let _out1 = dst.attach_output(Output::new(d, 0)).unwrap();
        let _out2 = dst.attach_output(Output::new(b, 0)).unwrap();
        dst.connect(Output::new(c, 0), Input::new(e, 0)).unwrap();
        dst.connect(Output::new(c, 0), Input::new(d, 0)).unwrap();

        dst
    } else {
        unreachable!()
    }
}

#[test]
fn test_run_macros() {
    let dst = make_dst();
    let macr = aflak_cake::Macro::new(dst);

    assert_eq!(
        macr.call(vec![Cow::Owned(AlgoIO::Integer(1))]),
        vec![Ok(AlgoIO::Integer(2)), Ok(AlgoIO::Integer(0))]
    );
}
