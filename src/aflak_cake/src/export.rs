use std::marker::PhantomData;

use boow::Bow;
use dst::{DSTError, Input, Output, OutputId, TransformIdx, DST};
use transform::{Algorithm, TransformId, Transformation};
use variant_name::VariantName;

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
            DeserTransform::Constant(constants) => Ok(Bow::Owned(Transformation {
                name: "const",
                input: vec![],
                output: constants.iter().map(|t| t.variant_name()).collect(),
                algorithm: Algorithm::Constant(constants),
            })),
            _ => panic!("PhantomData should not be used!"),
        }
    }
}

/// Vectors are more portable than hashmaps for serialization.
#[derive(Clone, Debug, Serialize)]
pub struct SerialDST<'d, T: 'd> {
    transforms: Vec<(&'d TransformIdx, SerialTransform<'d, T>)>,
    edges: Vec<(&'d Output, &'d Input)>,
    outputs: Vec<(&'d OutputId, &'d Option<Output>)>,
}

impl<'d, T> SerialDST<'d, T>
where
    T: 'd + Clone,
{
    pub fn new<E>(dst: &'d DST<T, E>) -> Self {
        Self {
            transforms: dst.transforms_iter()
                .map(|(t_idx, t)| (t_idx, SerialTransform::new(t)))
                .collect(),
            edges: dst.edges_iter().collect(),
            outputs: dst.outputs_iter().collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeserDST<T, E> {
    transforms: Vec<(TransformIdx, DeserTransform<T, E>)>,
    edges: Vec<(Output, Input)>,
    outputs: Vec<(OutputId, Option<Output>)>,
}

impl<T, E> DeserDST<T, E>
where
    T: Clone + NamedAlgorithms<E> + VariantName,
{
    pub fn into(self) -> Result<DST<'static, T, E>, ImportError<E>> {
        let mut dst = DST::new();
        for (t_idx, t) in self.transforms {
            let t = t.into()?;
            unsafe {
                dst.add_transform_with_idx(t_idx, t);
            }
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
            unsafe { dst.create_output_with_id(output_id) };
            if let Some(output) = some_output {
                dst.update_output(output_id, output);
            }
        }
        Ok(dst)
    }
}
