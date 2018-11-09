use clap::{App, Arg};

pub const TEMPLATES: &[&str] = &["waveform", "equivalent_width"];

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
        ).arg(
            Arg::with_name("template")
                .short("t")
                .long("template")
                .value_name("TEMPLATE NAME")
                .possible_values(TEMPLATES)
                .help("The name of the template to use"),
        )
}
