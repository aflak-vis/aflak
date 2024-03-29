use std::io::Cursor;
use std::path::Path;

pub fn select_template<P: AsRef<Path>>(
    template_name: &str,
    fits_path: P,
) -> Option<Cursor<String>> {
    match template_name {
        "waveform" => Some(show_frame_and_wave(fits_path)),
        "equivalent_width" => Some(show_equivalent_width(fits_path)),
        "fits_cleaning" => Some(show_fits_cleaning(fits_path)),
        "velocity_field" => Some(show_velocity_field(fits_path)),
        _ => None,
    }
}

pub fn show_frame_and_wave<P: AsRef<Path>>(path: P) -> Cursor<String> {
    let path = path.as_ref().to_string_lossy();
    let ron = format!(
        r#"
(
    dst: (
        main: (
            transforms: [
                ((None, 1), (
                    t: Constant(Paths(FileList([{:?}]))),
                    input_defaults: [
                    ],
                )),
                ((None, 2), (
                    t: Function("open_fits", 1, 0, 0),
                    input_defaults: [
                        None,
                    ],
                )),
                ((None, 3), (
                    t: Function("fits_to_image", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Integer(0)),
                        Some(Str("FLUX")),
                    ],
                )),
                ((None, 4), (
                    t: Function("extract_wave", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Roi(All)),
                    ],
                )),
                ((None, 5), (
                    t: Function("slice_one_frame", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Integer(100)),
                    ],
                )),
            ],
            edges: [
                ((
                    t_idx: (None, 1),
                    output_i: (0),
                ), (
                    t_idx: (None, 2),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 2),
                    output_i: (0),
                ), (
                    t_idx: (None, 3),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 3),
                    output_i: (0),
                ), (
                    t_idx: (None, 4),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 3),
                    output_i: (0),
                ), (
                    t_idx: (None, 5),
                    input_i: (0),
                )),
            ],
            outputs: [
                ((1), Some((
                    t_idx: (None, 4),
                    output_i: (0),
                )), "Spectrum"),
                ((2), Some((
                    t_idx: (None, 5),
                    output_i: (0),
                )), "Sliced Plane"),
            ],
        ),
        subs: [
        ],
    ),
    node_states: [
        (Transform((None, 1)), (
            pos: (-785, -596),
            size: (471, 200),
        )),
        (Transform((None, 2)), (
            pos: (-238, -417),
            size: (93, 46),
        )),
        (Transform((None, 3)), (
            pos: (-147, -362),
            size: (217, 84),
        )),
        (Transform((None, 4)), (
            pos: (113, -391),
            size: (114, 63),
        )),
        (Transform((None, 5)), (
            pos: (88, -252),
            size: (196, 65),
        )),
        (Output((1)), (
            pos: (314, -426),
            size: (72, 29),
        )),
        (Output((2)), (
            pos: (392, -271),
            size: (72, 29),
        )),
    ],
    scrolling: (-818, -667),
    nodes_edit: [
    ],
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
        main: (
            transforms: [
                ((None, 1), (
                    t: Constant(Paths(FileList([{:?}]))),
                    input_defaults: [
                    ],
                )),
                ((None, 2), (
                    t: Function("open_fits", 1, 0, 0),
                    input_defaults: [
                        None,
                    ],
                )),
                ((None, 3), (
                    t: Function("fits_to_image", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Integer(0)),
                        Some(Str("FLUX")),
                    ],
                )),
                ((None, 19), (
                    t: Function("average", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Integer(3134)),
                        Some(Integer(3154)),
                    ],
                )),
                ((None, 20), (
                    t: Function("average", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Integer(3099)),
                        Some(Integer(3119)),
                    ],
                )),
                ((None, 21), (
                    t: Function("average", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Integer(3174)),
                        Some(Integer(3194)),
                    ],
                )),
                ((None, 22), (
                    t: Function("linear_composition", 1, 0, 0),
                    input_defaults: [
                        None,
                        None,
                        Some(Float(0.533333)),
                        Some(Float(0.467)),
                    ],
                )),
                ((None, 23), (
                    t: Function("create_equivalent_width", 0, 1, 0),
                    input_defaults: [
                        None,
                        None,
                        Some(Float(20)),
                        Some(Float(10000000000)),
                        Some(Bool(true)),
                    ],
                )),
                ((None, 24), (
                    t: Function("ratio_from_bands", 1, 0, 0),
                    input_defaults: [
                        None,
                        None,
                        None,
                    ],
                )),
                ((None, 25), (
                    t: Function("image_min_max", 1, 0, 0),
                    input_defaults: [
                        None,
                    ],
                )),
                ((None, 26), (
                    t: Function("convert_to_logscale", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Float(1000)),
                        None,
                        None,
                    ],
                )),
            ],
            edges: [
                ((
                    t_idx: (None, 1),
                    output_i: (0),
                ), (
                    t_idx: (None, 2),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 2),
                    output_i: (0),
                ), (
                    t_idx: (None, 3),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 3),
                    output_i: (0),
                ), (
                    t_idx: (None, 19),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 3),
                    output_i: (0),
                ), (
                    t_idx: (None, 20),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 3),
                    output_i: (0),
                ), (
                    t_idx: (None, 21),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 19),
                    output_i: (0),
                ), (
                    t_idx: (None, 23),
                    input_i: (1),
                )),
                ((
                    t_idx: (None, 19),
                    output_i: (1),
                ), (
                    t_idx: (None, 24),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 20),
                    output_i: (0),
                ), (
                    t_idx: (None, 22),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 20),
                    output_i: (1),
                ), (
                    t_idx: (None, 24),
                    input_i: (1),
                )),
                ((
                    t_idx: (None, 21),
                    output_i: (0),
                ), (
                    t_idx: (None, 22),
                    input_i: (1),
                )),
                ((
                    t_idx: (None, 21),
                    output_i: (1),
                ), (
                    t_idx: (None, 24),
                    input_i: (2),
                )),
                ((
                    t_idx: (None, 22),
                    output_i: (0),
                ), (
                    t_idx: (None, 23),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 23),
                    output_i: (0),
                ), (
                    t_idx: (None, 25),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 23),
                    output_i: (0),
                ), (
                    t_idx: (None, 26),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 24),
                    output_i: (0),
                ), (
                    t_idx: (None, 22),
                    input_i: (2),
                )),
                ((
                    t_idx: (None, 24),
                    output_i: (1),
                ), (
                    t_idx: (None, 22),
                    input_i: (3),
                )),
                ((
                    t_idx: (None, 25),
                    output_i: (0),
                ), (
                    t_idx: (None, 26),
                    input_i: (2),
                )),
                ((
                    t_idx: (None, 25),
                    output_i: (1),
                ), (
                    t_idx: (None, 26),
                    input_i: (3),
                )),
            ],
            outputs: [
                ((6), Some((
                    t_idx: (None, 19),
                    output_i: (0),
                )), "Average of On-band"),
                ((7), Some((
                    t_idx: (None, 22),
                    output_i: (0),
                )), "Linear composited of Off-band"),
                ((8), Some((
                    t_idx: (None, 26),
                    output_i: (0),
                )), "EW Map"),
            ],
        ),
        subs: [
        ],
    ),
    node_states: [
        (Transform((None, 1)), (
            selected: false,
            pos: (-576, -816),
            size: (513, 200),
        )),
        (Transform((None, 2)), (
            selected: false,
            pos: (-297, -601),
            size: (72, 46),
        )),
        (Transform((None, 3)), (
            selected: false,
            pos: (-267, -539),
            size: (121, 46),
        )),
        (Transform((None, 19)), (
            selected: false,
            pos: (-53, -631),
            size: (196, 84),
        )),
        (Transform((None, 20)), (
            selected: false,
            pos: (-52, -515),
            size: (196, 84),
        )),
        (Transform((None, 21)), (
            selected: false,
            pos: (-48, -421),
            size: (196, 84),
        )),
        (Transform((None, 22)), (
            selected: false,
            pos: (348, -449),
            size: (210, 101),
        )),
        (Transform((None, 23)), (
            selected: false,
            pos: (573, -621),
            size: (210, 120),
        )),
        (Transform((None, 24)), (
            selected: false,
            pos: (204.29999, -356),
            size: (121, 80),
        )),
        (Transform((None, 25)), (
            selected: false,
            pos: (779.6, -485.40002),
            size: (100, 45.400024),
        )),
        (Transform((None, 26)), (
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
    nodes_edit: [
    ],
)
        "#,
        path
    );
    Cursor::new(ron)
}

