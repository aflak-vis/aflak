use transform::{NamedAlgorithms, Transformation, TypeContent};

use serde::ser::{Serialize, SerializeStruct, Serializer};

impl<'de, T: TypeContent> Serialize for Transformation<'de, T>
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

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use std::fmt;
use std::marker::PhantomData;

impl<'de, T> Deserialize<'de> for Transformation<'de, T>
where
    T::Type: Deserialize<'de>,
    T: 'static + TypeContent + NamedAlgorithms,
{
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

        impl<'de, T> Visitor<'de> for TransformationVisitor<T>
        where
            T::Type: Deserialize<'de>,
            T: 'static + TypeContent + NamedAlgorithms,
        {
            type Value = Transformation<'de, T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Transformation")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name = None;
                let mut input = None;
                let mut output = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                        Field::Input => {
                            if input.is_some() {
                                return Err(de::Error::duplicate_field("input"));
                            }
                            input = Some(map.next_value()?);
                        }
                        Field::Output => {
                            if output.is_some() {
                                return Err(de::Error::duplicate_field("output"));
                            }
                            output = Some(map.next_value()?);
                        }
                    }
                }
                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let input = input.ok_or_else(|| de::Error::missing_field("input"))?;
                let output = output.ok_or_else(|| de::Error::missing_field("output"))?;
                let algorithm = T::get_algorithm(name)
                    .ok_or_else(|| de::Error::custom("algorithm name not found"))?;
                Ok(Transformation {
                    name,
                    input,
                    output,
                    algorithm,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["name", "input", "output"];
        deserializer.deserialize_struct("Transformation", FIELDS, TransformationVisitor::new())
    }
}
