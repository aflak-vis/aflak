//! Structures for serialization and deserialization of node graph.
use std::borrow::Cow;
use std::error;
use std::fmt;

use boow::Bow;
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use variant_name::VariantName;

use super::ConvertibleVariants;
use dst::{DSTError, Input, Output, OutputId, TransformIdx, DST};
use macros::MacroManager;
use transform::{Algorithm, Transform, Version};

/// Trait that defines a function to get a [`Transform`] by its name.
pub trait NamedAlgorithms<E>: Sized {
    /// Get a transform by name.
    fn get_transform(s: &str) -> Option<&'static Transform<'static, Self, E>>;
}

/// Error type used to represent a failed deserialization into DST.
#[derive(Debug)]
pub enum ImportError {
    /// An unknown transform name was used.
    TransformNotFound(String),
    /// Default input type does not conform to expectation.
    UnexpectedDefaultInputType {
        expected: &'static str,
        got: &'static str,
        transform_name: Cow<'static, str>,
        transform_idx: TransformIdx,
    },
    /// The DST cannot be constructed because it is inconsistent.
    ConstructionError(&'static str, DSTError),
    /// Macro with the given ID not found.
    MacroNotFound(usize),
    /// Type found does not exist
    UnexpectedType(String),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ImportError::TransformNotFound(ref s) => write!(f, "Transform not found! {}", s),
            ImportError::UnexpectedDefaultInputType {
                expected,
                got,
                ref transform_name,
                transform_idx,
            } => {
                write!(
                f,
                "Unexpected default input type! Expected '{}' but got '{}' in transform '{}' (#{})",
                expected, got, transform_name, transform_idx.id(),
            )
            }
            ImportError::ConstructionError(s, ref e) => {
                write!(f, "Construction error! {} Caused by {}", s, e)
            }
            ImportError::MacroNotFound(id) => write!(f, "Macro with id {} not found", id),
            ImportError::UnexpectedType(ref type_id) => {
                write!(f, "Type '{}' does not exist", type_id)
            }
        }
    }
}

impl error::Error for ImportError {
    fn description(&self) -> &'static str {
        "ImportError"
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, Serialize)]
pub enum SerialTransform<'t, T: 't> {
    Function(&'static str, u8, u8, u8),
    Constant(&'t T),
    Macro(usize),
}

#[doc(hidden)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeserTransform<T> {
    Function(String, u8, u8, u8),
    Constant(T),
    Macro(usize),
}

impl<'t, T> SerialTransform<'t, T>
where
    T: 't + VariantName,
{
    pub fn new<E>(t: &'t Transform<T, E>) -> Self {
        match t.algorithm() {
            Algorithm::Function {
                id,
                version:
                    Version {
                        major,
                        minor,
                        patch,
                    },
                ..
            } => SerialTransform::Function(id.name(), *major, *minor, *patch),
            Algorithm::Constant(ref c) => SerialTransform::Constant(c),
            Algorithm::Macro { ref handle } => SerialTransform::Macro(handle.id()),
        }
    }
}

impl<T> DeserTransform<T> {
    pub fn from_transform<'t, E>(t: &Transform<'t, T, E>) -> Self
    where
        T: Clone,
    {
        match t.algorithm() {
            Algorithm::Function {
                id,
                version:
                    Version {
                        major,
                        minor,
                        patch,
                    },
                ..
            } => DeserTransform::Function(id.name().to_owned(), *major, *minor, *patch),
            Algorithm::Constant(ref c) => DeserTransform::Constant(c.clone()),
            Algorithm::Macro { ref handle } => DeserTransform::Macro(handle.id()),
        }
    }

    pub fn into_transform<E>(
        self,
        macro_manager: &MacroManager<'static, T, E>,
    ) -> Result<Bow<'static, Transform<'static, T, E>>, ImportError>
    where
        T: NamedAlgorithms<E>,
    {
        match self {
            DeserTransform::Function(name, major, _, _) => {
                if let Some(t) = NamedAlgorithms::get_transform(&name) {
                    if let Algorithm::Function { version: v, .. } = t.algorithm() {
                        if v.major != major {
                            eprintln!(
                                "Current built-in transform '{}' is of major version {}, but the program was written with the major version {}",
                                name, v.major, major
                            );
                        }
                    }
                    Ok(Bow::Borrowed(t))
                } else {
                    Err(ImportError::TransformNotFound(format!(
                        "Transform '{}' not found!",
                        name
                    )))
                }
            }
            DeserTransform::Constant(c) => Ok(Bow::Owned(Transform::new_constant(c))),
            DeserTransform::Macro(id) => macro_manager
                .get_macro(id)
                .map(|handle| Bow::Owned(Transform::from_macro(handle.clone())))
                .ok_or_else(|| ImportError::MacroNotFound(id)),
        }
    }
}

