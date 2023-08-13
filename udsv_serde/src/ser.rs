use serde::{ser, Serialize};

use crate::err::{Error, Result};

pub struct Serializer {
    output: String,
    in_seq: bool,
    in_map: bool,
}

pub fn record_to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut serializer = Serializer {
        output: String::new(),
        in_seq: false,
        in_map: false,
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

// TODO: struct Serializer owns a impl Write not a String see https://github.com/samscott89/serde_qs/blob/main/src/ser.rs
// pub fn record_to_writer<T,W>(input: &T, writer: &mut W) -> Result<()>
// where
// T: Serialize,
// W: Write,
// {
//     input.serialize(&mut Serializer::new(writer))
// }

impl Serializer {
    //TODO: do we want to escape tabs, returns?
    fn escape_str(&self, v: &str) -> String {
        let mut v = v.to_string();
        // We have to replace the backslashes first, otherwise we will double escape the other characters.
        v = v.replace('\\', r"\\");
        v = v.replace(':', r"\:");
        v = v.replace('\n', r"\n");

        if self.in_seq || self.in_map {
            v = v.replace(',', r"\,");
        }

        if self.in_map {
            v = v.replace('=', r"\=");
        }

        v
    }
}

//TODO: do we need atomics here?
pub struct UDSVSeq<'a>(&'a mut Serializer, i32);
pub struct UDSVMap<'a>(&'a mut Serializer, i32);
pub struct UDSVStuct<'a>(&'a mut Serializer, i32);
pub struct UDSVTuple<'a>(&'a mut Serializer, i32);

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = UDSVSeq<'a>;
    type SerializeTuple = UDSVTuple<'a>;
    type SerializeTupleStruct = UDSVTuple<'a>;
    type SerializeTupleVariant = UDSVTuple<'a>;
    type SerializeMap = UDSVMap<'a>;
    type SerializeStruct = UDSVStuct<'a>;
    type SerializeStructVariant = UDSVStuct<'a>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output += if v { "true" } else { "false" };
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    // Not particularly efficient but this is example code anyway. A more
    // performant approach would be to use the `itoa` crate.
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.output += &self.escape_str(v);
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::BytesUnsupported)
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
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
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        variant.serialize(&mut *self)?;
        self.output += ":";
        value.serialize(&mut *self)?;
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.in_seq = true;
        Ok(UDSVSeq(self, 0))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        self.in_seq = true;
        Ok(UDSVTuple(self, 0))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        variant.serialize(&mut *self)?;
        self.output += ":";
        self.in_seq = true;
        Ok(UDSVTuple(self, 0))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.in_map = true;
        Ok(UDSVMap(self, 0))
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(UDSVStuct(self, 0))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        variant.serialize(&mut *self)?;
        self.output += ":";
        Ok(UDSVStuct(self, 0))
    }
}

impl<'a> ser::SerializeSeq for UDSVSeq<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.1 > 0 {
            self.0.output += ",";
        }
        self.1 += 1;
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        self.0.in_seq = false;
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for UDSVTuple<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.1 > 0 {
            self.0.output += ",";
        }
        self.1 += 1;
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        self.0.in_seq = false;
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for UDSVTuple<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.1 > 0 {
            self.0.output += ",";
        }
        self.1 += 1;
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        self.0.in_seq = false;
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for UDSVTuple<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.1 > 0 {
            self.0.output += ",";
        }
        self.1 += 1;
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        self.0.in_seq = false;
        Ok(())
    }
}

