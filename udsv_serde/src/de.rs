use std::ops::{AddAssign, MulAssign, Neg};

use serde::de::{
    self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::Deserialize;

use crate::err::{Error, Result};

pub struct Deserializer<'de> {
    input: &'de str,
    in_seq: bool,
    in_map: bool,
}

impl<'de> Deserializer<'de> {
    fn from_str(input: &'de str) -> Self {
        Deserializer {
            input,
            in_seq: false,
            in_map: false,
        }
    }
}

pub fn record_from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

// SERDE IS NOT A PARSING LIBRARY. This impl block defines a few basic parsing
// functions from scratch. More complicated formats may wish to use a dedicated
// parsing library to help implement their Serde deserializer.
impl<'de> Deserializer<'de> {
    fn shift_input_forward(&mut self, len: usize) {
        self.input = &self.input[len..];
    }

    //TODO: we can probably do this better by creating a modified version of `get_next_nonescaped_char`.
    fn get_next_delimiter_idx(&self) -> Option<usize> {
        let mut idx = self.get_next_nonescaped_char(':');
        if self.in_seq || self.in_map {
            let comma_idx = self.get_next_nonescaped_char(',');
            // Choose the smaller of the two
            idx = match (idx, comma_idx) {
                (Some(idx), Some(comma_idx)) => Some(std::cmp::min(idx, comma_idx)),
                (Some(idx), None) => Some(idx),
                (None, Some(comma_idx)) => Some(comma_idx),
                (None, None) => None,
            };
        }

        if self.in_map {
            let equals_idx = self.get_next_nonescaped_char('=');
            // Choose the smaller of the two
            idx = match (idx, equals_idx) {
                (Some(idx), Some(equals_idx)) => Some(std::cmp::min(idx, equals_idx)),
                (Some(idx), None) => Some(idx),
                (None, Some(equals_idx)) => Some(equals_idx),
                (None, None) => None,
            };
        }

        idx
    }

    fn get_next_nonescaped_char(&self, ch: char) -> Option<usize> {
        self.input
            .match_indices(|c| c == ch)
            .map(|(idx, _)| idx)
            .filter(|idx| {
                idx > &0
                    && self
                        .input
                        .chars()
                        .nth(idx - 1)
                        .map(|ch| ch != '\\')
                        .unwrap_or(false)
            })
            .next()
    }

    // Look at the first character in the input without consuming it.
    fn peek_char(&mut self) -> Result<char> {
        self.input.chars().next().ok_or(Error::Eof)
    }

    // Consume the first character in the input.
    fn next_char(&mut self) -> Result<char> {
        let ch = self.peek_char()?;
        self.shift_input_forward(ch.len_utf8());
        Ok(ch)
    }

    // Parse the identifier `true` or `false`.
    fn parse_bool(&mut self) -> Result<bool> {
        if self.input.starts_with("true") {
            self.shift_input_forward("true".len());
            Ok(true)
        } else if self.input.starts_with("false") {
            self.shift_input_forward("false".len());
            Ok(false)
        } else {
            Err(Error::ExpectedBoolean)
        }
    }

    // The various arithmetic operations can overflow and
    // panic or return bogus data.
    fn parse_unsigned<T>(&mut self) -> Result<T>
    where
        T: AddAssign<T> + MulAssign<T> + From<u8>,
    {
        let mut int = match self.next_char()? {
            ch @ '0'..='9' => T::from(ch as u8 - b'0'),
            _ => {
                return Err(Error::ExpectedInteger);
            }
        };
        loop {
            match self.input.chars().next() {
                Some(ch @ '0'..='9') => {
                    self.shift_input_forward(1);
                    int *= T::from(10);
                    int += T::from(ch as u8 - b'0');
                }
                _ => {
                    return Ok(int);
                }
            }
        }
    }

    fn parse_signed<T>(&mut self) -> Result<T>
    where
        T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8>,
    {
        // Optional minus sign, delegate to `parse_unsigned`, negate if negative.
        todo!()
    }

