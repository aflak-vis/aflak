use std::io::Cursor;
use std::path::Path;

pub fn show_frame_and_wave<P: AsRef<Path>>(path: P) -> Cursor<String> {
    let path = path.as_ref().to_string_lossy();
    let ron = format!(
        r#"
(
    dst: (
        transforms: [
            ((6), (
                t: Function("slice_3d_to_2d"),
                input_defaults: [
                    None,
                    None,
                ],
            )),
            ((2), (
                t: Function("open_fits"),
                input_defaults: [
                    None,
                ],
            )),
            ((3), (
                t: Function("fits_to_image"),
                input_defaults: [
                    None,
                    Some(Integer(0)),
                    Some(Str("FLUX")),
                ],
            )),
            ((7), (
                t: Function("make_plane3d"),
                input_defaults: [
                    Some(Float3((100, 0, 0))),
                    Some(Float3((0, 0, 1))),
                    Some(Float3((0, 1, 0))),
                    Some(Integer(70)),
                    Some(Integer(70)),
                ],
            )),
            ((4), (
                t: Function("extract_wave"),
                input_defaults: [
                    None,
                    Some(Roi(All)),
                ],
            )),
            ((1), (
                t: Constant(Path({:?})),
                input_defaults: [
                ],
            )),
        ],
        edges: [
            ((
                t_idx: (7),
                output_i: (0),
            ), (
                t_idx: (6),
                input_i: (1),
            )),
            ((
                t_idx: (1),
                output_i: (0),
            ), (
                t_idx: (2),
                input_i: (0),
            )),
            ((
                t_idx: (2),
                output_i: (0),
            ), (
                t_idx: (3),
                input_i: (0),
            )),
            ((
                t_idx: (3),
                output_i: (0),
            ), (
                t_idx: (4),
                input_i: (0),
            )),
            ((
                t_idx: (3),
                output_i: (0),
            ), (
                t_idx: (6),
                input_i: (0),
            )),
        ],
        outputs: [
            ((2), Some((
                t_idx: (6),
                output_i: (0),
            ))),
            ((1), Some((
                t_idx: (4),
                output_i: (0),
            ))),
        ],
    ),
    node_states: [
        (Transform((1)), (
            selected: false,
            pos: (-785, -596),
            size: (217, 47.5),
        )),
        (Transform((2)), (
            selected: false,
            pos: (-300, -323),
            size: (72, 45.5),
        )),
        (Transform((3)), (
            selected: false,
            pos: (-147, -362),
            size: (121, 45.5),
        )),
        (Transform((4)), (
            selected: false,
            pos: (113, -391),
            size: (93, 45.5),
        )),
        (Transform((5)), (
            selected: false,
            pos: (51, -334),
            size: (44, 28.5),
        )),
        (Transform((6)), (
            selected: false,
            pos: (-48, -241),
            size: (107, 62.5),
        )),
        (Transform((7)), (
            selected: true,
            pos: (-408, -187),
            size: (231, 123.5),
        )),
        (Output((1)), (
            selected: false,
            pos: (314, -426),
            size: (135, 28.5),
        )),
        (Output((2)), (
            selected: false,
            pos: (247, -233),
            size: (135, 28.5),
        )),
    ],
    scrolling: (-818, -667),
)
    "#,
        path
    );
    Cursor::new(ron)
}

