use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use cake::{self, NodeId, SerialDST, VariantName};
use ron::{de, ser};
use serde::Serialize;

use layout::NodeEditorLayout;
use node_state::NodeState;
use vec2::Vec2;

#[derive(Serialize)]
pub struct SerialEditor<'e, T: 'e> {
    dst: SerialDST<'e, T>,
    node_states: Vec<(&'e NodeId, &'e NodeState)>,
    scrolling: Vec2,
}

impl<'e, T> SerialEditor<'e, T>
where
    T: Clone + VariantName,
{
    fn new<E>(editor: &'e NodeEditorLayout<T, E>) -> Self {
        Self {
            dst: SerialDST::new(&editor.dst),
            node_states: editor.node_states.iter().collect(),
            scrolling: editor.scrolling.get_current(),
        }
    }
}

#[derive(Debug)]
pub enum ExportError {
    SerializationError(ser::Error),
    IOError(io::Error),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExportError::SerializationError(ref e) => write!(f, "Serialization error! {}", e),
            ExportError::IOError(ref e) => write!(f, "I/O error! {}", e),
        }
    }
}

impl error::Error for ExportError {
    fn description(&self) -> &'static str {
        "ExportError"
    }
}

impl From<io::Error> for ExportError {
    fn from(io_error: io::Error) -> Self {
        ExportError::IOError(io_error)
    }
}

impl From<ser::Error> for ExportError {
    fn from(serial_error: ser::Error) -> Self {
        ExportError::SerializationError(serial_error)
    }
}

impl<T, E> NodeEditorLayout<T, E>
where
    T: Clone + Serialize + VariantName,
{
    /// Serialize node editor to writer as .ron format.
    fn export_to_buf<W: io::Write>(&self, w: &mut W) -> Result<(), ExportError> {
        let serializable = SerialEditor::new(self);
        let serialized = ser::to_string_pretty(&serializable, Default::default())?;
        w.write_all(serialized.as_bytes())?;
        w.flush()?;
        Ok(())
    }

    /// Serialize node editor to .ron file.
    pub(crate) fn export_to_file<P: AsRef<Path>>(&self, file_path: P) -> Result<(), ExportError> {
        let mut f = fs::File::create(file_path)?;
        self.export_to_buf(&mut f)
    }
}

#[derive(Debug)]
pub enum ImportError {
    DSTError(cake::ImportError),
    DeserializationError(de::Error),
    IOError(io::Error),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ImportError::DSTError(ref e) => write!(f, "Error while building DST! {}", e),
            ImportError::DeserializationError(ref e) => write!(f, "Deserialization error! {}", e),
            ImportError::IOError(ref e) => write!(f, "I/O error! {}", e),
        }
    }
}

impl error::Error for ImportError {
    fn description(&self) -> &'static str {
        "ImportError"
    }
}

impl From<io::Error> for ImportError {
    fn from(io_error: io::Error) -> Self {
        ImportError::IOError(io_error)
    }
}

impl From<de::Error> for ImportError {
    fn from(deserial_error: de::Error) -> Self {
        ImportError::DeserializationError(deserial_error)
    }
}

impl From<cake::ImportError> for ImportError {
    fn from(e: cake::ImportError) -> Self {
        ImportError::DSTError(e)
    }
}
