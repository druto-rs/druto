//! Serde Deserializer implementation for [`QsValue`] and query strings.

use std::collections::btree_map;

use serde::de::{
    self, DeserializeOwned, DeserializeSeed, Deserializer, EnumAccess, IntoDeserializer, MapAccess,
    SeqAccess, VariantAccess, Visitor,
};

use crate::{error::QsError, options::ParseOptions, parser::parse_query, value::QsValue};

/// Deserializes a query string into a strongly typed value using default options.
///
/// # Errors
///
/// Returns [`QsError`] if parsing or deserialization fails.
pub fn from_str<T: DeserializeOwned>(query: &str) -> Result<T, QsError> {
    from_str_with_options(query, &ParseOptions::default())
}

/// Deserializes a query string into a strongly typed value using the given options.
///
/// # Errors
///
/// Returns [`QsError`] if parsing or deserialization fails.
pub fn from_str_with_options<T: DeserializeOwned>(
    query: &str,
    options: &ParseOptions,
) -> Result<T, QsError> {
    let value = parse_query(query, options)?;
    T::deserialize(value)
}

impl<'de> Deserializer<'de> for QsValue {
    type Error = QsError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Null => visitor.visit_none(),
            Self::Scalar(text) => visitor.visit_string(text),
            Self::Array(list) => visitor.visit_seq(QsSeqAccess::new(list)),
            Self::Object(map) => visitor.visit_map(QsMapAccess::new(map)),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Scalar(ref text) => match text.to_ascii_lowercase().as_str() {
                "true" | "1" | "t" | "yes" | "on" => visitor.visit_bool(true),
                "false" | "0" | "f" | "no" | "off" => visitor.visit_bool(false),
                _ => Err(de::Error::invalid_value(
                    de::Unexpected::Str(text),
                    &"a boolean value",
                )),
            },
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Scalar(ref text) => text.parse::<i64>().map_or_else(
                |_| {
                    Err(de::Error::invalid_value(
                        de::Unexpected::Str(text),
                        &"an integer",
                    ))
                },
                |val| visitor.visit_i64(val),
            ),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Scalar(ref text) => text.parse::<i128>().map_or_else(
                |_| {
                    Err(de::Error::invalid_value(
                        de::Unexpected::Str(text),
                        &"an i128 integer",
                    ))
                },
                |val| visitor.visit_i128(val),
            ),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Scalar(ref text) => text.parse::<u64>().map_or_else(
                |_| {
                    Err(de::Error::invalid_value(
                        de::Unexpected::Str(text),
                        &"an unsigned integer",
                    ))
                },
                |val| visitor.visit_u64(val),
            ),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Scalar(ref text) => text.parse::<u128>().map_or_else(
                |_| {
                    Err(de::Error::invalid_value(
                        de::Unexpected::Str(text),
                        &"a u128 integer",
                    ))
                },
                |val| visitor.visit_u128(val),
            ),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Scalar(ref text) => text.parse::<f64>().map_or_else(
                |_| {
                    Err(de::Error::invalid_value(
                        de::Unexpected::Str(text),
                        &"a floating point number",
                    ))
                },
                |val| visitor.visit_f64(val),
            ),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Scalar(ref text) => {
                let mut chars = text.chars();
                match (chars.next(), chars.next()) {
                    (Some(ch), None) => visitor.visit_char(ch),
                    _ => Err(de::Error::invalid_value(
                        de::Unexpected::Str(text),
                        &"a single character",
                    )),
                }
            }
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Scalar(text) => visitor.visit_string(text),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Scalar(text) => visitor.visit_bytes(text.as_bytes()),
            Self::Array(list) => {
                let bytes: Result<Vec<u8>, Self::Error> = list
                    .into_iter()
                    .map(|item| match item {
                        Self::Scalar(text) => text.parse::<u8>().map_err(|_| {
                            de::Error::invalid_value(de::Unexpected::Str(&text), &"a byte (0..255)")
                        }),
                        _ => Err(de::Error::custom("expected scalar byte in array")),
                    })
                    .collect();
                visitor.visit_byte_buf(bytes?)
            }
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Null => visitor.visit_none(),
            Self::Scalar(ref text) if text.is_empty() => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Null => visitor.visit_unit(),
            Self::Scalar(ref text) if text.is_empty() => visitor.visit_unit(),
            _ => Err(de::Error::invalid_type(
                de::Unexpected::Other("non-unit value"),
                &"unit ()",
            )),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let _ = name;
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let _ = name;
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Array(list) => visitor.visit_seq(QsSeqAccess::new(list)),
            Self::Null => visitor.visit_seq(QsSeqAccess::new(Vec::new())),
            other => visitor.visit_seq(QsSeqAccess::new(vec![other])),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let _ = len;
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let _ = (name, len);
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Object(map) => visitor.visit_map(QsMapAccess::new(map)),
            _ => Err(de::Error::invalid_type(
                de::Unexpected::Other("non-object value"),
                &"a map",
            )),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let _ = (name, fields);
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let _ = (name, variants);
        match self {
            Self::Scalar(name) => visitor.visit_enum(ScalarEnumAccess(name)),
            Self::Object(mut map) => {
                if map.len() == 1 {
                    let (variant, val) = map.pop_first().expect("map verified with len 1");
                    visitor.visit_enum(MapEnumAccess(variant, val))
                } else {
                    Err(de::Error::custom(
                        "expected single-key map for enum variant",
                    ))
                }
            }
            _ => Err(de::Error::invalid_type(
                de::Unexpected::Other("non-enum value"),
                &"an enum identifier or object",
            )),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Scalar(text) => visitor.visit_string(text),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }
}

