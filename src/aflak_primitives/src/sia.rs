use std::borrow::Cow;

use reqwest;
use vo::sia;
use vo::table::{Cell, VOTable};

use super::{IOErr, IOValue};
use download;

pub type SiaService = sia::SiaService<Cow<'static, String>>;

pub fn run_query(pos: [f32; 2]) -> Result<IOValue, IOErr> {
    let query = sia::SiaService::GAVO
        .map(Cow::Borrowed)
        .create_query((pos[0] as f64, pos[1] as f64))
        .with_format(sia::Format::Fits);
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
                    return if let Some(Cell::Character(link)) = row
                        .get_by_ucd("VOX:Image_AccessReference")
                        .or_else(|| row.get_by_id("access_url"))
                        .or_else(|| row.get_by_name("access_url"))
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
    /// TODO:
    ///  - Use single client for whole aflak
    println!("URL: {}", url);
    let client = reqwest::Client::new();
    let tmp_file_path = download::download(&client, url).map_err(|e| {
        IOErr::UnexpectedInput(format!("download_fits: Could not download file. {}", e))
    })?;

    super::run_open_fits(tmp_file_path)
}