    // TODO: how do we have it so it can return a &str - use Cow?
    fn parse_string(&mut self) -> Result<String> {
        let len = match self.get_next_delimiter_idx() {
            Some(idx) => idx,
            None => self.input.len(),
        };

        let s = &self.input[..len];
        self.shift_input_forward(len);

        // Replace escape characters used in UDSV format
        let mut s = s.replace(r#"\:"#, ":");
        s = s.replace(r#"\,"#, ",");
        s = s.replace(r#"\="#, "=");
        s = s.replace(r#"\\"#, r#"\"#);

        // Remove an escaped newline
        s = s.replace("\\\n", "");

        // Replace escaped printables
        s = s.replace(r#"\n"#, "\n");
        s = s.replace(r#"\r"#, "\r");
        s = s.replace(r#"\t"#, "\t");

        Ok(s)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!("UDSV is not a self-describing format")
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_signed()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_signed()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_signed()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_signed()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_unsigned()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_unsigned()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_unsigned()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_unsigned()?)
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Parse a string, check that it is one character.
        let ch = self.parse_string()?;
        if ch.len() == 1 {
            visitor.visit_char(ch.chars().next().unwrap())
        } else {
            Err(Error::ExpectedChar)
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.parse_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::BytesUnsupported)
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::BytesUnsupported)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.is_empty() {
            return visitor.visit_none();
        }

        let next_char = self.peek_char()?;
        match (next_char, self.in_seq, self.in_map) {
            (':', false, false) => visitor.visit_none(), // Not in a sequence or map
            (':' | ',', true, false) => visitor.visit_none(), // In a sequence but not in a map
            (':' | ',' | '=', _, true) => visitor.visit_none(), // In a map and possibly in a sequence
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.is_empty() {
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedEmpty)
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.in_seq = true;
        let v = visitor.visit_seq(DelimiterSeparated::new(self, ','));
        self.in_seq = false;
        v
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.in_seq = true;
        let v = visitor.visit_seq(DelimiterSeparated::new(self, ','));
        self.in_seq = false;
        v
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.in_seq = true;
        let v = visitor.visit_seq(DelimiterSeparated::new(self, ','));
        self.in_seq = false;
        v
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.in_map = true;
        let v = visitor.visit_map(DelimiterSeparated::new(self, ','));
        self.in_map = false;
        v
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Do not set `in_seq` here as that is used to stop at commas.
        visitor.visit_seq(DelimiterSeparated::new(self, ':'))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(Enum::new(self))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct DelimiterSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
    delim: char,
}

impl<'a, 'de> DelimiterSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, delim: char) -> Self {
        DelimiterSeparated {
            de,
            first: true,
            delim,
        }
    }
}

impl<'de, 'a> SeqAccess<'de> for DelimiterSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.de.input.is_empty() || (self.delim != ':' && self.de.peek_char()? == ':') {
            return Ok(None);
        }

        if !self.first && self.de.next_char()? != self.delim {
            //TODO: this is not the right error if delim is not a comma
            return Err(Error::ExpectedArrayComma);
        }
        self.first = false;

        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'de, 'a> MapAccess<'de> for DelimiterSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.input.is_empty() || self.de.peek_char()? == ':' {
            return Ok(None);
        }

        if !self.first && self.de.next_char()? != ',' {
            return Err(Error::ExpectedMapComma);
        }
        self.first = false;

        let len = match self.de.get_next_nonescaped_char('=') {
            Some(idx) => idx,
            None => Err(Error::ExpectedMapEquals)?,
        };

        // validate no comma before equals
        let comma_idx = self.de.get_next_nonescaped_char(',');
        if comma_idx.is_some() && comma_idx.unwrap() < len {
            return Err(Error::ExpectedMapEquals);
        }

        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        // Make sure we have parsed until the equals.
        if self.de.next_char()? != '=' {
            return Err(Error::ExpectedMapEquals);
        }

        let len = match self.de.get_next_nonescaped_char(',') {
            Some(idx) => idx,
            None => self.de.input.len(),
        };

        // validate no equals before comma
        let equals_idx = self.de.get_next_nonescaped_char('=');
        if equals_idx.is_some() && equals_idx.unwrap() < len {
            return Err(Error::ExpectedMapComma);
        }

        seed.deserialize(&mut *self.de)
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self.de)?;

        if self.de.peek_char().map(|ch| ch == ':').unwrap_or(false) {
            self.de.shift_input_forward(1);
        }

        Ok((val, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.de, len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        //TODO: is the empty string correct here? probaby not
        //seems to work though
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {

    use std::collections::HashMap;

    use crate::record_from_str;
    use serde::Deserialize;

    #[test]
    fn test_unsigned() {
        let v = "1";
        assert_eq!(1, record_from_str::<u8>(v).unwrap());
        assert_eq!(1, record_from_str::<u16>(v).unwrap());
        assert_eq!(1, record_from_str::<u32>(v).unwrap());
        assert_eq!(1, record_from_str::<u64>(v).unwrap());

        let v = "11534";
        assert_eq!(11534, record_from_str::<u16>(v).unwrap());
        // assert!(from_str::<u8>(v).is_err());
    }

    #[test]
    fn test_escaped_str() {
        let v = r#"a\:b"#;
        let expected = "a:b";
        assert_eq!(expected, record_from_str::<String>(v).unwrap());

        // Test escaped non-printable characters
        let v = r#"a\nb\tc"#;
        let expected = "a\nb\tc";
        assert_eq!(expected, record_from_str::<String>(v).unwrap());

        // Test custom escapes
        let v = r#"a\:b\,c\=d\
e"#;
        let expected = "a:b,c=de";
        assert_eq!(expected, record_from_str::<String>(v).unwrap());
    }

    #[test]
    fn test_seq() {
        let v = "a,b";
        let expected = vec!["a", "b"];
        assert_eq!(expected, record_from_str::<Vec<String>>(v).unwrap());

        let v = "a,b&],\nc";
        let expected = vec!["a", "b&]", "\nc"];
        assert_eq!(expected, record_from_str::<Vec<String>>(v).unwrap());
    }

    #[test]
    fn test_tuple() {
        let v = "a,b";
        let expected = ("a".to_owned(), "b".to_owned());
        assert_eq!(expected, record_from_str(v).unwrap());

        // Test escaped comma
        let v = r#"a,b\,c"#;
        let expected = ("a".to_owned(), "b,c".to_owned());
        assert_eq!(expected, record_from_str(v).unwrap());
    }

    #[test]
    fn test_trailing_chars() {
        let v = "a::b";
        assert!(record_from_str::<Option<String>>(v).is_err());
        assert!(record_from_str::<Vec<Option<String>>>(v).is_err());
    }

    #[test]
    fn test_option() {
        let v = "a";
        let expected = Some("a".to_owned());
        assert_eq!(expected, record_from_str(v).unwrap());

        let v = "a,,b";
        let expected = vec![Some("a".to_owned()), None, Some("b".to_owned())];
        assert_eq!(expected, record_from_str::<Vec<Option<String>>>(v).unwrap());
    }

    #[test]
    fn test_map() {
        let v = "a=b,c=d";
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_owned(), "b".to_owned());
        map.insert("c".to_owned(), "d".to_owned());
        assert_eq!(map, record_from_str(v).unwrap());

        // Test escaped comma
        let v = r#"a=b\,x,c=d"#;
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_owned(), "b,x".to_owned());
        map.insert("c".to_owned(), "d".to_owned());
        assert_eq!(map, record_from_str(v).unwrap());

        // Test escaped equals
        let v = r#"a=b\,x,c=d\=e"#;
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_owned(), "b,x".to_owned());
        map.insert("c".to_owned(), "d=e".to_owned());
        assert_eq!(map, record_from_str(v).unwrap());

        // An ill formed map errors - comma before equals
        let v = r#"a=b,cx,y=d"#;
        assert!(record_from_str::<HashMap<String, String>>(v).is_err());

        // An ill formed map errors - equal before comma
        let v = r#"a=b=x,c=d"#;
        assert!(record_from_str::<HashMap<String, String>>(v).is_err());
    }

    #[test]
    fn test_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            int: u32,
            seq: Vec<String>,
        }

