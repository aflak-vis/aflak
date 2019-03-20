#[macro_use]
extern crate variant_name_derive;
extern crate variant_name;
#[macro_use]
extern crate lazy_static;
extern crate aflak_cake;

#[macro_use]
extern crate serde;
extern crate ron;

extern crate boow;

use boow::Bow;

mod support;
use support::*;

fn make_macro() -> aflak_cake::macros::MacroHandle<'static, AlgoIO, E> {
    if let &[plus1, minus1, _, _, _] = *TRANSFORMATIONS_REF {
        // An arrow points from a box's input to a box's output  `OUT -> INT`
        // We build the dst as follows (all functions are trivial and only have 1 output or 0/1 input):
        //           0 (default input) ---\
        //EMPTY                            c, plus1 -> d, plus1 -> OUT1
        // \-> b, minus1 -> OUT2            \-> e, plus1
        let mut dst = DST::new();
        let b = dst.add_transform(&minus1);
        let c = dst.add_transform(&plus1);
        let d = dst.add_transform(&plus1);
        let e = dst.add_transform(&plus1);
        let _out1 = dst.attach_output(Output::new(d, 0)).unwrap();
        let _out2 = dst.attach_output(Output::new(b, 0)).unwrap();
        dst.connect(Output::new(c, 0), Input::new(e, 0)).unwrap();
        dst.connect(Output::new(c, 0), Input::new(d, 0)).unwrap();

        let mut manager = aflak_cake::macros::MacroManager::new();
        let macr = manager.create_macro();
        *macr.write().dst_mut() = dst;
        macr.clone()
    } else {
        unreachable!()
    }
}

#[test]
fn test_run_macros() {
    let macr = make_macro();

    let got_outputs: Vec<_> = macr
        .call(vec![Bow::Owned(AlgoIO::Integer(1))])
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(got_outputs, vec![AlgoIO::Integer(2), AlgoIO::Integer(0)]);
}

#[test]
fn test_macro_inputs() {
    let macr = make_macro();

    assert_eq!(
        macr.inputs(),
        vec![
            aflak_cake::TransformInputSlot {
                type_id: TypeId("Integer"),
                default: None,
                name: "Macro input",
            },
            aflak_cake::TransformInputSlot {
                type_id: TypeId("Integer"),
                default: Some(AlgoIO::Integer(0)),
                name: "Macro input",
            },
        ]
    )
}
