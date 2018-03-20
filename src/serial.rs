use transform::{Transformation, TypeContent};
use serde::ser::{Serialize, SerializeStruct, Serializer};

impl<T: TypeContent> Serialize for Transformation<T>
where
    T::Type: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Transformation", 3)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("input", &self.input)?;
        state.serialize_field("output", &self.output)?;
        state.end()
    }
}
