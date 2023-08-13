mod de;
mod err;
mod ser;

pub use de::{record_from_str, Deserializer};
pub use err::{Error, Result};
pub use ser::{record_to_string, Serializer};
