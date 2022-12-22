use itertools::Itertools;
use std::iter;
use std::slice;

use super::super::util;

const LUT_SIZE: usize = 65536;

#[derive(Clone)]
pub struct ColorLUT {
    /// Linear gradient
    /// Takes a series of color stops that indicate how to interpolate between the colors
    read_mode: ReadMode,
    gradient: Vec<(f32, [u8; 3])>,
    gradient_alpha: Vec<(f32, u8)>,
    lut: Box<[[u8; 4]; LUT_SIZE]>,
    lims: (f32, f32, f32),
}

#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq)]
pub enum BuiltinLUT {
    Grey,
    GreyClip,
    Thermal,
    Flame,
    Yellowy,
    HeatMap,
    HeatMapInv,
    Red,
    Blue,
    Green,
    JetColor,
    TurboColor,
}

#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq)]
pub enum ReadMode {
    RGB,
    HSV,
    LAB,
}

impl From<BuiltinLUT> for (Vec<(f32, [u8; 3])>, Vec<(f32, u8)>, ReadMode) {
    fn from(lut: BuiltinLUT) -> Self {
        (
            lut.lut().gradient,
            lut.lut().gradient_alpha,
            lut.lut().read_mode,
        )
    }
}

impl From<String> for BuiltinLUT {
    fn from(name: String) -> Self {
        match name.as_str() {
            "Grey" => BuiltinLUT::Grey,
            "GreyClip" => BuiltinLUT::GreyClip,
            "Thermal" => BuiltinLUT::Thermal,
            "Flame" => BuiltinLUT::Flame,
            "Yellowy" => BuiltinLUT::Yellowy,
            "HeatMap" => BuiltinLUT::HeatMap,
            "HeatMapInv" => BuiltinLUT::HeatMapInv,
            "Red" => BuiltinLUT::Red,
            "Blue" => BuiltinLUT::Blue,
            "Green" => BuiltinLUT::Green,
            "JetColor" => BuiltinLUT::JetColor,
            "TurboColor" => BuiltinLUT::TurboColor,
            _ => unimplemented!(),
        }
    }
}

