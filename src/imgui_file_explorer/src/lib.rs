extern crate imgui;
use std::path::*;
use std::{fs, io};

use imgui::*;

pub trait UiFileExplorer {
    /// Can filter over several extensions.
    /// Anything that can be treated as a reference to a string `AsRef<str>` can be used as argument!
    /// Return the selected path, if any.
    fn file_explorer<T, S>(&self, target: T, extensions: &[S]) -> io::Result<Option<PathBuf>>
    where
        T: AsRef<str>,
        S: AsRef<str>;
}

/// Ui extends
impl<'ui> UiFileExplorer for Ui<'ui> {
    fn file_explorer<T, S>(&self, target: T, extensions: &[S]) -> io::Result<Option<PathBuf>>
    where
        T: AsRef<str>,
        S: AsRef<str>,
    {
        let ret = view_dirs(&self, target, extensions);
        fn view_dirs<'a, T: AsRef<str>, S: AsRef<str>>(
            ui: &Ui<'a>,
            target: T,
            extensions: &[S],
        ) -> io::Result<Option<PathBuf>> {
            let mut files: Vec<String> = Vec::new();
            for path in fs::read_dir(target.as_ref())? {
                let path = path?;
                files.push(
                    path.path()
                        .display()
                        .to_string()
                        .replacen(target.as_ref(), "", 1),
                );
            }
            files.sort();
            let mut ret = Ok(None);
            for i in files {
                let each_target = target.as_ref().to_string() + &i;
                if Path::new(&each_target).is_dir() && Path::new(&each_target).exists() {
                    ui.tree_node(im_str!("{}", i.replacen(MAIN_SEPARATOR, "", 1)))
                        .build(|| {
                            ret = view_dirs(ui, &each_target, extensions);
                        });
                } else {
                    let mut isprint = false;
                    for file_ext in extensions {
                        if Path::new(&each_target).extension() != None
                            && file_ext.as_ref().to_string().replacen(".", "", 1)
                                == Path::new(&each_target)
                                    .extension()
                                    .unwrap_or(std::ffi::OsStr::new(""))
                                    .to_str()
                                    .unwrap_or("")
                        {
                            isprint = true;
                            break;
                        }
                    }
                    if isprint == true {
                        ui.bullet_text(im_str!(""));
                        ui.same_line(0.0);
                        if ui.small_button(im_str!("Open {:?}", i.replacen(MAIN_SEPARATOR, "", 1)))
                        {
                            println!("Open \"{}\"", i.replacen(MAIN_SEPARATOR, "", 1));
                            ret = Ok(Some(PathBuf::from(each_target)));
                            /*some load events*/
                        }
                    }
                }
            }
            ret
        }

        ret
    }
}
