use boow::Bow;
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

#[derive(Copy, Clone, Debug, Serialize)]
pub enum SerialTransform<'t, T: 't> {
    Function(TransformId),
    Constant(&'t [T]),
}

use std::marker::PhantomData;

#[derive(Clone, Debug, Deserialize)]
pub enum DeserTransform<T, E> {
    Function(String),
    Constant(Vec<T>),
    Phantom(PhantomData<E>),
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
    pub fn into(self) -> Bow<'static, Transformation<T, E>> {
        match self {
            DeserTransform::Function(name) => {
                if let Some(t) = NamedAlgorithms::get_transform(&name) {
                    Bow::Borrowed(t)
                } else {
                    panic!("Transform '{}' not found!", name)
                }
            }
            DeserTransform::Constant(constants) => Bow::Owned(Transformation {
                name: "const",
                input: vec![],
                output: constants.iter().map(|t| t.variant_name()).collect(),
                algorithm: Algorithm::Constant(constants),
            }),
            _ => panic!("PhantomData should not be used!"),
        }
    }
}