impl BuiltinLUT {
    pub fn values() -> slice::Iter<'static, Self> {
        use self::BuiltinLUT::*;
        const VALUES: [BuiltinLUT; 12] = [
            Grey, GreyClip, Thermal, Flame, Yellowy, HeatMap, HeatMapInv, Red, Blue, Green,
            JetColor, TurboColor,
        ];
        VALUES.iter()
    }

    pub fn name(self) -> &'static str {
        match self {
            BuiltinLUT::Grey => &"Grey",
            BuiltinLUT::GreyClip => &"GreyClip",
            BuiltinLUT::Yellowy => &"Yellowy",
            BuiltinLUT::Thermal => &"Thermal",
            BuiltinLUT::Flame => &"Flame",
            BuiltinLUT::HeatMap => &"HeatMap",
            BuiltinLUT::HeatMapInv => &"HeatMap_Inv",
            BuiltinLUT::Red => &"Red",
            BuiltinLUT::Green => &"Green",
            BuiltinLUT::Blue => &"Blue",
            BuiltinLUT::JetColor => &"JetColor",
            BuiltinLUT::TurboColor => &"TurboColor",
        }
    }

    pub fn lut(self) -> ColorLUT {
        match self {
            BuiltinLUT::Grey => ColorLUT::linear(
                vec![(0.0, [0, 0, 0]), (1.0, [255, 255, 255])],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::RGB,
            ),
            BuiltinLUT::GreyClip => ColorLUT::linear(
                vec![
                    (0.0, [0, 0, 0]),
                    (0.99, [255, 255, 255]),
                    (1.0, [255, 0, 0]),
                ],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::RGB,
            ),
            BuiltinLUT::Yellowy => ColorLUT::linear(
                vec![
                    (0.0, [0, 0, 0]),
                    (0.25, [32, 0, 129]),
                    (0.5, [115, 15, 255]),
                    (0.75, [255, 255, 0]),
                    (1.0, [255, 255, 255]),
                ],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::RGB,
            ),
            BuiltinLUT::Thermal => ColorLUT::linear(
                vec![
                    (0.0, [0, 0, 0]),
                    (1.0 / 3.0, [185, 0, 0]),
                    (2.0 / 3.0, [255, 220, 0]),
                    (1.0, [255, 255, 255]),
                ],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::RGB,
            ),
            BuiltinLUT::Flame => ColorLUT::linear(
                vec![
                    (0.0, [0, 0, 0]),
                    (0.2, [7, 0, 220]),
                    (0.5, [236, 0, 134]),
                    (0.8, [246, 246, 0]),
                    (1.0, [255, 255, 255]),
                ],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::RGB,
            ),
            BuiltinLUT::HeatMap => ColorLUT::linear(
                vec![
                    (0.0, [1, 1, 85]),
                    (0.1, [0, 0, 255]),
                    (0.25, [0, 255, 255]),
                    (0.5, [0, 255, 0]),
                    (0.75, [255, 255, 0]),
                    (0.9, [255, 0, 0]),
                    (0.99, [108, 6, 10]),
                    (1.0, [255, 255, 255]),
                ],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::RGB,
            ),
            BuiltinLUT::HeatMapInv => ColorLUT::linear(
                vec![
                    (0.0, [108, 6, 10]),
                    (0.1, [255, 0, 0]),
                    (0.25, [255, 255, 0]),
                    (0.5, [0, 255, 0]),
                    (0.75, [0, 255, 255]),
                    (0.89, [0, 0, 255]),
                    (0.99, [1, 1, 85]),
                    (1.0, [255, 255, 255]),
                ],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::RGB,
            ),
            BuiltinLUT::Red => ColorLUT::linear(
                vec![(0.0, [0, 0, 0]), (1.0, [255, 0, 0])],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::RGB,
            ),
            BuiltinLUT::Green => ColorLUT::linear(
                vec![(0.0, [0, 0, 0]), (1.0, [0, 255, 0])],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::RGB,
            ),
            BuiltinLUT::Blue => ColorLUT::linear(
                vec![(0.0, [0, 0, 0]), (1.0, [0, 0, 255])],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::RGB,
            ),
            BuiltinLUT::JetColor => ColorLUT::linear(
                vec![(0.0, [170, 255, 255]), (1.0, [0, 255, 255])],
                vec![(0.0, 0), (1.0, 255)],
                ReadMode::HSV,
            ),
            BuiltinLUT::TurboColor => {
                let turbo_lut = vec![
                    [0.18995, 0.07176, 0.23217],
                    [0.19483, 0.08339, 0.26149],
                    [0.19956, 0.09498, 0.29024],
                    [0.20415, 0.10652, 0.31844],
                    [0.20860, 0.11802, 0.34607],
                    [0.21291, 0.12947, 0.37314],
                    [0.21708, 0.14087, 0.39964],
                    [0.22111, 0.15223, 0.42558],
                    [0.22500, 0.16354, 0.45096],
                    [0.22875, 0.17481, 0.47578],
                    [0.23236, 0.18603, 0.50004],
                    [0.23582, 0.19720, 0.52373],
                    [0.23915, 0.20833, 0.54686],
                    [0.24234, 0.21941, 0.56942],
                    [0.24539, 0.23044, 0.59142],
                    [0.24830, 0.24143, 0.61286],
                    [0.25107, 0.25237, 0.63374],
                    [0.25369, 0.26327, 0.65406],
                    [0.25618, 0.27412, 0.67381],
                    [0.25853, 0.28492, 0.69300],
                    [0.26074, 0.29568, 0.71162],
                    [0.26280, 0.30639, 0.72968],
                    [0.26473, 0.31706, 0.74718],
                    [0.26652, 0.32768, 0.76412],
                    [0.26816, 0.33825, 0.78050],
                    [0.26967, 0.34878, 0.79631],
                    [0.27103, 0.35926, 0.81156],
                    [0.27226, 0.36970, 0.82624],
                    [0.27334, 0.38008, 0.84037],
                    [0.27429, 0.39043, 0.85393],
                    [0.27509, 0.40072, 0.86692],
                    [0.27576, 0.41097, 0.87936],
                    [0.27628, 0.42118, 0.89123],
                    [0.27667, 0.43134, 0.90254],
                    [0.27691, 0.44145, 0.91328],
                    [0.27701, 0.45152, 0.92347],
                    [0.27698, 0.46153, 0.93309],
                    [0.27680, 0.47151, 0.94214],
                    [0.27648, 0.48144, 0.95064],
                    [0.27603, 0.49132, 0.95857],
                    [0.27543, 0.50115, 0.96594],
                    [0.27469, 0.51094, 0.97275],
                    [0.27381, 0.52069, 0.97899],
                    [0.27273, 0.53040, 0.98461],
                    [0.27106, 0.54015, 0.98930],
                    [0.26878, 0.54995, 0.99303],
                    [0.26592, 0.55979, 0.99583],
                    [0.26252, 0.56967, 0.99773],
                    [0.25862, 0.57958, 0.99876],
                    [0.25425, 0.58950, 0.99896],
                    [0.24946, 0.59943, 0.99835],
                    [0.24427, 0.60937, 0.99697],
                    [0.23874, 0.61931, 0.99485],
                    [0.23288, 0.62923, 0.99202],
                    [0.22676, 0.63913, 0.98851],
                    [0.22039, 0.64901, 0.98436],
                    [0.21382, 0.65886, 0.97959],
                    [0.20708, 0.66866, 0.97423],
                    [0.20021, 0.67842, 0.96833],
                    [0.19326, 0.68812, 0.96190],
                    [0.18625, 0.69775, 0.95498],
                    [0.17923, 0.70732, 0.94761],
                    [0.17223, 0.71680, 0.93981],
                    [0.16529, 0.72620, 0.93161],
                    [0.15844, 0.73551, 0.92305],
                    [0.15173, 0.74472, 0.91416],
                    [0.14519, 0.75381, 0.90496],
                    [0.13886, 0.76279, 0.89550],
                    [0.13278, 0.77165, 0.88580],
                    [0.12698, 0.78037, 0.87590],
                    [0.12151, 0.78896, 0.86581],
                    [0.11639, 0.79740, 0.85559],
                    [0.11167, 0.80569, 0.84525],
                    [0.10738, 0.81381, 0.83484],
                    [0.10357, 0.82177, 0.82437],
                    [0.10026, 0.82955, 0.81389],
                    [0.09750, 0.83714, 0.80342],
                    [0.09532, 0.84455, 0.79299],
                    [0.09377, 0.85175, 0.78264],
                    [0.09287, 0.85875, 0.77240],
                    [0.09267, 0.86554, 0.76230],
                    [0.09320, 0.87211, 0.75237],
                    [0.09451, 0.87844, 0.74265],
                    [0.09662, 0.88454, 0.73316],
                    [0.09958, 0.89040, 0.72393],
                    [0.10342, 0.89600, 0.71500],
                    [0.10815, 0.90142, 0.70599],
                    [0.11374, 0.90673, 0.69651],
                    [0.12014, 0.91193, 0.68660],
                    [0.12733, 0.91701, 0.67627],
                    [0.13526, 0.92197, 0.66556],
                    [0.14391, 0.92680, 0.65448],
                    [0.15323, 0.93151, 0.64308],
                    [0.16319, 0.93609, 0.63137],
                    [0.17377, 0.94053, 0.61938],
                    [0.18491, 0.94484, 0.60713],
                    [0.19659, 0.94901, 0.59466],
                    [0.20877, 0.95304, 0.58199],
                    [0.22142, 0.95692, 0.56914],
                    [0.23449, 0.96065, 0.55614],
                    [0.24797, 0.96423, 0.54303],
                    [0.26180, 0.96765, 0.52981],
                    [0.27597, 0.97092, 0.51653],
                    [0.29042, 0.97403, 0.50321],
                    [0.30513, 0.97697, 0.48987],
                    [0.32006, 0.97974, 0.47654],
                    [0.33517, 0.98234, 0.46325],
                    [0.35043, 0.98477, 0.45002],
                    [0.36581, 0.98702, 0.43688],
                    [0.38127, 0.98909, 0.42386],
                    [0.39678, 0.99098, 0.41098],
                    [0.41229, 0.99268, 0.39826],
                    [0.42778, 0.99419, 0.38575],
                    [0.44321, 0.99551, 0.37345],
                    [0.45854, 0.99663, 0.36140],
                    [0.47375, 0.99755, 0.34963],
                    [0.48879, 0.99828, 0.33816],
                    [0.50362, 0.99879, 0.32701],
                    [0.51822, 0.99910, 0.31622],
                    [0.53255, 0.99919, 0.30581],
                    [0.54658, 0.99907, 0.29581],
                    [0.56026, 0.99873, 0.28623],
                    [0.57357, 0.99817, 0.27712],
                    [0.58646, 0.99739, 0.26849],
                    [0.59891, 0.99638, 0.26038],
                    [0.61088, 0.99514, 0.25280],
                    [0.62233, 0.99366, 0.24579],
                    [0.63323, 0.99195, 0.23937],
                    [0.64362, 0.98999, 0.23356],
                    [0.65394, 0.98775, 0.22835],
                    [0.66428, 0.98524, 0.22370],
                    [0.67462, 0.98246, 0.21960],
                    [0.68494, 0.97941, 0.21602],
                    [0.69525, 0.97610, 0.21294],
                    [0.70553, 0.97255, 0.21032],
                    [0.71577, 0.96875, 0.20815],
                    [0.72596, 0.96470, 0.20640],
                    [0.73610, 0.96043, 0.20504],
                    [0.74617, 0.95593, 0.20406],
                    [0.75617, 0.95121, 0.20343],
                    [0.76608, 0.94627, 0.20311],
                    [0.77591, 0.94113, 0.20310],
                    [0.78563, 0.93579, 0.20336],
                    [0.79524, 0.93025, 0.20386],
                    [0.80473, 0.92452, 0.20459],
                    [0.81410, 0.91861, 0.20552],
                    [0.82333, 0.91253, 0.20663],
                    [0.83241, 0.90627, 0.20788],
                    [0.84133, 0.89986, 0.20926],
                    [0.85010, 0.89328, 0.21074],
                    [0.85868, 0.88655, 0.21230],
                    [0.86709, 0.87968, 0.21391],
                    [0.87530, 0.87267, 0.21555],
                    [0.88331, 0.86553, 0.21719],
                    [0.89112, 0.85826, 0.21880],
                    [0.89870, 0.85087, 0.22038],
                    [0.90605, 0.84337, 0.22188],
                    [0.91317, 0.83576, 0.22328],
                    [0.92004, 0.82806, 0.22456],
                    [0.92666, 0.82025, 0.22570],
                    [0.93301, 0.81236, 0.22667],
                    [0.93909, 0.80439, 0.22744],
                    [0.94489, 0.79634, 0.22800],
                    [0.95039, 0.78823, 0.22831],
                    [0.95560, 0.78005, 0.22836],
                    [0.96049, 0.77181, 0.22811],
                    [0.96507, 0.76352, 0.22754],
                    [0.96931, 0.75519, 0.22663],
                    [0.97323, 0.74682, 0.22536],
                    [0.97679, 0.73842, 0.22369],
                    [0.98000, 0.73000, 0.22161],
                    [0.98289, 0.72140, 0.21918],
                    [0.98549, 0.71250, 0.21650],
                    [0.98781, 0.70330, 0.21358],
                    [0.98986, 0.69382, 0.21043],
                    [0.99163, 0.68408, 0.20706],
                    [0.99314, 0.67408, 0.20348],
                    [0.99438, 0.66386, 0.19971],
                    [0.99535, 0.65341, 0.19577],
                    [0.99607, 0.64277, 0.19165],
                    [0.99654, 0.63193, 0.18738],
                    [0.99675, 0.62093, 0.18297],
                    [0.99672, 0.60977, 0.17842],
                    [0.99644, 0.59846, 0.17376],
                    [0.99593, 0.58703, 0.16899],
                    [0.99517, 0.57549, 0.16412],
                    [0.99419, 0.56386, 0.15918],
                    [0.99297, 0.55214, 0.15417],
                    [0.99153, 0.54036, 0.14910],
                    [0.98987, 0.52854, 0.14398],
                    [0.98799, 0.51667, 0.13883],
                    [0.98590, 0.50479, 0.13367],
                    [0.98360, 0.49291, 0.12849],
                    [0.98108, 0.48104, 0.12332],
                    [0.97837, 0.46920, 0.11817],
                    [0.97545, 0.45740, 0.11305],
                    [0.97234, 0.44565, 0.10797],
                    [0.96904, 0.43399, 0.10294],
                    [0.96555, 0.42241, 0.09798],
                    [0.96187, 0.41093, 0.09310],
                    [0.95801, 0.39958, 0.08831],
                    [0.95398, 0.38836, 0.08362],
                    [0.94977, 0.37729, 0.07905],
                    [0.94538, 0.36638, 0.07461],
                    [0.94084, 0.35566, 0.07031],
                    [0.93612, 0.34513, 0.06616],
                    [0.93125, 0.33482, 0.06218],
                    [0.92623, 0.32473, 0.05837],
                    [0.92105, 0.31489, 0.05475],
                    [0.91572, 0.30530, 0.05134],
                    [0.91024, 0.29599, 0.04814],
                    [0.90463, 0.28696, 0.04516],
                    [0.89888, 0.27824, 0.04243],
                    [0.89298, 0.26981, 0.03993],
                    [0.88691, 0.26152, 0.03753],
                    [0.88066, 0.25334, 0.03521],
                    [0.87422, 0.24526, 0.03297],
                    [0.86760, 0.23730, 0.03082],
                    [0.86079, 0.22945, 0.02875],
                    [0.85380, 0.22170, 0.02677],
                    [0.84662, 0.21407, 0.02487],
                    [0.83926, 0.20654, 0.02305],
                    [0.83172, 0.19912, 0.02131],
                    [0.82399, 0.19182, 0.01966],
                    [0.81608, 0.18462, 0.01809],
                    [0.80799, 0.17753, 0.01660],
                    [0.79971, 0.17055, 0.01520],
                    [0.79125, 0.16368, 0.01387],
                    [0.78260, 0.15693, 0.01264],
                    [0.77377, 0.15028, 0.01148],
                    [0.76476, 0.14374, 0.01041],
                    [0.75556, 0.13731, 0.00942],
                    [0.74617, 0.13098, 0.00851],
                    [0.73661, 0.12477, 0.00769],
                    [0.72686, 0.11867, 0.00695],
                    [0.71692, 0.11268, 0.00629],
                    [0.70680, 0.10680, 0.00571],
                    [0.69650, 0.10102, 0.00522],
                    [0.68602, 0.09536, 0.00481],
                    [0.67535, 0.08980, 0.00449],
                    [0.66449, 0.08436, 0.00424],
                    [0.65345, 0.07902, 0.00408],
                    [0.64223, 0.07380, 0.00401],
                    [0.63082, 0.06868, 0.00401],
                    [0.61923, 0.06367, 0.00410],
                    [0.60746, 0.05878, 0.00427],
                    [0.59550, 0.05399, 0.00453],
                    [0.58336, 0.04931, 0.00486],
                    [0.57103, 0.04474, 0.00529],
                    [0.55852, 0.04028, 0.00579],
                    [0.54583, 0.03593, 0.00638],
                    [0.53295, 0.03169, 0.00705],
                    [0.51989, 0.02756, 0.00780],
                    [0.50664, 0.02354, 0.00863],
                    [0.49321, 0.01963, 0.00955],
                    [0.47960, 0.01583, 0.01055],
                ];
                let mut colors = vec![];
                for (k, c) in turbo_lut.iter().enumerate() {
                    colors.push((
                        k as f32 / 256.0,
                        [
                            (c[0] * 255.0) as u8,
                            (c[1] * 255.0) as u8,
                            (c[2] * 255.0) as u8,
                        ],
                    ));
                }
                ColorLUT::linear(colors, vec![(0.0, 0), (1.0, 255)], ReadMode::RGB)
            }
        }
    }
}