pub fn show_equivalent_width<P: AsRef<Path>>(path: P) -> Cursor<String> {
    let path = path.as_ref().to_string_lossy();
    let ron = format!(
        r#"
(
    dst: (
        transforms: [
            ((21), (
                t: Function("average"),
                input_defaults: [
                    None,
                    Some(Integer(3174)),
                    Some(Integer(3194)),
                ],
            )),
            ((25), (
                t: Function("image_min_max"),
                input_defaults: [
                    None,
                ],
            )),
            ((2), (
                t: Function("open_fits"),
                input_defaults: [
                    None,
                ],
            )),
            ((26), (
                t: Function("convert_to_logscale"),
                input_defaults: [
                    None,
                    Some(Float(1000)),
                    None,
                    None,
                ],
            )),
            ((19), (
                t: Function("average"),
                input_defaults: [
                    None,
                    Some(Integer(3134)),
                    Some(Integer(3154)),
                ],
            )),
            ((23), (
                t: Function("create_equivalent_width"),
                input_defaults: [
                    None,
                    None,
                    Some(Float(20)),
                    Some(Float(10000000000)),
                    Some(Bool(true)),
                ],
            )),
            ((24), (
                t: Function("ratio_from_bands"),
                input_defaults: [
                    None,
                    None,
                    None,
                ],
            )),
            ((22), (
                t: Function("linear_composition"),
                input_defaults: [
                    None,
                    None,
                    Some(Float(0.533333)),
                    Some(Float(0.467)),
                ],
            )),
            ((20), (
                t: Function("average"),
                input_defaults: [
                    None,
                    Some(Integer(3099)),
                    Some(Integer(3119)),
                ],
            )),
            ((3), (
                t: Function("fits_to_image"),
                input_defaults: [
                    None,
                    Some(Integer(0)),
                    Some(Str("FLUX")),
                ],
            )),
            ((1), (
                t: Constant(Path({:?})),
                input_defaults: [
                ],
            )),
        ],
        edges: [
            ((
                t_idx: (21),
                output_i: (0),
            ), (
                t_idx: (22),
                input_i: (1),
            )),
            ((
                t_idx: (24),
                output_i: (1),
            ), (
                t_idx: (22),
                input_i: (3),
            )),
            ((
                t_idx: (25),
                output_i: (0),
            ), (
                t_idx: (26),
                input_i: (2),
            )),
            ((
                t_idx: (2),
                output_i: (0),
            ), (
                t_idx: (3),
                input_i: (0),
            )),
            ((
                t_idx: (20),
                output_i: (0),
            ), (
                t_idx: (22),
                input_i: (0),
            )),
            ((
                t_idx: (25),
                output_i: (1),
            ), (
                t_idx: (26),
                input_i: (3),
            )),
            ((
                t_idx: (1),
                output_i: (0),
            ), (
                t_idx: (2),
                input_i: (0),
            )),
            ((
                t_idx: (20),
                output_i: (1),
            ), (
                t_idx: (24),
                input_i: (1),
            )),
            ((
                t_idx: (19),
                output_i: (0),
            ), (
                t_idx: (23),
                input_i: (1),
            )),
            ((
                t_idx: (22),
                output_i: (0),
            ), (
                t_idx: (23),
                input_i: (0),
            )),
            ((
                t_idx: (19),
                output_i: (1),
            ), (
                t_idx: (24),
                input_i: (0),
            )),
            ((
                t_idx: (21),
                output_i: (1),
            ), (
                t_idx: (24),
                input_i: (2),
            )),
            ((
                t_idx: (24),
                output_i: (0),
            ), (
                t_idx: (22),
                input_i: (2),
            )),
            ((
                t_idx: (3),
                output_i: (0),
            ), (
                t_idx: (19),
                input_i: (0),
            )),
            ((
                t_idx: (3),
                output_i: (0),
            ), (
                t_idx: (20),
                input_i: (0),
            )),
            ((
                t_idx: (3),
                output_i: (0),
            ), (
                t_idx: (21),
                input_i: (0),
            )),
            ((
                t_idx: (23),
                output_i: (0),
            ), (
                t_idx: (25),
                input_i: (0),
            )),
            ((
                t_idx: (23),
                output_i: (0),
            ), (
                t_idx: (26),
                input_i: (0),
            )),
        ],
        outputs: [
            ((6), Some((
                t_idx: (19),
                output_i: (0),
            ))),
            ((7), Some((
                t_idx: (22),
                output_i: (0),
            ))),
            ((8), Some((
                t_idx: (26),
                output_i: (0),
            ))),
        ],
    ),
    node_states: [
        (Transform((1)), (
            selected: false,
            pos: (-576, -816),
            size: (513, 200),
        )),
        (Transform((2)), (
            selected: false,
            pos: (-297, -601),
            size: (72, 46),
        )),
        (Transform((3)), (
            selected: false,
            pos: (-267, -539),
            size: (121, 46),
        )),
        (Transform((19)), (
            selected: false,
            pos: (-53, -631),
            size: (196, 84),
        )),
        (Transform((20)), (
            selected: false,
            pos: (-52, -515),
            size: (196, 84),
        )),
        (Transform((21)), (
            selected: false,
            pos: (-48, -421),
            size: (196, 84),
        )),
        (Transform((22)), (
            selected: false,
            pos: (348, -449),
            size: (210, 101),
        )),
        (Transform((23)), (
            selected: false,
            pos: (573, -621),
            size: (210, 120),
        )),
        (Transform((24)), (
            selected: false,
            pos: (204.29999, -356),
            size: (121, 80),
        )),
        (Transform((25)), (
            selected: false,
            pos: (779.6, -485.40002),
            size: (100, 45.400024),
        )),
        (Transform((26)), (
            selected: false,
            pos: (873.6, -634),
            size: (209.40002, 99),
        )),
        (Output((6)), (
            selected: false,
            pos: (314, -539),
            size: (72, 29),
        )),
        (Output((7)), (
            selected: false,
            pos: (628, -408),
            size: (72, 29),
        )),
        (Output((8)), (
            selected: true,
            pos: (1056.2999, -473.10004),
            size: (72, 28.100037),
        )),
    ],
    scrolling: (-578, -819),
)
        "#,
        path
    );
    Cursor::new(ron)
}

