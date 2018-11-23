use std::collections::BTreeMap;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use cake::{self, Cache, DeserDST, NamedAlgorithms, NodeId, SerialDST, Transform, VariantName};
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
    fn new<E, ED>(editor: &'e NodeEditor<T, E, ED>) -> Self {
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

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
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

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
where
    T: Clone + Serialize + VariantName,
{
    pub fn export_to_buf<W: io::Write>(&self, w: &mut W) -> Result<(), ExportError> {
        let serializable = self.export();
        let serialized = ser::to_string_pretty(&serializable, Default::default())?;
        w.write_all(serialized.as_bytes())?;
        w.flush()?;
        Ok(())
    }

    pub fn export_to_file<P: AsRef<Path>>(&self, file_path: P) -> Result<(), ExportError> {
        let mut f = fs::File::create(file_path)?;
        self.export_to_buf(&mut f)
    }
}

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
where
    T: 'static + Clone + NamedAlgorithms<E> + VariantName,
    E: 'static,
{
    pub fn import(&mut self, import: DeserEditor<T, E>) -> Result<(), cake::ImportError<E>> {
        self.dst = import.dst.into_dst()?;

        // Set Ui node states
        self.node_states = {
            let mut node_states = NodeStates::new();
            for (node_id, state) in import.node_states {
                node_states.insert(node_id, state);
            }
            node_states
        };
        // Set scrolling offset
        self.scrolling = Scrolling::new(import.scrolling);

        // Reset cache
        self.output_results = BTreeMap::new();
        self.cache = Cache::new();
        Ok(())
    }
}

#[derive(Debug)]
pub enum ImportError<E> {
    DSTError(cake::ImportError<E>),
    DeserializationError(de::Error),
    IOError(io::Error),
}

impl<E: fmt::Display> fmt::Display for ImportError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ImportError::DSTError(ref e) => write!(f, "Error while building DST! {}", e),
            ImportError::DeserializationError(ref e) => write!(f, "Deserialization error! {}", e),
            ImportError::IOError(ref e) => write!(f, "I/O error! {}", e),
        }
    }
}

impl<E: fmt::Display + fmt::Debug> error::Error for ImportError<E> {
    fn description(&self) -> &'static str {
        "ImportError"
    }
}

impl<E> From<io::Error> for ImportError<E> {
    fn from(io_error: io::Error) -> Self {
        ImportError::IOError(io_error)
    }
}

impl<E> From<de::Error> for ImportError<E> {
    fn from(deserial_error: de::Error) -> Self {
        ImportError::DeserializationError(deserial_error)
    }
}

impl<E> From<cake::ImportError<E>> for ImportError<E> {
    fn from(e: cake::ImportError<E>) -> Self {
        ImportError::DSTError(e)
    }
}

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
where
    T: 'static + Clone + NamedAlgorithms<E> + VariantName + for<'de> Deserialize<'de>,
    E: 'static,
    ED: Default,
{
    pub fn from_export_buf<R>(
        r: R,
        addable_nodes: &'t [&'t Transform<T, E>],
        ed: ED,
    ) -> Result<Self, ImportError<E>>
    where
        R: io::Read,
    {
        let mut editor = Self::new(addable_nodes, ed);
        editor.import_from_buf(r)?;
        Ok(editor)
    }

    pub fn from_ron_file<P>(
        file_path: P,
        addable_nodes: &'t [&'t Transform<T, E>],
        ed: ED,
    ) -> Result<Self, ImportError<E>>
    where
        P: AsRef<Path>,
    {
        let f = fs::File::open(file_path)?;
        Self::from_export_buf(f, addable_nodes, ed)
    }

    pub fn import_from_buf<R: io::Read>(&mut self, r: R) -> Result<(), ImportError<E>> {
        let deserialized = de::from_reader(r)?;
        self.import(deserialized)?;
        Ok(())
    }

    pub fn import_from_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<(), ImportError<E>> {
        let f = fs::File::open(file_path)?;
        self.import_from_buf(f)
    }
}