impl ColorLUT {
    /// Create a linear gradient.
    pub fn linear<T: Into<f32>>(
        colors: Vec<(T, [u8; 3])>,
        alpha: Vec<(T, u8)>,
        read_mode: ReadMode,
    ) -> ColorLUT {
        let mut vec = Vec::with_capacity(colors.len());
        for (c, color) in colors {
            vec.push((c.into(), color))
        }
        let mut vec_alpha = Vec::with_capacity(alpha.len());
        for (c, alpha) in alpha {
            vec_alpha.push((c.into(), alpha));
        }
        let mut color_lut = ColorLUT {
            read_mode,
            gradient: vec,
            gradient_alpha: vec_alpha,
            lut: Box::new([[0; 4]; LUT_SIZE]),
            lims: (0.0, 0.5, 1.0),
        };
        color_lut.lut_init();
        color_lut
    }

    pub fn color_at_bounds(&self, point: f32, vmin: f32, vmax: f32) -> [u8; 4] {
        let point = util::clamp(point, vmin, vmax);
        self.color_at((point - vmin) / (vmax - vmin))
    }

    pub fn color_at(&self, point: f32) -> [u8; 4] {
        let c = (point - self.lims.0) / (self.lims.2 - self.lims.0);
        let mut i = self.mtf(self.lims.1, c) * (LUT_SIZE - 1) as f32;

        if i < 0.0 {
            i = 0.0
        }
        let mut i = i as usize;
        if i >= LUT_SIZE {
            i = LUT_SIZE - 1;
        }
        self.lut[i]
    }

