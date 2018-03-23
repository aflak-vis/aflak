use transform::{NamedAlgorithms, Transformation};

use serde::ser::{Serialize, SerializeStruct, Serializer};

impl<T, E> Serialize for Transformation<T, E>
where
    T: Clone,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Transformation", 3)?;
        state.serialize_field("name", &self.name)?;
        state.end()
    }
}

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use std::fmt;
use std::marker::PhantomData;

impl<'de, T, E> Deserialize<'de> for Transformation<T, E>
where
    T: 'static + NamedAlgorithms<E>,
    E: 'static + Clone,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Name,
        };

        struct TransformationVisitor<T, E> {
            marker: PhantomData<fn() -> (T, E)>,
        };
        impl<T, E> TransformationVisitor<T, E> {
            fn new() -> Self {
                TransformationVisitor {
                    marker: PhantomData,
                }
            }
        }

        impl<'de, T, E> Visitor<'de> for TransformationVisitor<T, E>
        where
            T: 'static + NamedAlgorithms<E>,
            E: 'static + Clone,
        {
            type Value = Transformation<T, E>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Transformation")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                    }
                }
                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let transform = T::get_transform(name)
                    .ok_or_else(|| de::Error::custom("algorithm name not found"))?;
                Ok(transform.clone())
            }
        }

        const FIELDS: &'static [&'static str] = &["name"];
        deserializer.deserialize_struct("Transformation", FIELDS, TransformationVisitor::new())
    }
}
