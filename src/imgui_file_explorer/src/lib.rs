extern crate aflak_imgui as imgui;
#[macro_use]
extern crate cfg_if;

use std::cmp::Ordering;
use std::path::*;
use std::{fs, io};

use imgui::*;

pub trait UiFileExplorer {
    /// Can filter over several extensions.
    /// Anything that can be treated as a reference to a string `AsRef<str>` can be used as argument!
    /// Return the selected path, if any.
    fn file_explorer<T, S>(&self, target: T, extensions: &[S]) -> io::Result<Option<PathBuf>>
    where
        T: AsRef<Path>,
        S: AsRef<str>;
}

fn has_extension<P: AsRef<Path>, S: AsRef<str>>(path: P, extensions: &[S]) -> bool {
    let path = path.as_ref();
    if let Some(test_ext) = path.extension() {
        if let Some(test_ext) = test_ext.to_str() {
            extensions
                .iter()
                .any(|ext| test_ext.to_lowercase() == ext.as_ref().to_lowercase())
        } else {
            false
        }
    } else {
        false
    }
}

cfg_if! {
    if #[cfg(unix)] {
        pub const TOP_FOLDER: &str = "/";
    } else {
        pub const TOP_FOLDER: &str = "C:";
    }
}

/// Ui extends
impl<'ui> UiFileExplorer for Ui<'ui> {
    fn file_explorer<T, S>(&self, target: T, extensions: &[S]) -> io::Result<Option<PathBuf>>
    where
        T: AsRef<Path>,
        S: AsRef<str>,
    {
        let ret = view_dirs(&self, target, extensions);

        fn view_dirs<'a, T: AsRef<Path>, S: AsRef<str>>(
            ui: &Ui<'a>,
            target: T,
            extensions: &[S],
        ) -> io::Result<Option<PathBuf>> {
            let target = target.as_ref();
            let mut files = Vec::new();

            for direntry in fs::read_dir(target)? {
                match direntry {
                    Ok(direntry) => {
                        let path = direntry.path();
                        if path.is_dir() || has_extension(&path, extensions) {
                            files.push(path);
                        }
                    }
                    Err(e) => eprintln!("Error on listing files: {:?}", e),
                }
            }

            // Sort folder first
            files.sort_by(|path1, path2| match (path1.is_dir(), path2.is_dir()) {
                (true, true) => path1.cmp(path2),
                (false, false) => path1.cmp(path2),
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
            });

            let mut ret = Ok(None);
            for i in files {
                if i.is_dir() {
                    if let Some(dirname) = i.file_name() {
                        if let Some(dirname) = dirname.to_str() {
                            let im_dirname = ImString::new(dirname);
                            ui.tree_node(&im_dirname).build(|| {
                                ret = view_dirs(ui, &i, extensions);
                            });
                        } else {
                            eprintln!("Could not get str out of directory: {:?}", i);
                        }
                    } else {
                        eprintln!("Could not get dirname for path: {:?}", i);
                    }
                } else {
                    if let Some(file_name) = i.file_name() {
                        if let Some(file_name) = file_name.to_str() {
                            ui.bullet_text(im_str!(""));
                            ui.same_line(0.0);
                            if ui.small_button(&ImString::new(file_name)) {
                                ret = Ok(Some(i.clone()));
                            }
                        } else {
                            eprintln!("Could not get str out of file: {:?}", i);
                        }
                    } else {
                        eprintln!("Could not get file_name for path: {:?}", i);
                    }
                }
            }
            ret
        }

        ret
    }
}
