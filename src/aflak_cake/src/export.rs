use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use boow::Bow;
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use variant_name::VariantName;

use dst::{DSTError, Input, Output, OutputId, TransformIdx, DST};
use transform::{Algorithm, TransformId, Transformation};

/// Trait that defines a function to get a [`Transformation`] by its name.
pub trait NamedAlgorithms<E>
where
    Self: Clone,
{
    /// Get a transform by name.
    fn get_transform(s: &str) -> Option<&'static Transformation<Self, E>>;
}

#[derive(Debug)]
pub enum ImportError<E> {
    TransformationNotFound(String),
    ConstructionError(&'static str, DSTError<E>),
    EmptyConstant,
}

impl<E> fmt::Display for ImportError<E>
where
    E: fmt::Debug,
{
    // TODO: Should make a better implementation of Display!
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub enum SerialTransform<'t, T: 't> {
    Function(TransformId),
    Constant(&'t [T]),
}

#[derive(Clone, Debug, Deserialize)]
pub enum DeserTransform<T, E> {
    Function(String),
    Constant(Vec<T>),
    Phantom(PhantomData<fn() -> E>),
}

impl<'t, T> SerialTransform<'t, T>
where
    T: 't + Clone,
{
    pub fn new<E>(t: &'t Transformation<T, E>) -> Self {
        match t.algorithm {
            Algorithm::Function(_) => SerialTransform::Function(t.name),
            Algorithm::Constant(ref c) => SerialTransform::Constant(c),
        }
    }
}

impl<T, E> DeserTransform<T, E>
where
    T: Clone + NamedAlgorithms<E> + VariantName,
{
    pub fn into(self) -> Result<Bow<'static, Transformation<T, E>>, ImportError<E>> {
        match self {
            DeserTransform::Function(name) => {
                if let Some(t) = NamedAlgorithms::get_transform(&name) {
                    Ok(Bow::Borrowed(t))
                } else {
                    Err(ImportError::TransformationNotFound(format!(
                        "Transform '{}' not found!",
                        name
                    )))
                }
            }
            DeserTransform::Constant(constants) => {
                let first_variant = constants
                    .get(0)
                    .map(|t| t.variant_name())
                    .ok_or(ImportError::EmptyConstant)?;
                Ok(Bow::Owned(Transformation {
                    name: first_variant,
                    description: Cow::Owned(format!(
                        "Constant variable of type '{}'",
                        first_variant
                    )),
                    input: vec![],
                    output: constants.iter().map(|t| t.variant_name()).collect(),
                    algorithm: Algorithm::Constant(constants),
                }))
            }
            _ => panic!("PhantomData should not be used!"),
        }
    }
}

/// Vectors are more portable than hashmaps for serialization.
#[derive(Clone, Debug, Serialize)]
pub struct SerialDST<'d, T: 'd> {
    transforms: Vec<(&'d TransformIdx, SerialMetaTransform<'d, T>)>,
    edges: Vec<(&'d Output, &'d Input)>,
    outputs: Vec<(&'d OutputId, &'d Option<Output>)>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SerialMetaTransform<'d, T: 'd> {
    t: SerialTransform<'d, T>,
    input_defaults: Vec<Option<T>>,
}

impl<'d, T> SerialDST<'d, T>
where
    T: 'd + Clone,
{
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

#[derive(Clone, Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct DeserDST<T, E> {
    transforms: Vec<(TransformIdx, DeserMetaTransform<T, E>)>,
    edges: Vec<(Output, Input)>,
    outputs: Vec<(OutputId, Option<Output>)>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct DeserMetaTransform<T, E> {
    t: DeserTransform<T, E>,
    input_defaults: Vec<Option<T>>,
}

impl<T, E> DeserDST<T, E>
where
    T: Clone + NamedAlgorithms<E> + VariantName,
{
    pub fn into(self) -> Result<DST<'static, T, E>, ImportError<E>> {
        let mut dst = DST::new();
        for (t_idx, DeserMetaTransform { t, input_defaults }) in self.transforms {
            let t = t.into()?;
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
    T: 't + Clone + Serialize,
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
    T: 't + Clone + Deserialize<'de> + NamedAlgorithms<E> + VariantName,
    E: fmt::Debug,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        DeserDST::deserialize(deserializer)
            .and_then(|deser_dst| deser_dst.into().map_err(de::Error::custom))
    }
}
