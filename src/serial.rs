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

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use std::fmt;
use std::marker::PhantomData;

impl<'de, T: TypeContent> Deserialize<'de> for Transformation<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Name,
            Input,
            Output,
        };

        struct TransformationVisitor<T> {
            marker: PhantomData<fn() -> T>,
        };
        impl<T> TransformationVisitor<T> {
            fn new() -> Self {
                TransformationVisitor {
                    marker: PhantomData,
                }
            }
        }

        impl<'de, T: TypeContent> Visitor<'de> for TransformationVisitor<T> {
            type Value = Transformation<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Transformation")
            }
        }

        const FIELDS: &'static [&'static str] = &["name", "input", "output"];
        deserializer.deserialize_struct("Transformation", FIELDS, TransformationVisitor::new())
    }
}