struct QsSeqAccess {
    iter: std::vec::IntoIter<QsValue>,
}

impl QsSeqAccess {
    fn new(list: Vec<QsValue>) -> Self {
        Self {
            iter: list.into_iter(),
        }
    }
}

impl<'de> SeqAccess<'de> for QsSeqAccess {
    type Error = QsError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }
}

struct QsMapAccess {
    iter: btree_map::IntoIter<String, QsValue>,
    current_value: Option<QsValue>,
}

impl QsMapAccess {
    fn new(map: std::collections::BTreeMap<String, QsValue>) -> Self {
        Self {
            iter: map.into_iter(),
            current_value: None,
        }
    }
}

impl<'de> MapAccess<'de> for QsMapAccess {
    type Error = QsError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.iter.next() {
            Some((key, val)) => {
                self.current_value = Some(val);
                seed.deserialize(key.into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        let val = self
            .current_value
            .take()
            .ok_or_else(|| de::Error::custom("next_value called before next_key"))?;
        seed.deserialize(val)
    }
}

struct ScalarEnumAccess(String);

impl<'de> EnumAccess<'de> for ScalarEnumAccess {
    type Error = QsError;
    type Variant = UnitVariantAccess;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let variant = seed.deserialize(self.0.into_deserializer())?;
        Ok((variant, UnitVariantAccess))
    }
}

struct UnitVariantAccess;

impl<'de> VariantAccess<'de> for UnitVariantAccess {
    type Error = QsError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        let _ = seed;
        Err(de::Error::invalid_type(
            de::Unexpected::UnitVariant,
            &"newtype variant",
        ))
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let _ = (len, visitor);
        Err(de::Error::invalid_type(
            de::Unexpected::UnitVariant,
            &"tuple variant",
        ))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let _ = (fields, visitor);
        Err(de::Error::invalid_type(
            de::Unexpected::UnitVariant,
            &"struct variant",
        ))
    }
}

struct MapEnumAccess(String, QsValue);

impl<'de> EnumAccess<'de> for MapEnumAccess {
    type Error = QsError;
    type Variant = ValueVariantAccess;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let variant = seed.deserialize(self.0.into_deserializer())?;
        Ok((variant, ValueVariantAccess(self.1)))
    }
}

struct ValueVariantAccess(QsValue);

impl<'de> VariantAccess<'de> for ValueVariantAccess {
    type Error = QsError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        self.0.deserialize_unit(de::IgnoredAny)?;
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        seed.deserialize(self.0)
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let _ = len;
        self.0.deserialize_seq(visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let _ = fields;
        self.0.deserialize_map(visitor)
    }
}

#[derive(Copy, Clone)]
struct QsValueVisitor;

