use clap::{App, Arg};

use templates;

pub fn build_cli() -> App<'static, 'static> {
    App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
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
                .possible_values(templates::TEMPLATES)
                .help("The name of the template to use"),
        )
}
