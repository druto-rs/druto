use std::{any::Any, collections::HashMap, fmt};

use crate::error::EnvError;

/// An immutable, thread-safe container holding validated typed environment variables.
pub struct Env {
    raw_values: HashMap<String, String>,
    typed_values: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl fmt::Debug for Env {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut keys: Vec<&String> = self.raw_values.keys().collect();
        keys.sort();
        formatter.debug_struct("Env").field("keys", &keys).finish()
    }
}

impl Env {
    /// Constructs a new `Env` instance with the given raw and typed mappings.
    pub fn new(
        raw_values: HashMap<String, String>,
        typed_values: HashMap<String, Box<dyn Any + Send + Sync>>,
    ) -> Self {
        Self {
            raw_values,
            typed_values,
        }
    }

    /// Retrieves a typed reference to a validated environment variable.
    ///
    /// # Errors
    /// Returns `EnvError::NotFound` if the key was not defined or validated.
    /// Returns `EnvError::TypeMismatch` if the key cannot be downcast to `T`.
    pub fn get<T: 'static>(&self, key: &str) -> Result<&T, EnvError> {
        match self.typed_values.get(key) {
            Some(boxed) => match boxed.downcast_ref::<T>() {
                Some(value) => Ok(value),
                None => Err(EnvError::TypeMismatch {
                    key: key.to_string(),
                    expected: std::any::type_name::<T>(),
                }),
            },
            None => Err(EnvError::NotFound {
                key: key.to_string(),
            }),
        }
    }

    /// Retrieves the raw string representation of an environment variable.
    pub fn get_raw(&self, key: &str) -> Option<&str> {
        self.raw_values.get(key).map(String::as_str)
    }

    /// Checks whether an environment variable exists in the validated set or raw map.
    pub fn contains(&self, key: &str) -> bool {
        self.typed_values.contains_key(key) || self.raw_values.contains_key(key)
    }

    /// Returns a reference to all raw environment key-value pairs.
    pub fn raw_map(&self) -> &HashMap<String, String> {
        &self.raw_values
    }

    /// Exports all raw environment variables into the process-level environment (`std::env`).
    ///
    /// # Safety
    /// Modifying process environment variables via POSIX `setenv` is inherently not thread-safe.
    /// This method must only be called during early single-threaded application bootstrap before
    /// any worker threads or async runtimes have been spawned.
    pub unsafe fn export_to_process(&self) {
        for (key, value) in &self.raw_values {
            // Safety: caller guarantees single-threaded boot phase
            unsafe {
                std::env::set_var(key, value);
            }
        }
    }
}

