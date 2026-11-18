//! Serde Serializer implementation converting Rust types to [`QsValue`] and query strings.

use std::collections::BTreeMap;

use serde::ser::{
    self, Error as _, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, Serializer,
};

use crate::{
    error::QsError, options::StringifyOptions, stringify::stringify_query, value::QsValue,
};

/// Serializes a value into a query string using default options.
///
/// # Errors
///
/// Returns a [`QsError`] if serialization fails.
pub fn to_string<T: ser::Serialize + ?Sized>(value: &T) -> Result<String, QsError> {
    to_string_with_options(value, &StringifyOptions::default())
}

/// Serializes a value into a query string using the provided options.
///
/// # Errors
///
/// Returns a [`QsError`] if serialization fails.
pub fn to_string_with_options<T: ser::Serialize + ?Sized>(
    value: &T,
    options: &StringifyOptions,
) -> Result<String, QsError> {
    let qs_value = to_value(value)?;
    Ok(stringify_query(&qs_value, options))
}

/// Converts a serializable value into an intermediate [`QsValue`] representation.
///
/// # Errors
///
/// Returns a [`QsError`] if serialization fails.
pub fn to_value<T: ser::Serialize + ?Sized>(value: &T) -> Result<QsValue, QsError> {
    value.serialize(QsSerializer)
}

struct QsSerializer;

impl Serializer for QsSerializer {
    type Ok = QsValue;
    type Error = QsError;

    type SerializeSeq = QsSeqSerializer;
    type SerializeTuple = QsSeqSerializer;
    type SerializeTupleStruct = QsSeqSerializer;
    type SerializeTupleVariant = QsTupleVariantSerializer;
    type SerializeMap = QsMapSerializer;
    type SerializeStruct = QsStructSerializer;
    type SerializeStructVariant = QsStructVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Scalar(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let items = v.iter().map(|b| QsValue::Scalar(b.to_string())).collect();
        Ok(QsValue::Array(items))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Null)
    }

    fn serialize_some<T: ser::Serialize + ?Sized>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Null)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        let _ = name;
        Ok(QsValue::Null)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        let _ = (name, variant_index);
        Ok(QsValue::Scalar(variant.to_string()))
    }

    fn serialize_newtype_struct<T: ser::Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let _ = name;
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ser::Serialize + ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let _ = (name, variant_index);
        let mut map = BTreeMap::new();
        map.insert(variant.to_string(), value.serialize(self)?);
        Ok(QsValue::Object(map))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let _ = len;
        Ok(QsSeqSerializer { list: Vec::new() })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        let _ = len;
        Ok(QsSeqSerializer { list: Vec::new() })
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        let _ = (name, len);
        Ok(QsSeqSerializer { list: Vec::new() })
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        let _ = (name, variant_index, len);
        Ok(QsTupleVariantSerializer {
            variant,
            list: Vec::new(),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let _ = len;
        Ok(QsMapSerializer {
            map: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let _ = (name, len);
        Ok(QsStructSerializer {
            map: BTreeMap::new(),
        })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        let _ = (name, variant_index, len);
        Ok(QsStructVariantSerializer {
            variant,
            inner: QsStructSerializer {
                map: BTreeMap::new(),
            },
        })
    }
}

struct QsSeqSerializer {
    list: Vec<QsValue>,
}

impl QsSeqSerializer {
    fn push_element<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<(), QsError> {
        self.list.push(value.serialize(QsSerializer)?);
        Ok(())
    }
}

impl SerializeSeq for QsSeqSerializer {
    type Ok = QsValue;
    type Error = QsError;

    fn serialize_element<T: ser::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.push_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Array(self.list))
    }
}

impl SerializeTuple for QsSeqSerializer {
    type Ok = QsValue;
    type Error = QsError;

    fn serialize_element<T: ser::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.push_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Array(self.list))
    }
}

impl SerializeTupleStruct for QsSeqSerializer {
    type Ok = QsValue;
    type Error = QsError;

    fn serialize_field<T: ser::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.push_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Array(self.list))
    }
}

struct QsTupleVariantSerializer {
    variant: &'static str,
    list: Vec<QsValue>,
}