impl<'de> Visitor<'de> for QsValueVisitor {
    type Value = QsValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("any valid query string value")
    }

    fn visit_bool<E: de::Error>(self, val: bool) -> Result<Self::Value, E> {
        Ok(QsValue::Scalar(val.to_string()))
    }

    fn visit_i64<E: de::Error>(self, val: i64) -> Result<Self::Value, E> {
        Ok(QsValue::Scalar(val.to_string()))
    }

    fn visit_u64<E: de::Error>(self, val: u64) -> Result<Self::Value, E> {
        Ok(QsValue::Scalar(val.to_string()))
    }

    fn visit_f64<E: de::Error>(self, val: f64) -> Result<Self::Value, E> {
        Ok(QsValue::Scalar(val.to_string()))
    }

    fn visit_str<E: de::Error>(self, val: &str) -> Result<Self::Value, E> {
        Ok(QsValue::Scalar(val.to_string()))
    }

    fn visit_string<E: de::Error>(self, val: String) -> Result<Self::Value, E> {
        Ok(QsValue::Scalar(val))
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(QsValue::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        de::Deserialize::deserialize(deserializer)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(QsValue::Null)
    }

    fn visit_seq<S: SeqAccess<'de>>(self, mut access: S) -> Result<Self::Value, S::Error> {
        let mut list = Vec::new();
        while let Some(item) = access.next_element()? {
            list.push(item);
        }
        Ok(QsValue::Array(list))
    }

    fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
        let mut map = std::collections::BTreeMap::new();
        while let Some((key, val)) = access.next_entry()? {
            map.insert(key, val);
        }
        Ok(QsValue::Object(map))
    }
}

impl<'de> de::Deserialize<'de> for QsValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(QsValueVisitor)
    }
}

#[cfg(test)]
mod tests {
    use serde::de::{DeserializeSeed, Expected, IgnoredAny, VariantAccess};

    use super::*;

    struct TestSeed;

    impl<'de> DeserializeSeed<'de> for TestSeed {
        type Value = ();

