use clap::{App, Arg};

pub const TEMPLATES: &[&str] = &["waveform", "equivalent_width", "fits_cleaning"];

pub fn build_cli() -> App<'static, 'static> {
    App::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("fits")
                .short("f")
                .long("fits")
                .value_name("FITS")
                .help("Set a FITS file to load"),
        )
        .arg(
            Arg::with_name("template")
                .short("t")
                .long("template")
                .value_name("TEMPLATE NAME")
                .possible_values(TEMPLATES)
                .help("The name of the template to use"),
        )
        .arg(
            Arg::with_name("load")
                .short("l")
                .long("load")
                .value_name("FILE")
                .conflicts_with("template")
                .help("Import editor from .ron or aflak-exported .fits file"),
        )
}
