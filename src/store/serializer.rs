use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::Write;

use multimap::MultiMap;
use serde::{
    Deserializer, Serializer,
    de::Visitor,
    ser::{self, SerializeSeq},
};

#[derive(Debug)]
pub enum LedgerError {
    IoError,
    Message(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::IoError => f.write_str("IO error"),
            LedgerError::Message(m) => f.write_fmt(format_args!("deserialization: {}", m)),
        }
    }
}

impl std::error::Error for LedgerError {}

impl From<std::io::Error> for LedgerError {
    fn from(_e: std::io::Error) -> Self {
        Self::IoError
    }
}

impl ser::Error for LedgerError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self::Message(msg.to_string())
    }
}

pub(crate) struct LedgerSerializer {
    w: File,
    first_item: bool,
    level: usize,
    equals_needed: bool,
}

impl LedgerSerializer {
    pub fn from_file(file: File) -> Self {
        Self {
            w: file,
            first_item: true,
            level: 0,
            equals_needed: false,
        }
    }

    fn serialize_number<T: Display>(&mut self, v: T) -> Result<(), LedgerError> {
        self.maybe_equals()?;
        self.w
            .write_all(format!("{}", v).as_bytes())
            .map_err(|_| LedgerError::IoError)?;
        Ok(())
    }

    fn first_comma(&mut self) -> Result<(), LedgerError> {
        if self.first_item {
            self.first_item = false;
            return Ok(());
        }

        self.w.write_all(",".as_bytes())?;
        Ok(())
    }

    // Inserts the equals symbol in case it was requested
    // Bools are serialized without it, but other values use it
    fn maybe_equals(&mut self) -> Result<(), LedgerError> {
        if self.equals_needed {
            self.equals_needed = false;
            self.w.write_all("=".as_bytes())?;
        }
        Ok(())
    }
}

impl ser::Serializer for &mut LedgerSerializer {
    type Ok = ();

    type Error = LedgerError;

    type SerializeSeq = Self;

    type SerializeTuple = Self;

    type SerializeTupleStruct = Self;

    type SerializeTupleVariant = Self;

    type SerializeMap = Self;

    type SerializeStruct = Self;

    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        if self.equals_needed {
            // key was added already, skip true
            self.equals_needed = false;
            return Ok(());
        }

        if v {
            self.w.write_all("1".as_bytes())?;
        } else {
            self.w.write_all("0".as_bytes())?;
        }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_number(v)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.maybe_equals()?;
        self.w.write_all("\"".as_bytes())?;
        self.w.write_all(
            v.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .as_bytes(),
        )?;
        self.w.write_all("\"".as_bytes())?;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.maybe_equals()?;
        self.w.write_all("b\"".as_bytes())?;
        for b in v {
            self.w.write_all(format!("{:02x}", b).as_bytes())?;
        }
        self.w.write_all("\"".as_bytes())?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        // NOP
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.maybe_equals()?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        // NOP
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.maybe_equals()?;
        self.w.write_all(name.as_bytes())?;
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.maybe_equals()?;
        self.w.write_all(name.as_bytes())?;
        self.w.write_all(b"::")?;
        self.w.write_all(variant.as_bytes())?;
        Ok(())
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.maybe_equals()?;
        self.w.write_all(name.as_bytes())?;
        self.w.write_all(b"{")?;
        self.first_item = true;
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.maybe_equals()?;
        self.w.write_all(name.as_bytes())?;
        self.w.write_all(b"::")?;
        self.w.write_all(variant.as_bytes())?;
        self.w.write_all(b"{")?;
        self.first_item = true;
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.maybe_equals()?;
        if self.level > 0 {
            self.w.write_all("[".as_bytes())?;
        }
        self.level += 1;
        self.first_item = true;
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.maybe_equals()?;
        if self.level > 0 {
            self.w.write_all("(".as_bytes())?;
        }
        self.level += 1;
        self.first_item = true;
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.maybe_equals()?;
        self.w.write_all(name.as_bytes())?;
        self.w.write_all("(".as_bytes())?;
        self.first_item = true;
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.maybe_equals()?;
        self.w.write_all(name.as_bytes())?;
        self.w.write_all(b"::")?;
        self.w.write_all(variant.as_bytes())?;
        self.w.write_all(b"{")?;
        self.first_item = true;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.maybe_equals()?;
        if self.level > 0 {
            self.w.write_all("{".as_bytes())?;
        }
        self.level += 1;
        self.first_item = true;
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.maybe_equals()?;
        if self.level > 0 {
            self.w.write_all(b"{")?;
        }
        self.level += 1;
        self.first_item = true;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.maybe_equals()?;
        self.w.write_all(name.as_bytes())?;
        self.w.write_all(b"::")?;
        self.w.write_all(variant.as_bytes())?;
        self.first_item = true;
        Ok(self)
    }
}