    /// Midtone Transfer Function (MTF)
    pub fn mtf(&self, m: f32, x: f32) -> f32 {
        if x >= 1.0 {
            1.0
        } else if x <= 0.0 {
            0.0
        } else {
            (m - 1.0) * x / ((2.0 * m - 1.0) * x - m)
        }
    }

    fn color_at_init(&self, point: f32) -> [u8; 3] {
        for ((v1, c1), (v2, c2)) in self.bounds() {
            let dv = v2 - v1;
            if v1 <= point && point <= v2 {
                let [r1, g1, b1] = c1;
                let [r2, g2, b2] = c2;
                return if dv == 0.0 {
                    c1
                } else {
                    let r1 = f32::from(r1);
                    let r2 = f32::from(r2);
                    let g1 = f32::from(g1);
                    let g2 = f32::from(g2);
                    let b1 = f32::from(b1);
                    let b2 = f32::from(b2);
                    let dp = point - v1;
                    let coef = dp / dv;
                    [
                        (r1 + (r2 - r1) * coef) as u8,
                        (g1 + (g2 - g1) * coef) as u8,
                        (b1 + (b2 - b1) * coef) as u8,
                    ]
                };
            }
        }
        [0, 0, 0]
    }

    fn alpha_at_init(&self, point: f32) -> u8 {
        for ((v1, a1), (v2, a2)) in self.gradient_alpha.iter().tuple_windows() {
            let dv = v2 - v1;
            if *v1 <= point && point <= *v2 {
                return if dv == 0.0 {
                    *a1
                } else {
                    let dp = point - *v1;
                    let coef = dp / dv;
                    let a1 = f32::from(*a1);
                    let a2 = f32::from(*a2);
                    (a1 + (a2 - a1) * coef) as u8
                };
            }
        }
        0
    }