        let j = r#"1:a"#;
        let expected = Test {
            int: 1,
            seq: vec!["a".to_owned()],
        };
        assert_eq!(expected, record_from_str(j).unwrap());

        let j = r#"1:a,b"#;
        let expected = Test {
            int: 1,
            seq: vec!["a".to_owned(), "b".to_owned()],
        };
        assert_eq!(expected, record_from_str(j).unwrap());

        let j = r#"1:a,b,c"#;
        let expected = Test {
            int: 1,
            seq: vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
        };
        assert_eq!(expected, record_from_str(j).unwrap());
    }

    #[test]
    fn test_enum() {
        #[derive(Deserialize, PartialEq, Debug)]
        enum E {
            Unit,
            Newtype(u32),
            Tuple(u32, u32),
            Struct { a: u32 },
            Opt(Option<u32>),
        }

        let j = "Unit";
        let expected = E::Unit;
        assert_eq!(expected, record_from_str(j).unwrap());

        let j = "Newtype:1";
        let expected = E::Newtype(1);
        assert_eq!(expected, record_from_str(j).unwrap());

        let j = "Tuple:1,2";
        let expected = E::Tuple(1, 2);
        assert_eq!(expected, record_from_str(j).unwrap());

        let j = "Struct:1";
        let expected = E::Struct { a: 1 };
        assert_eq!(expected, record_from_str(j).unwrap());

        let j = "Opt:1";
        let expected = E::Opt(Some(1));
        assert_eq!(expected, record_from_str(j).unwrap());
        let j = "Opt:";
        let expected = E::Opt(None);
        assert_eq!(expected, record_from_str(j).unwrap());
    }
}