impl ser::SerializeStruct for &mut LedgerSerializer {
    type Ok = ();

    type Error = LedgerError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma()?;
        self.w.write_all(key.as_bytes())?;
        self.equals_needed = true;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.level = self.level.saturating_sub(1);
        if self.level > 0 {
            self.w.write_all("}".as_bytes())?;
        } else {
            self.w.write_all("\n".as_bytes())?;
        }
        Ok(())
    }
}

impl ser::SerializeTuple for &mut LedgerSerializer {
    type Ok = ();

    type Error = LedgerError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma()?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.level = self.level.saturating_sub(1);
        if self.level > 0 {
            self.w.write_all(b")")?;
        } else {
            self.w.write_all(b"\n")?;
        }
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut LedgerSerializer {
    type Ok = ();

    type Error = LedgerError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma()?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.w.write_all(b")")?;
        Ok(())
    }
}

impl ser::SerializeMap for &mut LedgerSerializer {
    type Ok = ();

    type Error = LedgerError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma()?;
        key.serialize(&mut **self)?;
        self.equals_needed = true;
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.level = self.level.saturating_sub(1);
        if self.level > 0 {
            self.w.write_all(b")")?;
        } else {
            self.w.write_all(b"\n")?;
        }
        Ok(())
    }
}

impl ser::SerializeSeq for &mut LedgerSerializer {
    type Ok = ();

    type Error = LedgerError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma()?;
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.level = self.level.saturating_sub(1);
        if self.level > 0 {
            self.w.write_all(b"]")?;
        } else {
            self.w.write_all(b"\n")?;
        }
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut LedgerSerializer {
    type Ok = ();

    type Error = LedgerError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma()?;
        self.w.write_all(key.as_bytes())?;
        self.equals_needed = true;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.w.write_all(b"}")?;
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut LedgerSerializer {
    type Ok = ();

    type Error = LedgerError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.w.write_all(b")")?;
        Ok(())
    }
}

pub(super) fn serialize_labels<S: Serializer>(
    v: &MultiMap<String, String>,
    s: S,
) -> Result<S::Ok, S::Error> {
    let mut list = s.serialize_seq(Some(v.len()))?;
    for (k, v) in v.flat_iter() {
        let mut el = HashMap::new();
        el.insert(k, v);
        list.serialize_element(&el)?;
    }
    list.end()
}

struct LabelsVisitor;

impl<'de> Visitor<'de> for LabelsVisitor {
    type Value = Vec<HashMap<String, String>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Expecting list of labels in the form `label: value`")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut vec: Self::Value = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(el) = seq.next_element()? {
            vec.push(el);
        }
        Ok(vec)
    }
}

pub(super) fn deserialize_labels<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<MultiMap<String, String>, D::Error> {
    d.deserialize_seq(LabelsVisitor {}).map(|v| {
        let mut res = MultiMap::new();
        for el in v {
            for (k, v) in el.iter() {
                res.insert(k.clone(), v.clone());
            }
        }
        res
    })
}