    fn lut_init(&mut self) {
        for i in 0..LUT_SIZE {
            match self.read_mode {
                ReadMode::RGB => {
                    let color = self.color_at_init(i as f32 / (LUT_SIZE - 1) as f32);
                    let alpha = self.alpha_at_init(i as f32 / (LUT_SIZE - 1) as f32);
                    self.lut[i] = [color[0], color[1], color[2], alpha];
                }
                ReadMode::HSV => {
                    let color = self.color_at_init_hsv(i as f32 / (LUT_SIZE - 1) as f32);
                    let alpha = self.alpha_at_init(i as f32 / (LUT_SIZE - 1) as f32);
                    self.lut[i] = [color[0], color[1], color[2], alpha];
                }
                ReadMode::LAB => {
                    unimplemented!()
                }
            }
        }
    }

    fn hsv2rgb(&self, hsv: [u8; 3]) -> [u8; 3] {
        let h = hsv[0] as f32 / 255.0 * 360.0;
        let (s, v) = (hsv[1] as f32, hsv[2] as f32);
        let max = v as f32;
        let min = max - ((s / 255.0) * max);
        if 0.0 <= h && h < 60.0 {
            let g = (h / 60.0) * (max - min) + min;
            [max as u8, g as u8, min as u8]
        } else if 60.0 <= h && h < 120.0 {
            let r = ((120.0 - h) / 60.0) * (max - min) + min;
            [r as u8, max as u8, min as u8]
        } else if 120.0 <= h && h < 180.0 {
            let b = ((h - 120.0) / 60.0) * (max - min) + min;
            [min as u8, max as u8, b as u8]
        } else if 180.0 <= h && h < 240.0 {
            let g = ((240.0 - h) / 60.0) * (max - min) + min;
            [min as u8, g as u8, max as u8]
        } else if 240.0 <= h && h < 300.0 {
            let r = ((h - 240.0) / 60.0) * (max - min) + min;
            [r as u8, min as u8, max as u8]
        } else if 300.0 <= h && h < 360.0 {
            let b = ((360.0 - h) / 60.0) * (max - min) + min;
            [max as u8, min as u8, b as u8]
        } else {
            [0, 0, 0]
        }
    }