pub fn show_fits_cleaning<P: AsRef<Path>>(path: P) -> Cursor<String> {
    let path = path.as_ref().to_string_lossy();
    let ron = format!(
        r#"
(
    dst: (
        transforms: [
            ((6), (
                t: Function("slice_one_frame"),
                input_defaults: [
                    None,
                    Some(Integer(0)),
                ],
            )),
            ((8), (
                t: Function("clip_image"),
                input_defaults: [
                    None,
                    Some(Float(0)),
                    Some(Bool(false)),
                ],
            )),
            ((9), (
                t: Function("replace_nan_image"),
                input_defaults: [
                    None,
                    Some(Float(0)),
                ],
            )),
            ((4), (
                t: Function("extract_wave"),
                input_defaults: [
                    None,
                    Some(Roi(All)),
                ],
            )),
            ((1), (
                t: Constant(Path({:?})),
                input_defaults: [
                ],
            )),
            ((3), (
                t: Function("fits_to_image"),
                input_defaults: [
                    None,
                    Some(Integer(0)),
                    Some(Str("")),
                ],
            )),
            ((2), (
                t: Function("open_fits"),
                input_defaults: [
                    None,
                ],
            )),
        ],
        edges: [
            ((
                t_idx: (1),
                output_i: (0),
            ), (
                t_idx: (2),
                input_i: (0),
            )),
            ((
                t_idx: (8),
                output_i: (0),
            ), (
                t_idx: (6),
                input_i: (0),
            )),
            ((
                t_idx: (8),
                output_i: (0),
            ), (
                t_idx: (9),
                input_i: (0),
            )),
            ((
                t_idx: (3),
                output_i: (0),
            ), (
                t_idx: (8),
                input_i: (0),
            )),
            ((
                t_idx: (2),
                output_i: (0),
            ), (
                t_idx: (3),
                input_i: (0),
            )),
            ((
                t_idx: (9),
                output_i: (0),
            ), (
                t_idx: (4),
                input_i: (0),
            )),
        ],
        outputs: [
            ((1), Some((
                t_idx: (4),
                output_i: (0),
            ))),
            ((2), Some((
                t_idx: (6),
                output_i: (0),
            ))),
        ],
    ),
    node_states: [
        (Transform((1)), (
            selected: false,
            pos: (-819, -648),
            size: (415, 200),
        )),
        (Transform((2)), (
            selected: false,
            pos: (-364, -502),
            size: (72, 46),
        )),
        (Transform((3)), (
            selected: false,
            pos: (-273, -443),
            size: (217, 84),
        )),
        (Transform((4)), (
            selected: false,
            pos: (547, -570),
            size: (93, 63),
        )),
        (Transform((5)), (
            selected: false,
            pos: (51, -334),
            size: (44, 28.5),
        )),
        (Transform((6)), (
            selected: false,
            pos: (244, -424),
            size: (196, 65),
        )),
        (Transform((7)), (
            selected: false,
            pos: (-241, -273),
            size: (231, 124),
        )),
        (Transform((8)), (
            selected: false,
            pos: (-0.5, -491.5),
            size: (209.5, 83.5),
        )),
        (Transform((9)), (
            selected: false,
            pos: (267.59998, -533.80005),
            size: (209.40002, 64.80005),
        )),
        (Output((1)), (
            selected: true,
            pos: (710, -527),
            size: (72, 29),
        )),
        (Output((2)), (
            selected: false,
            pos: (501, -385),
            size: (72, 29),
        )),
    ],
    scrolling: (-818, -667),
)
        "#,
        path
    );
    Cursor::new(ron)
}

#[cfg(test)]
mod test {
    use node_editor::NodeEditor;
    use primitives;

    use super::{show_equivalent_width, show_fits_cleaning, show_frame_and_wave};
    use constant_editor::MyConstantEditor;

    #[test]
    fn import_frame_and_wave() {
        let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
        let transformations = transformations_ref.as_slice();

        let buf = show_frame_and_wave("file.fits");
        let editor = NodeEditor::from_export_buf(buf, transformations, MyConstantEditor);
        assert!(editor.is_ok());
    }

    #[test]
    fn import_equivalent_width() {
        let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
        let transformations = transformations_ref.as_slice();

        let buf = show_equivalent_width("file.fits");
        let editor = NodeEditor::from_export_buf(buf, transformations, MyConstantEditor);
        assert!(editor.is_ok());
    }

    #[test]
    fn import_with_windows_style_paths() {
        let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
        let transformations = transformations_ref.as_slice();

        let buf = show_equivalent_width(r"C:\path\to\fits\file.fits");
        let editor = NodeEditor::from_export_buf(buf, transformations, MyConstantEditor);
        assert!(editor.is_ok());
    }

    #[test]
    fn import_fits_cleaning() {
        let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
        let transformations = transformations_ref.as_slice();

        let buf = show_fits_cleaning("file.fits");
        let editor = NodeEditor::from_export_buf(buf, transformations, MyConstantEditor);
        assert!(editor.is_ok());
    }

}
