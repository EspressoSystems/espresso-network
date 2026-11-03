use std::{cell::Cell, fmt};

use bincode::Options as _;
use serde::de::{
    self, DeserializeSeed, Deserializer, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};

// Bincode recurses once per nested value with no bound of its own, so a peer can exhaust the stack
// with a few megabytes. Scalars count as values. The deepest legitimate payload is a whole reward v2
// tree in a catchup response: 160 levels at two values each, plus a few for the leaf and envelope.
// The limit assumes decoding runs on a stack of at least 2 MiB, the size of a tokio worker's.
const MAX_DEPTH: usize = 512;

/// Deserialize a bincode value from bytes received from a peer.
///
/// Decodes exactly like [`bincode::deserialize`], but rejects input nested more than a few hundred
/// values deep instead of recursing until the stack overflows.
pub fn deserialize_limited<'de, T>(bytes: &'de [u8]) -> bincode::Result<T>
where
    T: de::Deserialize<'de>,
{
    let options = bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();
    let mut deserializer = bincode::Deserializer::from_slice(bytes, options);
    let depth = Cell::new(0);
    T::deserialize(Limited {
        inner: &mut deserializer,
        depth: &depth,
    })
}

/// Wraps a deserializer, visitor, access or seed so every value nested under it is counted.
struct Limited<'a, T> {
    inner: T,
    depth: &'a Cell<usize>,
}

impl<'a, T> Limited<'a, T> {
    fn wrap<U>(&self, inner: U) -> Limited<'a, U> {
        Limited {
            inner,
            depth: self.depth,
        }
    }
}

struct DepthGuard<'a>(&'a Cell<usize>);

impl Drop for DepthGuard<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}

fn enter<E>(depth: &Cell<usize>) -> Result<DepthGuard<'_>, E>
where
    E: de::Error,
{
    let next = depth.get() + 1;
    if next > MAX_DEPTH {
        return Err(E::custom(format_args!(
            "input is nested more than {MAX_DEPTH} values deep"
        )));
    }
    depth.set(next);
    Ok(DepthGuard(depth))
}

macro_rules! forward_deserialize {
    ($($method:ident($($arg:ident: $ty:ty),*)),* $(,)?) => {$(
        fn $method<V>(self, $($arg: $ty,)* visitor: V) -> Result<V::Value, D::Error>
        where
            V: Visitor<'de>,
        {
            let _guard = enter::<D::Error>(self.depth)?;
            let visitor = self.wrap(visitor);
            self.inner.$method($($arg,)* visitor)
        }
    )*};
}

impl<'de, D> Deserializer<'de> for Limited<'_, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    forward_deserialize! {
        deserialize_any(),
        deserialize_bool(),
        deserialize_i8(),
        deserialize_i16(),
        deserialize_i32(),
        deserialize_i64(),
        deserialize_i128(),
        deserialize_u8(),
        deserialize_u16(),
        deserialize_u32(),
        deserialize_u64(),
        deserialize_u128(),
        deserialize_f32(),
        deserialize_f64(),
        deserialize_char(),
        deserialize_str(),
        deserialize_string(),
        deserialize_bytes(),
        deserialize_byte_buf(),
        deserialize_option(),
        deserialize_unit(),
        deserialize_unit_struct(name: &'static str),
        deserialize_newtype_struct(name: &'static str),
        deserialize_seq(),
        deserialize_tuple(len: usize),
        deserialize_tuple_struct(name: &'static str, len: usize),
        deserialize_map(),
        deserialize_struct(name: &'static str, fields: &'static [&'static str]),
        deserialize_enum(name: &'static str, variants: &'static [&'static str]),
        deserialize_identifier(),
        deserialize_ignored_any(),
    }

    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}

macro_rules! forward_visit {
    ($($method:ident($ty:ty)),* $(,)?) => {$(
        fn $method<E>(self, v: $ty) -> Result<V::Value, E>
        where
            E: de::Error,
        {
            self.inner.$method(v)
        }
    )*};
}

impl<'de, V> Visitor<'de> for Limited<'_, V>
where
    V: Visitor<'de>,
{
    type Value = V::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.expecting(formatter)
    }

    forward_visit! {
        visit_bool(bool),
        visit_i8(i8),
        visit_i16(i16),
        visit_i32(i32),
        visit_i64(i64),
        visit_i128(i128),
        visit_u8(u8),
        visit_u16(u16),
        visit_u32(u32),
        visit_u64(u64),
        visit_u128(u128),
        visit_f32(f32),
        visit_f64(f64),
        visit_char(char),
        visit_str(&str),
        visit_borrowed_str(&'de str),
        visit_string(String),
        visit_bytes(&[u8]),
        visit_borrowed_bytes(&'de [u8]),
        visit_byte_buf(Vec<u8>),
    }

    fn visit_none<E>(self) -> Result<V::Value, E>
    where
        E: de::Error,
    {
        self.inner.visit_none()
    }

    fn visit_unit<E>(self) -> Result<V::Value, E>
    where
        E: de::Error,
    {
        self.inner.visit_unit()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<V::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let deserializer = self.wrap(deserializer);
        self.inner.visit_some(deserializer)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<V::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let deserializer = self.wrap(deserializer);
        self.inner.visit_newtype_struct(deserializer)
    }

    fn visit_seq<A>(self, seq: A) -> Result<V::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let seq = self.wrap(seq);
        self.inner.visit_seq(seq)
    }

    fn visit_map<A>(self, map: A) -> Result<V::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let map = self.wrap(map);
        self.inner.visit_map(map)
    }

    fn visit_enum<A>(self, data: A) -> Result<V::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let data = self.wrap(data);
        self.inner.visit_enum(data)
    }
}

impl<'de, S> DeserializeSeed<'de> for Limited<'_, S>
where
    S: DeserializeSeed<'de>,
{
    type Value = S::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<S::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let deserializer = self.wrap(deserializer);
        self.inner.deserialize(deserializer)
    }
}

impl<'de, A> SeqAccess<'de> for Limited<'_, A>
where
    A: SeqAccess<'de>,
{
    type Error = A::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, A::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let seed = self.wrap(seed);
        self.inner.next_element_seed(seed)
    }

    fn size_hint(&self) -> Option<usize> {
        self.inner.size_hint()
    }
}

impl<'de, A> MapAccess<'de> for Limited<'_, A>
where
    A: MapAccess<'de>,
{
    type Error = A::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, A::Error>
    where
        K: DeserializeSeed<'de>,
    {
        let seed = self.wrap(seed);
        self.inner.next_key_seed(seed)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, A::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let seed = self.wrap(seed);
        self.inner.next_value_seed(seed)
    }

    fn size_hint(&self) -> Option<usize> {
        self.inner.size_hint()
    }
}

impl<'a, 'de, A> EnumAccess<'de> for Limited<'a, A>
where
    A: EnumAccess<'de>,
{
    type Error = A::Error;
    type Variant = Limited<'a, A::Variant>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Limited<'a, A::Variant>), A::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let depth = self.depth;
        let (value, variant) = self.inner.variant_seed(seed)?;
        Ok((
            value,
            Limited {
                inner: variant,
                depth,
            },
        ))
    }
}