    fn color_at_init_hsv(&self, point: f32) -> [u8; 3] {
        for ((val1, c1), (val2, c2)) in self.bounds() {
            let dv = val2 - val1;
            if val1 <= point && point <= val2 {
                let [h1, s1, v1] = c1;
                let [h2, s2, v2] = c2;
                return if dv == 0.0 {
                    self.hsv2rgb(c1)
                } else {
                    let h1 = f32::from(h1);
                    let h2 = f32::from(h2);
                    let s1 = f32::from(s1);
                    let s2 = f32::from(s2);
                    let v1 = f32::from(v1);
                    let v2 = f32::from(v2);
                    let dp = point - val1;
                    let coef = dp / dv;
                    self.hsv2rgb([
                        (h1 + (h2 - h1) * coef) as u8,
                        (s1 + (s2 - s1) * coef) as u8,
                        (v1 + (v2 - v1) * coef) as u8,
                    ])
                };
            }
        }
        [0, 0, 0]
    }

    pub fn bounds(&self) -> iter::Zip<StopIter, iter::Skip<StopIter>> {
        let first_color = StopIter::new(self);
        let next_color = first_color.skip(1);
        first_color.zip(next_color)
    }

    pub fn set_min(&mut self, mut min: f32) {
        if min < 0.0 {
            min = 0.0;
        } else if min > 1.0 {
            min = 1.0;
        }
        if min > self.lims.2 {
            self.lims.2 = min;
        }
        self.lims.0 = min;
    }