pub fn show_velocity_field<P: AsRef<Path>>(path: P) -> Cursor<String> {
    let path = path.as_ref().to_string_lossy();
    let ron = format!(
        r#"
(
    dst: (
        main: (
            transforms: [
                ((None, 1), (
                    t: Constant(Paths(FileList([{:?}]))),
                    input_defaults: [
                    ],
                )),
                ((None, 2), (
                    t: Function("open_fits", 1, 0, 0),
                    input_defaults: [
                        None,
                    ],
                )),
                ((None, 3), (
                    t: Function("fits_to_image", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Integer(0)),
                        Some(Str("FLUX")),
                    ],
                )),
                ((None, 4), (
                    t: Function("extract_argmin_max_wavelength", 0, 1, 0),
                    input_defaults: [
                        None,
                        Some(Integer(3135)),
                        Some(Integer(3155)),
                        Some(Bool(false)),
                    ],
                )),
                ((None, 5), (
                    t: Function("extract_centrobaric_wavelength", 0, 1, 0),
                    input_defaults: [
                        None,
                        Some(Integer(3135)),
                        Some(Integer(3155)),
                    ],
                )),
                ((None, 6), (
                    t: Function("create_velocity_field_map", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Float(6765.6)),
                    ],
                )),
                ((None, 7), (
                    t: Function("create_velocity_field_map", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Float(6765.6)),
                    ],
                )),
            ],
            edges: [
                ((
                    t_idx: (None, 1),
                    output_i: (0),
                ), (
                    t_idx: (None, 2),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 2),
                    output_i: (0),
                ), (
                    t_idx: (None, 3),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 3),
                    output_i: (0),
                ), (
                    t_idx: (None, 4),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 3),
                    output_i: (0),
                ), (
                    t_idx: (None, 5),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 4),
                    output_i: (0),
                ), (
                    t_idx: (None, 6),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 5),
                    output_i: (0),
                ), (
                    t_idx: (None, 7),
                    input_i: (0),
                )),
            ],
            outputs: [
                ((1), Some((
                    t_idx: (None, 6),
                    output_i: (0),
                )), "VF Map using argminmax"),
                ((2), Some((
                    t_idx: (None, 7),
                    output_i: (0),
                )), "VF Map using centroid"),
            ],
        ),
        subs: [
        ],
    ),
    node_states: [
        (Transform((None, 1)), (
            pos: (-785, -596),
            size: (410, 200),
        )),
        (Transform((None, 2)), (
            pos: (-469, -361),
            size: (72, 46),
        )),
        (Transform((None, 3)), (
            pos: (-425, -298),
            size: (217, 84),
        )),
        (Transform((None, 4)), (
            pos: (-125.10004, -423.3),
            size: (212, 102.29999),
        )),
        (Transform((None, 5)), (
            pos: (-128.5, -150.70001),
            size: (219, 83.70001),
        )),
        (Transform((None, 6)), (
            pos: (207.19995, -401.80002),
            size: (209.80005, 64.80002),
        )),
        (Transform((None, 7)), (
            pos: (207.59998, -141.79999),
            size: (209.40002, 64.79999),
        )),
        (Output((1)), (
            pos: (489.90002, -383.1),
            size: (72, 29),
        )),
        (Output((2)), (
            pos: (480.69995, -123),
            size: (72, 29),
        )),
    ],
    scrolling: (-788, -596),
    nodes_edit: [
    ],
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
        main: (
            transforms: [
                ((None, 1), (
                    t: Constant(Paths(FileList([{:?}]))),
                    input_defaults: [
                    ],
                )),
                ((None, 2), (
                    t: Function("open_fits", 1, 0, 0),
                    input_defaults: [
                        None,
                    ],
                )),
                ((None, 3), (
                    t: Function("fits_to_image", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Integer(0)),
                        Some(Str("")),
                    ],
                )),
                ((None, 4), (
                    t: Function("extract_wave", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Roi(All)),
                    ],
                )),
                ((None, 6), (
                    t: Function("slice_one_frame", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Integer(0)),
                    ],
                )),
                ((None, 8), (
                    t: Function("clip_image", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Float(0)),
                        Some(Bool(false)),
                    ],
                )),
                ((None, 9), (
                    t: Function("replace_nan_image", 1, 0, 0),
                    input_defaults: [
                        None,
                        Some(Float(0)),
                    ],
                )),
            ],
            edges: [
                ((
                    t_idx: (None, 1),
                    output_i: (0),
                ), (
                    t_idx: (None, 2),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 2),
                    output_i: (0),
                ), (
                    t_idx: (None, 3),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 3),
                    output_i: (0),
                ), (
                    t_idx: (None, 8),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 8),
                    output_i: (0),
                ), (
                    t_idx: (None, 6),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 8),
                    output_i: (0),
                ), (
                    t_idx: (None, 9),
                    input_i: (0),
                )),
                ((
                    t_idx: (None, 9),
                    output_i: (0),
                ), (
                    t_idx: (None, 4),
                    input_i: (0),
                )),
            ],
            outputs: [
                ((1), Some((
                    t_idx: (None, 4),
                    output_i: (0),
                )), "Spectrum"),
                ((2), Some((
                    t_idx: (None, 6),
                    output_i: (0),
                )), "Sliced Plane"),
                ((3), Some((
                    t_idx: (None, 2),
                    output_i: (0),
                )), "FITS Headers"),
            ],
        ),
        subs: [
        ],
    ),
    node_states: [
        (Transform((None, 1)), (
            selected: false,
            pos: (-819, -648),
            size: (415, 200),
        )),
        (Transform((None, 2)), (
            selected: false,
            pos: (-364, -502),
            size: (72, 46),
        )),
        (Transform((None, 3)), (
            selected: false,
            pos: (-273, -443),
            size: (217, 84),
        )),
        (Transform((None, 4)), (
            selected: false,
            pos: (547, -570),
            size: (93, 63),
        )),
        (Transform((None, 5)), (
            selected: false,
            pos: (51, -334),
            size: (44, 28.5),
        )),
        (Transform((None, 6)), (
            selected: false,
            pos: (244, -424),
            size: (196, 65),
        )),
        (Transform((None, 7)), (
            selected: false,
            pos: (-241, -273),
            size: (231, 124),
        )),
        (Transform((None, 8)), (
            selected: false,
            pos: (-0.5, -491.5),
            size: (209.5, 83.5),
        )),
        (Transform((None, 9)), (
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
        (Output((3)), (
            selected: false,
            pos: (-176, -566.30005),
            size: (72, 28.300049),
        )),
    ],
    scrolling: (-818, -667),
    nodes_edit: [
    ],
)
        "#,
        path
    );
    Cursor::new(ron)
}

