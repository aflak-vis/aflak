use std::fs;

use reqwest;
use vo::sia::SiaService;
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
    println!("URL: {}", url);
    let client = reqwest::Client::new();
    let tmp_file_name = "tmp.fits";
    match fs::File::create(tmp_file_name) {
        Ok(mut file) => {
            match client
                .get(url)
                .send()
                .and_then(|response| response.error_for_status())
            {
                Ok(mut response) => response
                    .copy_to(&mut file)
                    .map_err(|e| IOErr::UnexpectedInput(format!("{}", e))),
                Err(e) => Err(IOErr::UnexpectedInput(format!("{}", e))),
            }?;

            super::run_open_fits(tmp_file_name)
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
