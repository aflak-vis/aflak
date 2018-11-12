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

fn make_macro() -> aflak_cake::Macro<'static, AlgoIO, E> {
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

        aflak_cake::Macro::new(dst)
    } else {
        unreachable!()
    }
}

#[test]
fn test_run_macros() {
    let macr = make_macro();

    assert_eq!(
        macr.call(vec![Cow::Owned(AlgoIO::Integer(1))]),
        vec![Ok(AlgoIO::Integer(2)), Ok(AlgoIO::Integer(0))]
    );
}

#[test]
fn test_add_macro_to_dst() {
    let get1 = get_get1_transform();
    let macr = make_macro();
    let macro_t = aflak_cake::Transformation::new_macro(&macr);
    let mut dst = DST::new();
    let macro_t_idx = dst.add_transform(&macro_t);
    let out_1 = dst.attach_output(Output::new(macro_t_idx, 0)).unwrap();
    let a = dst.add_transform(&get1);
    dst.connect(Output::new(a, 0), Input::new(macro_t_idx, 0))
        .unwrap();

    assert_eq!(dst.compute(out_1).unwrap(), AlgoIO::Integer(2));
}
