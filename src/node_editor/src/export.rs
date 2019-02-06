use std::collections::BTreeMap;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use cake::{self, Cache, DeserDST, NamedAlgorithms, NodeId, SerialDST, VariantName};
use ron::{de, ser};
use serde::{Deserialize, Serialize};

use editor::NodeEditor;
use node_state::{NodeState, NodeStates};
use scrolling::Scrolling;
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
    fn new<E>(editor: &'e NodeEditor<T, E>) -> Self {
        Self {
            dst: SerialDST::new(&editor.dst),
            node_states: editor.node_states.iter().collect(),
            scrolling: editor.scrolling.get_current(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct DeserEditor<T, E> {
    dst: DeserDST<T, E>,
    node_states: Vec<(NodeId, NodeState)>,
    scrolling: Vec2,
}

impl<T, E> NodeEditor<T, E>
where
    T: Clone + VariantName,
{
    pub fn export(&self) -> SerialEditor<T> {
        SerialEditor::new(self)
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

impl<T, E> NodeEditor<T, E>
where
    T: Clone + Serialize + VariantName,
{
    /// Serialize node editor to writer as .ron format.
    pub fn export_to_buf<W: io::Write>(&self, w: &mut W) -> Result<(), ExportError> {
        let serializable = self.export();
        let serialized = ser::to_string_pretty(&serializable, Default::default())?;
        w.write_all(serialized.as_bytes())?;
        w.flush()?;
        Ok(())
    }

    /// Serialize node editor to .ron file.
    pub fn export_to_file<P: AsRef<Path>>(&self, file_path: P) -> Result<(), ExportError> {
        let mut f = fs::File::create(file_path)?;
        self.export_to_buf(&mut f)
    }
}

impl<'t, T, E> NodeEditor<T, E>
where
    T: 'static + Clone + NamedAlgorithms<E> + VariantName + cake::ConvertibleVariants,
    E: 'static,
{
    pub fn import(&mut self, import: DeserEditor<T, E>) -> Result<(), cake::ImportError> {
        self.dst = import.dst.into_dst()?;

        // Set Ui node states
        self.node_states = {
            let mut node_states = NodeStates::new();
            for (node_id, state) in import.node_states {
                node_states.insert(node_id, state);
            }
            node_states
        };

        // Reset all temporary values
        self.active_node = None;
        self.drag_node = None;
        self.creating_link = None;
        self.new_link = None;

        // Set scrolling offset
        self.scrolling = Scrolling::new(import.scrolling);

        // Reset cache
        self.output_results = BTreeMap::new();
        self.cache = Cache::new();
        Ok(())
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

impl<T, E> NodeEditor<T, E>
where
    T: 'static
        + Clone
        + NamedAlgorithms<E>
        + VariantName
        + cake::ConvertibleVariants
        + for<'de> Deserialize<'de>,
    E: 'static,
{
    /// Deserialize a buffer in .ron format and make a node editor.
    pub fn from_export_buf<R>(r: R) -> Result<Self, ImportError>
    where
        R: io::Read,
    {
        let mut editor = Self::default();
        editor.import_from_buf(r)?;
        Ok(editor)
    }

    /// Deserialize a .ron file and make a node editor.
    pub fn from_ron_file<P>(file_path: P) -> Result<Self, ImportError>
    where
        P: AsRef<Path>,
    {
        let f = fs::File::open(file_path)?;
        Self::from_export_buf(f)
    }

    /// Replace the node editor with the content of the buffer in .ron format.
    pub fn import_from_buf<R: io::Read>(&mut self, r: R) -> Result<(), ImportError> {
        let deserialized = de::from_reader(r)?;
        self.import(deserialized)?;
        Ok(())
    }

    /// Replace the node editor with the content of the .ron file.
    pub fn import_from_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<(), ImportError> {
        let f = fs::File::open(file_path)?;
        self.import_from_buf(f)
    }
}