#[cfg(feature = "serde")]
mod de {
    use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};

    use super::*;

    impl Env {
        /// Deserializes the raw environment variables into a strongly-typed struct.
        pub fn deserialize<'de, T: serde::Deserialize<'de>>(&self) -> Result<T, EnvError> {
            T::deserialize(EnvDeserializer {
                raw_map: &self.raw_values,
            })
            .map_err(|err| EnvError::Deserialize(err.to_string()))
        }
    }

    struct EnvDeserializer<'a> {
        raw_map: &'a HashMap<String, String>,
    }

    impl<'de, 'a> Deserializer<'de> for EnvDeserializer<'a> {
        type Error = de::value::Error;

        fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            visitor.visit_map(EnvMapAccess {
                iter: self.raw_map.iter(),
                current_value: None,
            })
        }

        fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.deserialize_any(visitor)
        }

        fn deserialize_struct<V: Visitor<'de>>(
            self,
            name: &'static str,
            fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            let _ = (name, fields);
            self.deserialize_any(visitor)
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct enum identifier ignored_any
        }
    }

    struct EnvMapAccess<'a> {
        iter: std::collections::hash_map::Iter<'a, String, String>,
        current_value: Option<&'a str>,
    }

    impl<'de, 'a> MapAccess<'de> for EnvMapAccess<'a> {
        type Error = de::value::Error;

        fn next_key_seed<K: DeserializeSeed<'de>>(
            &mut self,
            seed: K,
        ) -> Result<Option<K::Value>, Self::Error> {
            if let Some((key, value)) = self.iter.next() {
                self.current_value = Some(value.as_str());
                seed.deserialize(de::IntoDeserializer::into_deserializer(key.as_str()))
                    .map(Some)
            } else {
                Ok(None)
            }
        }

        fn next_value_seed<V: DeserializeSeed<'de>>(
            &mut self,
            seed: V,
        ) -> Result<V::Value, Self::Error> {
            let value_str = self
                .current_value
                .take()
                .expect("next_value_seed called before next_key_seed");
            seed.deserialize(StringValueDeserializer { value: value_str })
        }
    }

    struct StringValueDeserializer<'a> {
        value: &'a str,
    }

    impl<'de, 'a> Deserializer<'de> for StringValueDeserializer<'a> {
        type Error = de::value::Error;

        fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            visitor.visit_str(self.value)
        }

        fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            match self.value.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => visitor.visit_bool(true),
                "false" | "0" | "no" | "off" => visitor.visit_bool(false),
                _ => Err(de::Error::custom(format!(
                    "invalid boolean value `{}`",
                    self.value
                ))),
            }
        }

        fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.value
                .trim()
                .parse::<i8>()
                .map_err(de::Error::custom)
                .and_then(|v| visitor.visit_i8(v))
        }

        fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.value
                .trim()
                .parse::<i16>()
                .map_err(de::Error::custom)
                .and_then(|v| visitor.visit_i16(v))
        }

        fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.value
                .trim()
                .parse::<i32>()
                .map_err(de::Error::custom)
                .and_then(|v| visitor.visit_i32(v))
        }

        fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.value
                .trim()
                .parse::<i64>()
                .map_err(de::Error::custom)
                .and_then(|v| visitor.visit_i64(v))
        }

        fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.value
                .trim()
                .parse::<u8>()
                .map_err(de::Error::custom)
                .and_then(|v| visitor.visit_u8(v))
        }

        fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.value
                .trim()
                .parse::<u16>()
                .map_err(de::Error::custom)
                .and_then(|v| visitor.visit_u16(v))
        }

        fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.value
                .trim()
                .parse::<u32>()
                .map_err(de::Error::custom)
                .and_then(|v| visitor.visit_u32(v))
        }

        fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.value
                .trim()
                .parse::<u64>()
                .map_err(de::Error::custom)
                .and_then(|v| visitor.visit_u64(v))
        }

        fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.value
                .trim()
                .parse::<f32>()
                .map_err(de::Error::custom)
                .and_then(|v| visitor.visit_f32(v))
        }

        fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.value
                .trim()
                .parse::<f64>()
                .map_err(de::Error::custom)
                .and_then(|v| visitor.visit_f64(v))
        }

        fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            if self.value.trim().is_empty() {
                visitor.visit_none()
            } else {
                visitor.visit_some(self)
            }
        }

        fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            let parts: Vec<&str> = self
                .value
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            visitor.visit_seq(SeqSliceAccess {
                iter: parts.into_iter(),
            })
        }

        serde::forward_to_deserialize_any! {
            i128 u128 char str string bytes byte_buf unit unit_struct
            newtype_struct tuple tuple_struct map struct enum identifier ignored_any
        }
    }

    struct SeqSliceAccess<'a> {
        iter: std::vec::IntoIter<&'a str>,
    }

    impl<'de, 'a> SeqAccess<'de> for SeqSliceAccess<'a> {
        type Error = de::value::Error;

        fn next_element_seed<T: DeserializeSeed<'de>>(
            &mut self,
            seed: T,
        ) -> Result<Option<T::Value>, Self::Error> {
            if let Some(item) = self.iter.next() {
                seed.deserialize(StringValueDeserializer { value: item })
                    .map(Some)
            } else {
                Ok(None)
            }
        }
    }
}