impl<'a> ser::SerializeMap for UDSVMap<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.1 > 0 {
            self.0.output += ",";
        }
        self.1 += 1;
        key.serialize(&mut *self.0)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.0.output += "=";
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        self.0.in_map = false;
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for UDSVStuct<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.1 > 0 {
            self.0.output += ":";
        }
        self.1 += 1;
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for UDSVStuct<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.1 > 0 {
            self.0.output += ":";
        }
        self.1 += 1;
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {

    use crate::record_to_string;
    use serde::Serialize;

    #[test]
    fn test_escaped_str() {
        let v = "a:b";
        let expected = r#"a\:b"#;
        assert_eq!(record_to_string(&v).unwrap(), expected);

        let v = r"a\b";
        let expected = r"a\\b";
        assert_eq!(record_to_string(&v).unwrap(), expected);

        let v = r"a
b";
        let expected = r"a\nb";
        assert_eq!(record_to_string(&v).unwrap(), expected);

        // commas and equals should not be escaped in a plain string
        let v = r"a,b=c";
        let expected = r"a,b=c";
        assert_eq!(record_to_string(&v).unwrap(), expected);
    }

    #[test]
    fn test_seq() {
        let v = vec!["a", "b"];
        let expected = r#"a,b"#;
        assert_eq!(record_to_string(&v).unwrap(), expected);

        // escaped comma
        let v = vec!["a,c", "b"];
        let expected = r#"a\,c,b"#;
        assert_eq!(record_to_string(&v).unwrap(), expected);

        // equals should not be escaped in a sequence
        let v = vec!["a=c", "b"];
        let expected = r#"a=c,b"#;
        assert_eq!(record_to_string(&v).unwrap(), expected);
    }

    #[test]
    fn test_tuple() {
        let v = ("a", "b");
        let expected = r#"a,b"#;
        assert_eq!(record_to_string(&v).unwrap(), expected);
    }

    #[test]
    fn test_option() {
        let v = Some("a");
        let expected = "a";
        assert_eq!(record_to_string(&v).unwrap(), expected);

        let v = vec![Some("a"), None, Some("b")];
        let expected = "a,,b";
        assert_eq!(record_to_string(&v).unwrap(), expected);
    }

    #[test]
    fn test_map() {
        let mut map = std::collections::HashMap::new();
        map.insert("a", "b");
        map.insert("c", "d");

        let expected1 = r#"a=b,c=d"#;
        let expected2 = r#"c=d,a=b"#;
        let output = record_to_string(&map).unwrap();
        assert!(
            output == expected1 || output == expected2,
            "output: {}",
            output
        );

        // escaped comma
        let mut map = std::collections::HashMap::new();
        map.insert("a", "b,c");
        let expected = r#"a=b\,c"#;
        assert_eq!(record_to_string(&map).unwrap(), expected);

        // escaped equals
        let mut map = std::collections::HashMap::new();
        map.insert("a", "b=c");
        let expected = r#"a=b\=c"#;
        assert_eq!(record_to_string(&map).unwrap(), expected);
    }

    #[test]
    fn test_struct() {
        #[derive(Serialize)]
        struct Test {
            int: u32,
            seq: Vec<&'static str>,
            tup: (&'static str, &'static str),
            txt: &'static str,
            opt1: Option<&'static str>,
            opt2: Option<&'static str>,
        }

        let test = Test {
            int: 1,
            seq: vec!["a", "b"],
            tup: ("c", "d"),
            txt: "hello",
            opt1: None,
            opt2: Some("world"),
        };

        let expected = r#"1:a,b:c,d:hello::world"#;
        assert_eq!(record_to_string(&test).unwrap(), expected);
    }

    #[test]
    fn test_enum() {
        #[derive(Serialize)]
        enum E {
            Unit,
            Newtype(u32),
            Tuple(u32, u32),
            Struct { a: u32 },
        }

        let u = E::Unit;
        let expected = r#"Unit"#;
        assert_eq!(record_to_string(&u).unwrap(), expected);

        let n = E::Newtype(1);
        let expected = r#"Newtype:1"#;
        assert_eq!(record_to_string(&n).unwrap(), expected);

        let t = E::Tuple(1, 2);
        let expected = r#"Tuple:1,2"#;
        assert_eq!(record_to_string(&t).unwrap(), expected);

        let s = E::Struct { a: 1 };
        let expected = r#"Struct:1"#;
        assert_eq!(record_to_string(&s).unwrap(), expected);
    }
}
