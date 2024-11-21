use std::{
    any::Any,
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{self, Write},
    ops::DerefMut,
};

use multimap::MultiMap;
use serde::{
    de::Visitor,
    ser::{self, SerializeSeq},
    Deserializer, Serializer,
};

pub(crate) struct LedgerSerializer {
    w: File,
    first_item: bool,
    level: usize,
    equals_needed: bool,
}

impl LedgerSerializer {
    pub fn close(self) -> File {
        self.w
    }

    pub fn from_file(file: File) -> Self {
        Self {
            w: file,
            first_item: true,
            level: 0,
            equals_needed: false,
        }
    }

    fn serialize_number<T: Display>(&mut self, v: T) -> Result<(), std::fmt::Error> {
        self.maybe_equals();
        self.w.write(format!("{}", v).as_bytes());
        Ok(())
    }

    fn first_comma(&mut self) {
        if self.first_item {
            self.first_item = false;
            return;
        }

        self.w.write(",".as_bytes());
    }

    fn maybe_equals(&mut self) {
        if self.equals_needed {
            self.equals_needed = false;
            self.w.write("=".as_bytes());
        }
    }
}

impl<'a> ser::Serializer for &'a mut LedgerSerializer {
    type Ok = ();

    type Error = std::fmt::Error;

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
            self.w.write("1".as_bytes());
        } else {
            self.w.write("0".as_bytes());
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
        self.maybe_equals();
        self.w.write("\"".as_bytes());
        self.w.write(
            v.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .as_bytes(),
        );
        self.w.write("\"".as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.maybe_equals();
        self.w.write("b\"".as_bytes());
        for b in v {
            self.w.write(format!("{:02x}", b).as_bytes());
        }
        self.w.write("\"".as_bytes());
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
        self.maybe_equals();
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        // NOP
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.maybe_equals();
        self.w.write(name.as_bytes());
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.maybe_equals();
        self.w.write(name.as_bytes());
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
        self.maybe_equals();
        self.w.write(name.as_bytes());
        self.w.write("={".as_bytes());
        self.first_item = true;
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.maybe_equals();
        self.w.write(name.as_bytes());
        self.w.write("={".as_bytes());
        self.first_item = true;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.maybe_equals();
        if self.level > 0 {
            self.w.write("[".as_bytes());
        }
        self.level += 1;
        self.first_item = true;
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.maybe_equals();
        if self.level > 0 {
            self.w.write("(".as_bytes());
        }
        self.level += 1;
        self.first_item = true;
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.maybe_equals();
        self.w.write(name.as_bytes());
        self.w.write("=(".as_bytes());
        self.first_item = true;
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.maybe_equals();
        self.w.write(name.as_bytes());
        self.w.write("=(".as_bytes());
        self.first_item = true;
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.maybe_equals();
        if self.level > 0 {
            self.w.write("{".as_bytes());
        }
        self.level += 1;
        self.first_item = true;
        Ok(self)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.maybe_equals();
        if self.level > 0 {
            self.w.write("{".as_bytes());
        }
        self.level += 1;
        self.first_item = true;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.maybe_equals();
        self.w.write(name.as_bytes());
        self.w.write("={".as_bytes());
        self.first_item = true;
        Ok(self)
    }
}

impl<'a> ser::SerializeStruct for &'a mut LedgerSerializer {
    type Ok = ();

    type Error = std::fmt::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma();
        self.w.write(key.as_bytes());
        self.equals_needed = true;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.level = self.level.saturating_sub(1);
        if self.level > 0 {
            self.w.write("}".as_bytes());
        } else {
            self.w.write("\n".as_bytes());
        }
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut LedgerSerializer {
    type Ok = ();

    type Error = std::fmt::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma();
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.level = self.level.saturating_sub(1);
        if self.level > 0 {
            self.w.write(")".as_bytes());
        } else {
            self.w.write("\n".as_bytes());
        }
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut LedgerSerializer {
    type Ok = ();

    type Error = std::fmt::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma();
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.w.write(")".as_bytes());
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut LedgerSerializer {
    type Ok = ();

    type Error = std::fmt::Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma();
        key.serialize(&mut **self);
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
            self.w.write(")".as_bytes());
        } else {
            self.w.write("\n".as_bytes());
        }
        Ok(())
    }
}

impl<'a> ser::SerializeSeq for &'a mut LedgerSerializer {
    type Ok = ();

    type Error = std::fmt::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma();
        value.serialize(&mut **self);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.level = self.level.saturating_sub(1);
        if self.level > 0 {
            self.w.write("]".as_bytes());
        } else {
            self.w.write("\n".as_bytes());
        }
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut LedgerSerializer {
    type Ok = ();

    type Error = std::fmt::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.first_comma();
        self.w.write(key.as_bytes());
        self.equals_needed = true;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.w.write("}".as_bytes());
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut LedgerSerializer {
    type Ok = ();

    type Error = std::fmt::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.w.write(")".as_bytes());
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
