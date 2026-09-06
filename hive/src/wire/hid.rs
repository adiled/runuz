//! Minimal content-addressable identity — a self-contained stand-in
//! for hum's `ensemble::Hid`. A `Hid` is the sha256 of a pubkey,
//! tagged with a role prefix. The forager hive only needs the
//! `fbee_` role + wire hex forms.

use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Role prefixes locked in hum's v0: `humd_`, `wbee_`, `fbee_`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HidPrefix {
    Humd,
    Wbee,
    Fbee,
}

impl HidPrefix {
    pub fn as_str(self) -> &'static str {
        match self {
            HidPrefix::Humd => "humd",
            HidPrefix::Wbee => "wbee",
            HidPrefix::Fbee => "fbee",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hid {
    pub prefix: HidPrefix,
    pub bytes: [u8; 32],
}

impl Hid {
    /// Hash a pubkey to its content-addressable bytes; tag with the role.
    pub fn from_pubkey(prefix: HidPrefix, pubkey: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(pubkey);
        let digest = h.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&digest[..32]);
        Self { prefix, bytes }
    }

    /// Full wire form: `<prefix>_<64 hex>`.
    pub fn to_hex(&self) -> String {
        format!("{}_{}", self.prefix.as_str(), hex::encode(self.bytes))
    }

    /// Log-friendly short form: `<prefix>_<12 hex>`.
    pub fn short(&self) -> String {
        format!("{}_{}", self.prefix.as_str(), hex::encode(&self.bytes[..6]))
    }
}

impl std::fmt::Display for Hid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Serialize for Hid {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        ser.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Hid {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        let s = String::deserialize(de)?;
        // Only the full `prefix_hex` form is needed by the hive.
        let (p, h) = s.split_once('_')
            .ok_or_else(|| serde::de::Error::custom("bad hid form"))?;
        let prefix = match p {
            "humd" => HidPrefix::Humd,
            "wbee" => HidPrefix::Wbee,
            "fbee" => HidPrefix::Fbee,
            _ => return Err(serde::de::Error::custom("unknown hid prefix")),
        };
        let decoded = hex::decode(h).map_err(serde::de::Error::custom)?;
        if decoded.len() != 32 {
            return Err(serde::de::Error::custom("wrong hid length"));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(Hid { prefix, bytes })
    }
}
