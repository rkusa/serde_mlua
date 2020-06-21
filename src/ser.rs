// based on https://github.com/wez/wezterm/tree/master/src/scripting/serde_lua

use crate::error::{Error, Result};
use mlua::{Lua, Table, ToLua, Value};
use serde::{ser, Serialize};

pub fn to_value<'lua, T>(lua: &'lua Lua, input: T) -> Result<Value<'lua>>
where
    T: Serialize,
{
    input.serialize(Serializer { lua })
}

pub struct Serializer<'lua> {
    lua: &'lua Lua,
}

pub struct SeqSerializer<'lua> {
    lua: &'lua Lua,
    table: Table<'lua>,
    index: usize,
}

pub struct TupleVariantSerializer<'lua> {
    lua: &'lua Lua,
    table: Table<'lua>,
    index: usize,
    name: String,
}

pub struct MapSerializer<'lua> {
    lua: &'lua Lua,
    table: Table<'lua>,
    key: Option<Value<'lua>>,
}

pub struct StructVariantSerializer<'lua> {
    lua: &'lua Lua,
    table: Table<'lua>,
    name: String,
}

impl<'lua> ser::Serializer for Serializer<'lua> {
    type Ok = Value<'lua>;
    type Error = Error;

    type SerializeSeq = SeqSerializer<'lua>;
    type SerializeTuple = SeqSerializer<'lua>;
    type SerializeTupleStruct = SeqSerializer<'lua>;
    type SerializeTupleVariant = TupleVariantSerializer<'lua>;
    type SerializeMap = MapSerializer<'lua>;
    type SerializeStruct = MapSerializer<'lua>;
    type SerializeStructVariant = StructVariantSerializer<'lua>;

    // primitive types

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        Ok(v.to_string().to_lua(self.lua)?)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        Ok(v.to_lua(self.lua)?)
    }

    // Serialize a byte array as an array of bytes. Could also use a base64
    // string here. Binary formats will typically represent byte arrays more
    // compactly.
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        use serde::ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        Ok(Value::Nil)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        Ok(Value::Nil)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: ?Sized + Serialize,
    {
        let value = value.serialize(Serializer { lua: self.lua })?;
        let table = self.lua.create_table()?;
        table.set(variant, value)?;
        Ok(Value::Table(table))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        let table = self.lua.create_table()?;
        Ok(SeqSerializer {
            lua: self.lua,
            table,
            index: 1,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        let table = self.lua.create_table()?;
        Ok(TupleVariantSerializer {
            lua: self.lua,
            table,
            index: 1,
            name: variant.to_string(),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        let table = self.lua.create_table()?;
        Ok(MapSerializer {
            lua: self.lua,
            table,
            key: None,
        })
    }
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        let table = self.lua.create_table()?;
        Ok(StructVariantSerializer {
            lua: self.lua,
            table,
            name: variant.to_owned(),
        })
    }
}

impl<'lua> ser::SerializeSeq for SeqSerializer<'lua> {
    type Ok = Value<'lua>;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let value = value.serialize(Serializer { lua: self.lua })?;
        self.table.set(self.index, value)?;
        self.index += 1;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Value::Table(self.table))
    }
}

impl<'lua> ser::SerializeTuple for SeqSerializer<'lua> {
    type Ok = Value<'lua>;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl<'lua> ser::SerializeTupleStruct for SeqSerializer<'lua> {
    type Ok = Value<'lua>;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl<'lua> ser::SerializeTupleVariant for TupleVariantSerializer<'lua> {
    type Ok = Value<'lua>;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let value = value.serialize(Serializer { lua: self.lua })?;
        self.table.set(self.index, value)?;
        self.index += 1;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        let map = self.lua.create_table()?;
        map.set(self.name, self.table)?;
        Ok(Value::Table(map))
    }
}

impl<'lua> ser::SerializeMap for MapSerializer<'lua> {
    type Ok = Value<'lua>;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = key.serialize(Serializer { lua: self.lua })?;
        self.key.replace(key);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let value = value.serialize(Serializer { lua: self.lua })?;
        let key = self
            .key
            .take()
            .expect("serialize_key must be called before serialize_value");
        self.table.set(key, value)?;
        Ok(())
    }

    fn serialize_entry<K: Serialize + ?Sized, V: Serialize + ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<()> {
        let key = key.serialize(Serializer { lua: self.lua })?;
        let value = value.serialize(Serializer { lua: self.lua })?;
        self.table.set(key, value)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Value::Table(self.table))
    }
}

impl<'lua> ser::SerializeStruct for MapSerializer<'lua> {
    type Ok = Value<'lua>;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        serde::ser::SerializeMap::end(self)
    }
}

impl<'lua> ser::SerializeStructVariant for StructVariantSerializer<'lua> {
    type Ok = Value<'lua>;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = key.serialize(Serializer { lua: self.lua })?;
        let value = value.serialize(Serializer { lua: self.lua })?;
        self.table.set(key, value)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        let map = self.lua.create_table()?;
        map.set(self.name, self.table)?;
        Ok(Value::Table(map))
    }
}