impl SerializeTupleVariant for QsTupleVariantSerializer {
    type Ok = QsValue;
    type Error = QsError;

    fn serialize_field<T: ser::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.list.push(value.serialize(QsSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map = BTreeMap::new();
        map.insert(self.variant.to_string(), QsValue::Array(self.list));
        Ok(QsValue::Object(map))
    }
}

struct QsMapSerializer {
    map: BTreeMap<String, QsValue>,
    next_key: Option<String>,
}

impl SerializeMap for QsMapSerializer {
    type Ok = QsValue;
    type Error = QsError;

    fn serialize_key<T: ser::Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        let key_value = key.serialize(QsSerializer)?;
        match key_value {
            QsValue::Scalar(text) => {
                self.next_key = Some(text);
                Ok(())
            }
            _ => Err(QsError::custom("map key must serialize to a string")),
        }
    }

    fn serialize_value<T: ser::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        let key = self
            .next_key
            .take()
            .ok_or_else(|| QsError::custom("serialize_value called before serialize_key"))?;
        let val = value.serialize(QsSerializer)?;
        self.map.insert(key, val);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Object(self.map))
    }
}

struct QsStructSerializer {
    map: BTreeMap<String, QsValue>,
}

impl SerializeStruct for QsStructSerializer {
    type Ok = QsValue;
    type Error = QsError;

    fn serialize_field<T: ser::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.map
            .insert(key.to_string(), value.serialize(QsSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(QsValue::Object(self.map))
    }
}

struct QsStructVariantSerializer {
    variant: &'static str,
    inner: QsStructSerializer,
}

impl SerializeStructVariant for QsStructVariantSerializer {
    type Ok = QsValue;
    type Error = QsError;

    fn serialize_field<T: ser::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.inner.serialize_field(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut outer = BTreeMap::new();
        outer.insert(self.variant.to_string(), self.inner.end()?);
        Ok(QsValue::Object(outer))
    }
}

impl ser::Serialize for QsValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Scalar(text) => serializer.serialize_str(text),
            Self::Array(list) => {
                let mut seq = serializer.serialize_seq(Some(list.len()))?;
                for item in list {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Self::Object(map) => {
                let mut map_ser = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    map_ser.serialize_entry(k, v)?;
                }
                map_ser.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_serializer_value_before_key_error() {
        let mut map_ser = QsMapSerializer {
            map: BTreeMap::new(),
            next_key: None,
        };
        let res = map_ser.serialize_value(&10u32);
        assert!(res.is_err());
    }

    #[test]
    fn test_qs_serializer_direct_methods() {
        assert_eq!(
            QsSerializer.serialize_char('A').unwrap(),
            QsValue::Scalar("A".to_string())
        );
        assert_eq!(
            QsSerializer.serialize_str("hello").unwrap(),
            QsValue::Scalar("hello".to_string())
        );
        assert_eq!(QsSerializer.serialize_none().unwrap(), QsValue::Null);
        assert_eq!(QsSerializer.serialize_unit().unwrap(), QsValue::Null);
        assert_eq!(
            QsSerializer.serialize_unit_struct("Unit").unwrap(),
            QsValue::Null
        );
        assert_eq!(
            QsSerializer
                .serialize_unit_variant("Enum", 0, "Variant")
                .unwrap(),
            QsValue::Scalar("Variant".to_string())
        );
    }

    #[test]
    fn test_qs_serializer_more_methods() {
        assert_eq!(
            QsSerializer.serialize_f32(12.5).unwrap(),
            QsValue::Scalar("12.5".to_string())
        );
        assert_eq!(
            QsSerializer.serialize_f64(45.75).unwrap(),
            QsValue::Scalar("45.75".to_string())
        );
        assert_eq!(
            QsSerializer
                .serialize_newtype_struct("Wrapper", &42u32)
                .unwrap(),
            QsValue::Scalar("42".to_string())
        );
        assert_eq!(
            QsSerializer.serialize_bytes(b"hi").unwrap(),
            QsValue::Array(vec![
                QsValue::Scalar("104".to_string()),
                QsValue::Scalar("105".to_string()),
            ])
        );
    }
}