        fn deserialize<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            let _ = deserializer;
            Ok(())
        }
    }

    #[test]
    fn test_unit_variant_access_errors() {
        assert!(TestSeed.deserialize(QsValue::Null).is_ok());
        assert!(UnitVariantAccess.unit_variant().is_ok());
        assert!(UnitVariantAccess.newtype_variant_seed(TestSeed).is_err());
        assert!(UnitVariantAccess.tuple_variant(2, IgnoredAny).is_err());
        assert!(UnitVariantAccess.struct_variant(&[], IgnoredAny).is_err());
    }

    #[test]
    fn test_map_access_value_before_key_error() {
        let mut map_access = QsMapAccess::new(std::collections::BTreeMap::new());
        let res = map_access.next_value_seed(TestSeed);
        assert!(res.is_err());
    }

    #[test]
    fn test_deserializer_invalid_scalar_errors() {
        let s = QsValue::from("not_a_bool");
        assert!(s.deserialize_bool(IgnoredAny).is_err());

        let s2 = QsValue::from("not_an_int");
        assert!(s2.deserialize_i64(IgnoredAny).is_err());

        let s3 = QsValue::from("not_a_u64");
        assert!(s3.deserialize_u64(IgnoredAny).is_err());

        let s4 = QsValue::from("not_a_f64");
        assert!(s4.deserialize_f64(IgnoredAny).is_err());

        let s5 = QsValue::from("not_a_char");
        assert!(s5.deserialize_char(IgnoredAny).is_err());
    }

    #[test]
    fn test_deserialize_identifier() {
        let s = QsValue::from("field_name");
        assert!(s.deserialize_identifier(IgnoredAny).is_ok());

        let non_s = QsValue::Null;
        assert!(non_s.deserialize_identifier(IgnoredAny).is_ok());
    }

    #[test]
    fn test_qs_value_visitor_direct() {
        let visitor = QsValueVisitor;
        assert_eq!(
            visitor.visit_bool::<QsError>(true).unwrap(),
            QsValue::Scalar("true".to_string())
        );
        assert_eq!(
            visitor.visit_i64::<QsError>(42).unwrap(),
            QsValue::Scalar("42".to_string())
        );
        assert_eq!(
            visitor.visit_u64::<QsError>(42).unwrap(),
            QsValue::Scalar("42".to_string())
        );
        assert_eq!(
            visitor.visit_f64::<QsError>(12.5).unwrap(),
            QsValue::Scalar("12.5".to_string())
        );
        assert_eq!(
            visitor.visit_str::<QsError>("test").unwrap(),
            QsValue::Scalar("test".to_string())
        );
        assert_eq!(
            visitor.visit_string::<QsError>("test".to_string()).unwrap(),
            QsValue::Scalar("test".to_string())
        );
        assert_eq!(visitor.visit_none::<QsError>().unwrap(), QsValue::Null);
        assert_eq!(visitor.visit_unit::<QsError>().unwrap(), QsValue::Null);
        let expected: &dyn Expected = &visitor;
        assert_eq!(format!("{expected}"), "any valid query string value");
        assert_eq!(
            visitor.visit_some::<QsValue>(QsValue::from("val")).unwrap(),
            QsValue::Scalar("val".to_string())
        );
    }

    #[test]
    fn test_deserialize_any_all_variants() {
        struct AnyVisitor;
        impl<'de> Visitor<'de> for AnyVisitor {
            type Value = String;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "any value")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(format!("scalar: {v}"))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                self.visit_str(&v)
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok("null".to_string())
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut count = 0;
                while let Some(IgnoredAny) = seq.next_element()? {
                    count += 1;
                }
                Ok(format!("array: {count}"))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut count = 0;
                while let Some((IgnoredAny, IgnoredAny)) = map.next_entry()? {
                    count += 1;
                }
                Ok(format!("map: {count}"))
            }
        }

        let exp: &dyn Expected = &AnyVisitor;
        assert_eq!(format!("{exp}"), "any value");
        assert_eq!(QsValue::Null.deserialize_any(AnyVisitor).unwrap(), "null");
        assert_eq!(
            QsValue::Scalar("hello".to_string())
                .deserialize_any(AnyVisitor)
                .unwrap(),
            "scalar: hello"
        );
        assert_eq!(
            QsValue::Array(vec![QsValue::from("a")])
                .deserialize_any(AnyVisitor)
                .unwrap(),
            "array: 1"
        );
        let mut map = std::collections::BTreeMap::new();
        map.insert("k".to_string(), QsValue::from("v"));
        assert_eq!(
            QsValue::Object(map).deserialize_any(AnyVisitor).unwrap(),
            "map: 1"
        );
    }

    #[test]
    fn test_deserialize_byte_buf() {
        struct ByteBufVisitor;
        impl<'de> Visitor<'de> for ByteBufVisitor {
            type Value = Vec<u8>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "byte buf")
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(v.to_vec())
            }
            fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(v)
            }
        }
        let exp: &dyn Expected = &ByteBufVisitor;
        assert_eq!(format!("{exp}"), "byte buf");
        let val = QsValue::Scalar("hello".to_string());
        let res = val.deserialize_byte_buf(ByteBufVisitor).unwrap();
        assert_eq!(res, b"hello");

        let arr = QsValue::Array(vec![QsValue::from("104"), QsValue::from("105")]);
        let res_arr = arr.deserialize_byte_buf(ByteBufVisitor).unwrap();
        assert_eq!(res_arr, b"hi");
    }

    #[test]
    fn test_deserialize_option_null() {
        let opt: Option<String> = serde::Deserialize::deserialize(QsValue::Null).unwrap();
        assert_eq!(opt, None);
    }

    #[test]
    fn test_more_de_branches() {
        assert!(
            QsValue::from("1")
                .deserialize_newtype_struct("Id", IgnoredAny)
                .is_ok()
        );
        assert!(
            QsValue::Null
                .deserialize_unit_struct("Unit", IgnoredAny)
                .is_ok()
        );
        assert!(
            QsValue::Array(vec![])
                .deserialize_tuple_struct("Tuple", 0, IgnoredAny)
                .is_ok()
        );
        assert!(
            QsValue::Array(vec![])
                .deserialize_tuple(0, IgnoredAny)
                .is_ok()
        );
    }
}
