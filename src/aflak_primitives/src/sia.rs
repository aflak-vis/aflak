use std::fs;
use std::io::Write;

use hyper;
use hyper::rt::{Future, Stream};
use tokio;
use vo::sia::{self, SiaService};
use vo::table::{Cell, VOTable};

use super::{IOErr, IOValue};

pub fn run_query(pos: [f32; 2]) -> Result<IOValue, IOErr> {
    let query = SiaService::UNI_HEIDELBERG.create_query((pos[0] as f64, pos[1] as f64));
    match query.execute_sync() {
        Ok(results) => Ok(IOValue::VOTable(results.into_table())),
        Err(e) => Err(IOErr::SIAError(e)),
    }
}

pub fn run_acref_from_record(votable: &VOTable, index: i64) -> Result<IOValue, IOErr> {
    if index < 0 {
        return Err(IOErr::UnexpectedInput(format!(
            "acref_from_record: index must be positive, got {}",
            index
        )));
    }

    let index = index as usize;

    let mut i = 0;
    for table in votable.tables() {
        if let Some(rows) = table.rows() {
            for row in rows {
                if i == index {
                    return if let Some(Cell::Character(link)) =
                        row.get_by_ucd("VOX:Image_AccessReference")
                    {
                        Ok(IOValue::Str(link.to_owned()))
                    } else {
                        Err(IOErr::UnexpectedInput(format!(
                            "acref_from_record: Record #{} has no acref (link to access FITS file) defined.",
                            index
                        )))
                    };
                }
                i += 1;
            }
        }
    }
    Err(IOErr::UnexpectedInput(format!(
        "acref_from_record: Could not find record #{} in VOTable",
        index
    )))
}

pub fn run_download_fits(url: &str) -> Result<IOValue, IOErr> {
    let client = hyper::Client::new();
    match url.parse() {
        Ok(uri) => {
            let tmp_file_name = "tmp.fits";
            match fs::File::create(tmp_file_name) {
                Ok(mut file) => {
                    let promise = client
                        .get(uri)
                        .map_err(|e| IOErr::UnexpectedInput(format!("{}", e)))
                        .map(|res| {
                            println!("{:?}", res);
                            println!("SUCCESS: {}", res.status().is_success());
                            res.into_body()
                        }).and_then(move |body| {
                            body.map_err(|e| IOErr::UnexpectedInput(format!("{}", e)))
                                .for_each(move |chunk| {
                                    file.write_all(&chunk).map_err(|e| {
                                        IOErr::IoError(
                                            e,
                                            format!(
                                                "download_fits: Could not write to file '{}'.",
                                                tmp_file_name
                                            ),
                                        )
                                    })
                                })
                        });
                    use super::run_open_fits;

                    let mut runtime = tokio::runtime::Runtime::new().map_err(|e| {
                        IOErr::SIAError(sia::Error::RuntimeError(
                            e,
                            "Could not initialize a Tokio runtime.",
                        ))
                    })?;
                    runtime
                        .block_on(promise)
                        .and_then(|_| run_open_fits(tmp_file_name))
                }
                Err(e) => Err(IOErr::IoError(
                    e,
                    format!(
                        "download_fits: Could not create temporary file '{}'.",
                        tmp_file_name
                    ),
                )),
            }
        }
        Err(e) => Err(IOErr::UnexpectedInput(format!(
            "download_fits: Could not parse privded URL '{}'. Error: {}",
            url, e
        ))),
    }
}
