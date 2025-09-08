use std::{fmt, sync::Arc};

use fxhash::FxHashMap;
use serde::{
    de::{Error, SeqAccess, Visitor},
    ser::{SerializeSeq},
    Deserialize, Deserializer, Serialize, Serializer,
};

pub fn serialize_arc_str<S>(value: &Arc<str>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Aprovecha que Arc<str> ya es un slice de string inmutable
    serializer.serialize_str(value.as_ref())
}

pub fn serialize_boxed_slice<T, S>(value: &Box<[T]>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    use serde::ser::SerializeTuple;
    use std::mem::size_of;

    // Solo para tipos POD (Plain Old Data)
    unsafe {
        if size_of::<T>() == 0 {
            #[warn(unused_mut)]
            return serializer
                .serialize_seq(Some(value.len()))
                .and_then(|s| s.end());
        }

        let ptr = value.as_ptr() as *const T;
        let slice = std::slice::from_raw_parts(ptr, value.len());

        let mut tup = serializer.serialize_tuple(slice.len())?;
        for item in slice {
            tup.serialize_element(item)?;
        }
        tup.end()
    }
}

struct ArcStrVisitor;

impl<'de> Visitor<'de> for ArcStrVisitor {
    type Value = Arc<str>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Arc::from(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Arc::from(v.to_owned()))
    }
}

pub fn deserialize_arc_str<'de, D>(deserializer: D) -> Result<Arc<str>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(ArcStrVisitor)
}

pub fn deserialize_boxed_slice<'de, T, D>(deserializer: D) -> Result<Box<[T]>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    struct BoxedSliceVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T> Visitor<'de> for BoxedSliceVisitor<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Box<[T]>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(value) = seq.next_element()? {
                values.push(value);
            }
            Ok(values.into_boxed_slice())
        }
    }

    deserializer.deserialize_seq(BoxedSliceVisitor(std::marker::PhantomData))
}
pub fn serialize_fxhash_arc<S, V>(
    map: &FxHashMap<Arc<str>, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    V: serde::Serialize,
{
    use serde::ser::SerializeMap;
    let mut map_serializer = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        map_serializer.serialize_entry(k.as_ref(), v)?;
    }
    map_serializer.end()
}

// Custom Deserializer (now using concrete String type)
pub fn deserialize_fxhash_arc<'de, V, D>(
    deserializer: D,
) -> Result<FxHashMap<Arc<str>, V>, D::Error>
where
    V: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    let temp_map = FxHashMap::<String, V>::deserialize(deserializer)?;
    Ok(temp_map
        .into_iter()
        .map(|(k, v)| (Arc::from(k), v))
        .collect())
}