/// A representation of a DST for serialization.
///
/// Vectors are more portable than hashmaps for serialization.
#[derive(Clone, Debug, Serialize)]
pub struct SerialDST<'d, T: 'd> {
    transforms: Vec<(&'d TransformIdx, SerialMetaTransform<'d, T>)>,
    edges: Vec<(&'d Output, &'d Input)>,
    outputs: Vec<(&'d OutputId, &'d Option<Output>)>,
}

#[derive(Clone, Debug, Serialize)]
struct SerialMetaTransform<'d, T: 'd> {
    t: SerialTransform<'d, T>,
    input_defaults: Vec<Option<T>>,
}

impl<'d, T> SerialDST<'d, T>
where
    T: 'd + Clone + VariantName,
{
    /// Create a serializable representaion of the DST given as argument.
    pub fn new<E>(dst: &'d DST<T, E>) -> Self {
        Self {
            transforms: dst
                .meta_transforms_iter()
                .map(|(t_idx, meta)| {
                    (
                        t_idx,
                        SerialMetaTransform {
                            t: SerialTransform::new(meta.transform()),
                            input_defaults: meta.defaults().to_vec(),
                        },
                    )
                })
                .collect(),
            edges: dst.edges_iter().collect(),
            outputs: dst.outputs_iter().collect(),
        }
    }
}

/// A representation of a DST for deserialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct DeserDST<T> {
    transforms: Vec<(TransformIdx, DeserMetaTransform<T>)>,
    edges: Vec<(Output, Input)>,
    outputs: Vec<(OutputId, Option<Output>)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct DeserMetaTransform<T> {
    t: DeserTransform<T>,
    input_defaults: Vec<Option<T>>,
}

impl<T> DeserDST<T> {
    pub fn from_dst<'t, E>(dst: &DST<'t, T, E>) -> Self
    where
        T: Clone,
    {
        Self {
            transforms: dst
                .meta_transforms_iter()
                .map(|(t_idx, meta)| {
                    (
                        *t_idx,
                        DeserMetaTransform {
                            t: DeserTransform::from_transform(meta.transform()),
                            input_defaults: meta.defaults().to_vec(),
                        },
                    )
                })
                .collect(),
            edges: dst
                .edges_iter()
                .map(|(output, input)| (*output, *input))
                .collect(),
            outputs: dst
                .outputs_iter()
                .map(|(ouput_id, some_output)| (*ouput_id, *some_output))
                .collect(),
        }
    }
}

impl<T> DeserDST<T>
where
    T: Clone + VariantName + ConvertibleVariants,
{
    /// Converts this intermediary representation of a DST into a normal DST.
    pub fn into_dst<E>(
        self,
        macro_manager: &MacroManager<'static, T, E>,
    ) -> Result<DST<'static, T, E>, ImportError>
    where
        T: NamedAlgorithms<E>,
    {
        let mut dst = DST::new();
        for (t_idx, meta) in self.transforms {
            let t = meta.t.into_transform(macro_manager)?;
            let orig_defaults = t.defaults();
            let mut input_defaults = Vec::with_capacity(orig_defaults.len());
            let mut orig_defaults_iter = orig_defaults.into_iter();
            // Add received default values.
            // Complete with original default values if some are missing.
            for (got_default, orig_default) in
                meta.input_defaults.into_iter().zip(&mut orig_defaults_iter)
            {
                if got_default.is_none() {
                    input_defaults.push(orig_default);
                } else {
                    input_defaults.push(got_default);
                }
            }
            for orig_default in orig_defaults_iter {
                input_defaults.push(orig_default);
            }

            // Type check
            for (input_default, expected_type_id) in input_defaults.iter().zip(t.input_types()) {
                if let Some(input_default) = input_default {
                    if input_default.variant_name() != expected_type_id.name() {
                        return Err(ImportError::UnexpectedDefaultInputType {
                            expected: expected_type_id.name(),
                            got: input_default.variant_name(),
                            transform_name: t.name(),
                            transform_idx: t_idx,
                        });
                    }
                }
            }

            dst.add_transform_with_idx(t_idx, t, input_defaults);
        }
        for (output, input) in self.edges {
            dst.connect(output, input).map_err(|err| {
                ImportError::ConstructionError(
                    "Data is inconsistent. DST cannot be constructed.",
                    err,
                )
            })?;
        }
        for (output_id, some_output) in self.outputs {
            dst.create_output_with_id(output_id);
            if let Some(output) = some_output {
                dst.update_output(output_id, output);
            }
        }
        Ok(dst)
    }
}

impl<'t, T, E> Serialize for DST<'t, T, E>
where
    T: 't + Clone + Serialize + VariantName,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SerialDST::new(self).serialize(serializer)
    }
}

impl<'de, 't, T, E> Deserialize<'de> for DST<'static, T, E>
where
    T: 't + Clone + Deserialize<'de> + NamedAlgorithms<E> + VariantName + ConvertibleVariants,
    E: fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let empty_manager = MacroManager::new();
        DeserDST::deserialize(deserializer).and_then(|deser_dst| {
            deser_dst
                .into_dst(&empty_manager)
                .map_err(de::Error::custom)
        })
    }
}
