use std::error;
use std::fmt;
use std::marker::PhantomData;

use boow::Bow;
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use variant_name::VariantName;

use super::ConvertibleVariants;
use dst::{DSTError, Input, Output, OutputId, TransformIdx, DST};
use transform::{Algorithm, Transform};

/// Trait that defines a function to get a [`Transform`] by its name.
pub trait NamedAlgorithms<E>: Sized {
    /// Get a transform by name.
    fn get_transform(s: &str) -> Option<&'static Transform<Self, E>>;
}

/// Error type used to represent a failed deserialization into DST.
#[derive(Debug)]
pub enum ImportError<E> {
    /// An unknown transform name was used.
    TransformNotFound(String),
    /// The DST cannot be constructed because it is inconsistent.
    ConstructionError(&'static str, DSTError<E>),
}

impl<E> fmt::Display for ImportError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ImportError::TransformNotFound(ref s) => write!(f, "Transform not found! {}", s),
            ImportError::ConstructionError(s, ref e) => {
                write!(f, "Construction error! {} Caused by {}", s, e)
            }
        }
    }
}

impl<E: fmt::Display + fmt::Debug> error::Error for ImportError<E> {
    fn description(&self) -> &'static str {
        "ImportError"
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, Serialize)]
pub enum SerialTransform<'t, T: 't> {
    Function(&'static str),
    Constant(&'t T),
}

#[doc(hidden)]
#[derive(Clone, Debug, Deserialize)]
pub enum DeserTransform<T, E> {
    Function(String),
    Constant(T),
    Phantom(PhantomData<fn() -> E>),
}

impl<'t, T> SerialTransform<'t, T>
where
    T: 't + VariantName,
{
    pub fn new<E>(t: &'t Transform<T, E>) -> Self {
        match t.algorithm() {
            Algorithm::Function { id, .. } => SerialTransform::Function(id.name()),
            Algorithm::Constant(ref c) => SerialTransform::Constant(c),
        }
    }
}

impl<T, E> DeserTransform<T, E>
where
    T: NamedAlgorithms<E>,
{
    pub fn into_transform(self) -> Result<Bow<'static, Transform<T, E>>, ImportError<E>> {
        match self {
            DeserTransform::Function(name) => {
                if let Some(t) = NamedAlgorithms::get_transform(&name) {
                    Ok(Bow::Borrowed(t))
                } else {
                    Err(ImportError::TransformNotFound(format!(
                        "Transform '{}' not found!",
                        name
                    )))
                }
            }
            DeserTransform::Constant(c) => Ok(Bow::Owned(Transform::new_constant(c))),
            _ => panic!("PhantomData should not be used!"),
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
                }).collect(),
            edges: dst.edges_iter().collect(),
            outputs: dst.outputs_iter().collect(),
        }
    }
}

/// A representation of a DST for deserialization.
#[derive(Clone, Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct DeserDST<T, E> {
    transforms: Vec<(TransformIdx, DeserMetaTransform<T, E>)>,
    edges: Vec<(Output, Input)>,
    outputs: Vec<(OutputId, Option<Output>)>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct DeserMetaTransform<T, E> {
    t: DeserTransform<T, E>,
    input_defaults: Vec<Option<T>>,
}

impl<T, E> DeserDST<T, E>
where
    T: NamedAlgorithms<E> + VariantName + ConvertibleVariants,
{
    /// Converts this intermediary representation of a DST into a normal DST.
    pub fn into_dst(self) -> Result<DST<'static, T, E>, ImportError<E>> {
        let mut dst = DST::new();
        for (t_idx, DeserMetaTransform { t, input_defaults }) in self.transforms {
            let t = t.into_transform()?;
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
        DeserDST::deserialize(deserializer)
            .and_then(|deser_dst| deser_dst.into_dst().map_err(de::Error::custom))
    }
}