impl<'de, A> VariantAccess<'de> for Limited<'_, A>
where
    A: VariantAccess<'de>,
{
    type Error = A::Error;

    fn unit_variant(self) -> Result<(), A::Error> {
        self.inner.unit_variant()
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, A::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let seed = self.wrap(seed);
        self.inner.newtype_variant_seed(seed)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, A::Error>
    where
        V: Visitor<'de>,
    {
        let visitor = self.wrap(visitor);
        self.inner.tuple_variant(len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, A::Error>
    where
        V: Visitor<'de>,
    {
        let visitor = self.wrap(visitor);
        self.inner.struct_variant(fields, visitor)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Nest {
        End,
        Inner(Box<Nest>),
    }

    fn nest(values: usize) -> Nest {
        (1..values).fold(Nest::End, |inner, _| Nest::Inner(Box::new(inner)))
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Shape {
        Unit,
        Tuple(u8, i64),
        Struct { name: String, bytes: Vec<u8> },
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Sample {
        flag: bool,
        wide: u128,
        float: f64,
        text: String,
        maybe: Option<Box<Sample>>,
        shapes: Vec<Shape>,
        map: BTreeMap<u32, (char, Option<u16>)>,
        unit: (),
    }

    fn sample() -> Sample {
        Sample {
            flag: true,
            wide: u128::MAX,
            float: 1.5,
            text: "hello".to_owned(),
            maybe: Some(Box::new(Sample {
                flag: false,
                wide: 0,
                float: -0.0,
                text: String::new(),
                maybe: None,
                shapes: Vec::new(),
                map: BTreeMap::new(),
                unit: (),
            })),
            shapes: Vec::from([
                Shape::Unit,
                Shape::Tuple(7, -9),
                Shape::Struct {
                    name: "x".to_owned(),
                    bytes: Vec::from([1, 2, 3]),
                },
            ]),
            map: BTreeMap::from([(1, ('a', None)), (2, ('b', Some(3)))]),
            unit: (),
        }
    }

    #[test]
    fn decodes_like_bincode() {
        let mut bytes = bincode::serialize(&sample()).unwrap();
        bytes.extend([0xff, 0xff]);

        let limited = deserialize_limited::<Sample>(&bytes).unwrap();
        let unlimited = bincode::deserialize::<Sample>(&bytes).unwrap();
        assert_eq!(limited, unlimited);
        assert_eq!(limited, sample());
    }

    #[test]
    fn accepts_nesting_at_the_limit() {
        let bytes = bincode::serialize(&nest(MAX_DEPTH)).unwrap();
        assert_eq!(
            deserialize_limited::<Nest>(&bytes).unwrap(),
            nest(MAX_DEPTH)
        );
    }

    #[test]
    fn rejects_nesting_past_the_limit() {
        let bytes = bincode::serialize(&nest(MAX_DEPTH + 1)).unwrap();
        let err = deserialize_limited::<Nest>(&bytes).unwrap_err();
        assert!(err.to_string().contains("nested more than"), "{err}");
    }

    #[test]
    fn rejects_input_bincode_rejects() {
        let limited = deserialize_limited::<Sample>(&[1, 2, 3]).unwrap_err();
        let unlimited = bincode::deserialize::<Sample>(&[1, 2, 3]).unwrap_err();
        assert_eq!(limited.to_string(), unlimited.to_string());
    }
}