#[cfg(test)]
mod test {
    use crate::aflak::AflakNodeEditor;

    use super::{
        show_equivalent_width, show_fits_cleaning, show_frame_and_wave, show_velocity_field,
    };

    #[test]
    fn import_frame_and_wave() {
        let buf = show_frame_and_wave("file.fits");
        let editor = AflakNodeEditor::from_export_buf(buf);
        assert!(editor.is_ok());
    }

    #[test]
    fn import_equivalent_width() {
        let buf = show_equivalent_width("file.fits");
        let editor = AflakNodeEditor::from_export_buf(buf);
        assert!(editor.is_ok());
    }

    #[test]
    fn import_with_windows_style_paths() {
        let buf = show_equivalent_width(r"C:\path\to\fits\file.fits");
        let editor = AflakNodeEditor::from_export_buf(buf);
        assert!(editor.is_ok());
    }

    #[test]
    fn import_fits_cleaning() {
        let buf = show_fits_cleaning("file.fits");
        let editor = AflakNodeEditor::from_export_buf(buf);
        assert!(editor.is_ok());
    }

    #[test]
    fn import_velocity_field() {
        let buf = show_velocity_field("file.fits");
        let editor = AflakNodeEditor::from_export_buf(buf);
        assert!(editor.is_ok());
    }
}