    pub fn set_mid(&mut self, mut mid: f32) {
        if mid < 0.0 {
            mid = 0.0;
        } else if mid > 1.0 {
            mid = 1.0;
        }
        self.lims.1 = mid;
    }

    pub fn set_max(&mut self, mut max: f32) {
        if max < 0.0 {
            max = 0.0;
        } else if max > 1.0 {
            max = 1.0;
        }
        if max < self.lims.0 {
            self.lims.0 = max;
        }
        self.lims.2 = max;
    }

    pub fn set_lims(&mut self, min: f32, mid: f32, max: f32) {
        self.set_min(min);
        self.set_mid(mid);
        self.set_max(max);
    }

    pub fn lims(&self) -> (f32, f32, f32) {
        self.lims
    }

    pub fn gradient(&self) -> Vec<(f32, [u8; 3])> {
        self.gradient.clone()
    }

    pub fn gradient_mut(&mut self) -> &mut Vec<(f32, [u8; 3])> {
        &mut self.gradient
    }

    pub fn gradient_alpha(&self) -> Vec<(f32, u8)> {
        self.gradient_alpha.clone()
    }

    pub fn read_mode(&self) -> ReadMode {
        self.read_mode
    }

    pub fn set_gradient<G: Into<(Vec<(f32, [u8; 3])>, Vec<(f32, u8)>, ReadMode)>>(
        &mut self,
        gradient: G,
    ) {
        (self.gradient, self.gradient_alpha, self.read_mode) = gradient.into();
        self.lut_init();
    }
}

