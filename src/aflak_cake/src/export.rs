use transform::{Algorithm, TransformId, Transformation};

/// Trait that defines a function to get a [`Transformation`] by its name.
pub trait NamedAlgorithms<E>
where
    Self: Clone,
{
    /// Get a transform by name.
    fn get_transform(s: &str) -> Option<&'static Transformation<Self, E>>;
}

#[derive(Serialize)]
pub enum SerialTransform<'t, T: 't> {
    Function(TransformId),
    Constant(&'t [T]),
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
