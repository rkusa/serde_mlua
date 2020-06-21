// based on https://github.com/zrkn/rlua_serde/blob/master/src/de.rs

use crate::error::{Error, Result};
use mlua::{Table, TablePairs, TableSequence, Value};
use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::Deserialize;

pub struct Deserializer<'lua> {
    value: Value<'lua>,
}

impl<'de> Deserializer<'de> {
    pub fn from_value(value: Value<'de>) -> Self {
        Deserializer { value }
    }
}

pub fn from_value<'a, T>(value: Value<'a>) -> Result<T>
where
    T: Deserialize<'a>,
{
    let deserializer = Deserializer::from_value(value);
    let t = T::deserialize(deserializer)?;
    Ok(t)
}

impl<'lua, 'de> de::Deserializer<'de> for Deserializer<'lua> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Nil => visitor.visit_unit(),
            Value::Boolean(v) => visitor.visit_bool(v),
            Value::Integer(v) => visitor.visit_i64(v),
            Value::Number(v) => visitor.visit_f64(v),
            Value::String(v) => visitor.visit_str(v.to_str()?),
            Value::Table(v) => {
                // TODO: better way to distinguish between map and seq?
                if is_seq(v.clone())? {
                    let len = v.len()? as usize;
                    let mut deserializer = SeqDeserializer(v.sequence_values());
                    let seq = visitor.visit_seq(&mut deserializer)?;
                    let remaining = deserializer.0.count();
                    if remaining == 0 {
                        Ok(seq)
                    } else {
                        Err(serde::de::Error::invalid_length(
                            len,
                            &"fewer elements in array",
                        ))
                    }
                } else {
                    let len = v.len()? as usize;
                    let mut deserializer = MapDeserializer(v.pairs(), None);
                    let map = visitor.visit_map(&mut deserializer)?;
                    let remaining = deserializer.0.count();
                    if remaining == 0 {
                        Ok(map)
                    } else {
                        Err(serde::de::Error::invalid_length(
                            len,
                            &"fewer elements in array",
                        ))
                    }
                }
            }
            _ => Err(serde::de::Error::custom("invalid value type")),
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Value::Nil => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let (variant, value) = match self.value {
            Value::Table(value) => {
                let mut iter = value.pairs::<String, Value>();
                let (variant, value) = match iter.next() {
                    Some(v) => v?,
                    None => {
                        return Err(serde::de::Error::invalid_value(
                            serde::de::Unexpected::Map,
                            &"map with a single key",
                        ))
                    }
                };

                if iter.next().is_some() {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Map,
                        &"map with a single key",
                    ));
                }
                (variant, Some(value))
            }
            Value::String(variant) => (variant.to_str()?.to_owned(), None),
            _ => return Err(serde::de::Error::custom("bad enum value")),
        };

        visitor.visit_enum(EnumDeserializer { variant, value })
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Value::Table(v) => {
                let len = v.len()? as usize;
                let mut deserializer = SeqDeserializer(v.sequence_values());
                let seq = visitor.visit_seq(&mut deserializer)?;
                let remaining = deserializer.0.count();
                if remaining == 0 {
                    Ok(seq)
                } else {
                    Err(serde::de::Error::invalid_length(
                        len,
                        &"fewer elements in array",
                    ))
                }
            }
            _ => Err(serde::de::Error::custom("invalid value type")),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf unit unit_struct newtype_struct
        map struct identifier ignored_any
    }
}

struct SeqDeserializer<'lua>(TableSequence<'lua, Value<'lua>>);

impl<'lua, 'de> SeqAccess<'de> for SeqDeserializer<'lua> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.0.next() {
            Some(value) => seed.deserialize(Deserializer { value: value? }).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.0.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

struct MapDeserializer<'lua>(
    TablePairs<'lua, Value<'lua>, Value<'lua>>,
    Option<Value<'lua>>,
);

impl<'lua, 'de> MapAccess<'de> for MapDeserializer<'lua> {
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.0.next() {
            Some(item) => {
                let (key, value) = item?;
                self.1 = Some(value);
                let key_de = Deserializer { value: key };
                seed.deserialize(key_de).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        match self.1.take() {
            Some(value) => seed.deserialize(Deserializer { value }),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.0.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

struct EnumDeserializer<'lua> {
    variant: String,
    value: Option<Value<'lua>>,
}

impl<'lua, 'de> EnumAccess<'de> for EnumDeserializer<'lua> {
    type Error = Error;
    type Variant = VariantDeserializer<'lua>;

    fn variant_seed<T>(self, seed: T) -> Result<(T::Value, Self::Variant)>
    where
        T: DeserializeSeed<'de>,
    {
        let variant = self.variant.into_deserializer();
        let variant_access = VariantDeserializer { value: self.value };
        seed.deserialize(variant).map(|v| (v, variant_access))
    }
}

struct VariantDeserializer<'lua> {
    value: Option<Value<'lua>>,
}

impl<'lua, 'de> VariantAccess<'de> for VariantDeserializer<'lua> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.value {
            Some(_) => Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::NewtypeVariant,
                &"unit variant",
            )),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => seed.deserialize(Deserializer { value }),
            None => Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(value) => serde::Deserializer::deserialize_seq(Deserializer { value }, visitor),
            None => Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(value) => serde::Deserializer::deserialize_map(Deserializer { value }, visitor),
            None => Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

fn is_seq(val: Table) -> Result<bool> {
    let mut next_key = 1;
    for pair in val.pairs::<Value, Value>() {
        let (key, _) = pair?;
        if key != Value::Integer(next_key) {
            return Ok(false);
        }
        next_key += 1;
    }

    Ok(true)
}

#[cfg(test)]
mod test {
    use super::from_value;
    use mlua::Lua;
    use serde::Deserialize;

    #[test]
    fn enum_variant_with_empty_seq() {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(tag = "id")]
        enum Variant {
            Combo { params: ComboParams },
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct ComboParams {
            values: Vec<i64>,
        }

        let expected = Variant::Combo {
            params: ComboParams { values: vec![1] },
        };

        let lua = Lua::new();
        let result = lua
            .load(
                r#"
                return {
                    id = "Combo",
                    params = {
                        values = {1}
                    }
                }
            "#,
            )
            .eval()
            .unwrap();
        let result = from_value(result).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn enum_variant_with_seq() {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(tag = "id")]
        enum Variant {
            Combo { params: ComboParams },
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct ComboParams {
            values: Vec<i64>,
        }

        let expected = Variant::Combo {
            params: ComboParams { values: vec![] },
        };

        let lua = Lua::new();
        let result = lua
            .load(
                r#"
                return {
                    id = "Combo",
                    params = {
                        values = {}
                    }
                }
            "#,
            )
            .eval()
            .unwrap();
        let result = from_value(result).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn untagged_enum_with_empty_seq() {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(untagged)]
        enum Variant {
            Seq { seq: Vec<u8> },
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct ComboParams {
            values: Vec<i64>,
        }

        let expected = Variant::Seq { seq: Vec::new() };

        let lua = Lua::new();
        let result = lua
            .load(
                r#"
                return {
                    seq = {}
                }
            "#,
            )
            .eval()
            .unwrap();
        let result = from_value(result).unwrap();
        assert_eq!(expected, result);
    }
}