#[derive(Copy, Clone)]
pub struct StopIter<'a> {
    lut: &'a ColorLUT,
    i: usize,
}

impl<'a> StopIter<'a> {
    fn new(lut: &'a ColorLUT) -> Self {
        Self { lut, i: 0 }
    }
}

impl<'a> Iterator for StopIter<'a> {
    type Item = (f32, [u8; 3]);
    fn next(&mut self) -> Option<Self::Item> {
        let grad = &self.lut.gradient;
        if grad.is_empty() {
            None
        } else {
            let i = self.i;
            self.i += 1;
            if i == 0 {
                Some((0.0, grad[0].1))
            } else if i - 1 == grad.len() {
                Some((1.0, grad[grad.len() - 1].1))
            } else {
                self.lut.gradient.get(i - 1).map(|value| {
                    (
                        self.lut.lims.0 + (self.lut.lims.2 - self.lut.lims.0) * value.0,
                        value.1,
                    )
                })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::ColorLUT;
    #[test]
    fn test_color_at() {
        let lut = ColorLUT::linear(vec![
            (0.0, [0, 0, 255]),
            (0.5, [255, 255, 255]),
            (1.0, [255, 0, 0]),
        ]);
        assert_eq!(lut.color_at(0.0), [0, 0, 255]);
        assert_eq!(lut.color_at(1.0), [255, 0, 0]);
        assert_eq!(lut.color_at(0.5), [254, 254, 255]);
        assert_eq!(lut.color_at(0.25), [127, 127, 255]);
    }

    #[test]
    fn test_bounds() {
        let lut = ColorLUT::linear(vec![
            (0.0, [0, 0, 255]),
            (0.5, [255, 255, 255]),
            (1.0, [255, 0, 0]),
        ]);
        let mut bounds = lut.bounds();
        assert_eq!(
            bounds.next(),
            Some(((0.0, [0, 0, 255]), (0.0, [0, 0, 255])))
        );
        assert_eq!(
            bounds.next(),
            Some(((0.0, [0, 0, 255]), (0.5, [255, 255, 255])))
        );
        assert_eq!(
            bounds.next(),
            Some(((0.5, [255, 255, 255]), (1.0, [255, 0, 0])))
        );
        assert_eq!(
            bounds.next(),
            Some(((1.0, [255, 0, 0]), (1.0, [255, 0, 0])))
        );
        assert_eq!(bounds.next(), None);
    }

    #[test]
    fn test_color_bounds_with_limits() {
        let mut lut = ColorLUT::linear(vec![(0.0, [0, 0, 0]), (1.0, [255, 255, 255])]);
        lut.lims.0 = 0.2;
        lut.lims.2 = 0.9;
        assert_eq!(lut.color_at(0.0), [0, 0, 0]);
        assert_eq!(lut.color_at(0.1), [0, 0, 0]);
        assert_eq!(lut.color_at(0.2), [0, 0, 0]);
        assert_eq!(lut.color_at(0.55), [127, 127, 127]);
        assert_eq!(lut.color_at(0.9), [255, 255, 255]);
        assert_eq!(lut.color_at(0.95), [255, 255, 255]);
        assert_eq!(lut.color_at(1.0), [255, 255, 255]);
    }
}
