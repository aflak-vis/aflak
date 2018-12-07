use std::collections::hash_map::DefaultHasher;
use std::error;
use std::fmt;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

use reqwest;

const DOWNLOAD_DIR: &str = ".aflak-downloads";

/// Download file (with caching) and return the file path.
pub fn download(client: &reqwest::Client, url: &str) -> Result<PathBuf, Error> {
    let download_path = Path::new(DOWNLOAD_DIR);
    if !download_path.exists() {
        if let Err(e) = fs::create_dir(&download_path) {
            return Err(Error::IoError {
                msg: format!("Could not create directory '{:?}'", download_path),
                e,
            });
        }
    }

    if !download_path.is_dir() {
        return Err(Error::DownloadPathNotDir(download_path));
    }

    // Hash URL to get a unique filename for use in cache
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();
    let file_name = format!("{:x}", hash);
    let file_path = download_path.join(&file_name);

    if !file_path.exists() {
        // Download file from provided URL
        let download_file_name = format!("{}.download", &file_name);
        let download_file_path = download_path.join(download_file_name);

        if download_file_path.exists() {
            // File is currently being downloaded. Wait for download to complete.
            // TODO: Add some sort of timeout.
            while !file_path.exists() {
                let duration = ::std::time::Duration::from_millis(500);
                ::std::thread::sleep(duration);
            }
            return Ok(file_path);
        }
        match fs::File::create(&download_file_path) {
            Ok(mut file) => {
                match client
                    .get(url)
                    .send()
                    .and_then(|res| res.error_for_status())
                {
                    Ok(mut res) => {
                        if let Err(e) = res.copy_to(&mut file) {
                            return Err(Error::Reqwest {
                                msg: format!(
                                    "Could not write response's body to file {:?}",
                                    download_file_path
                                ),
                                e,
                            });
                        }
                    }
                    Err(e) => {
                        return Err(Error::Reqwest {
                            msg: format!("Error on getting '{}'", url),
                            e,
                        });
                    }
                }
            }
            Err(e) => {
                return Err(Error::IoError {
                    msg: format!(
                        "Could not create temporary file in {:?}.",
                        download_file_path
                    ),
                    e,
                })
            }
        }
        // TODO: Delete download file on errors... (Make type and implement Drop)
        if let Err(e) = fs::rename(&download_file_path, &file_path) {
            return Err(Error::IoError {
                msg: format!(
                    "Could not move downloaded in from {:?} to {:?}.",
                    download_file_path, file_path
                ),
                e,
            });
        }
    }

    Ok(file_path)
}

#[derive(Debug)]
pub enum Error {
    IoError { msg: String, e: io::Error },
    Reqwest { msg: String, e: reqwest::Error },
    DownloadPathNotDir(&'static Path),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError { msg, e } => write!(f, "I/O Error. {}. {}.", msg, e),
            Error::Reqwest { msg, e } => write!(f, "Error on handling request. {}. {}.", msg, e),
            Error::DownloadPathNotDir(path) => {
                write!(f, "Download path {:?} is nor a directory.", path)
            }
        }
    }
}

impl error::Error for Error {}
