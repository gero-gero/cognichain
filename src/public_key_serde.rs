// public_key_serde.rs

use secp256k1::PublicKey;
use serde::{Serialize, Deserialize};
use serde::de::{self, Deserializer};
use serde::ser::Serializer;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct SerializablePublicKey(pub PublicKey);

impl Serialize for SerializablePublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for SerializablePublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let pk = PublicKey::from_str(&s).map_err(de::Error::custom)?;
        Ok(SerializablePublicKey(pk))
    }
}
