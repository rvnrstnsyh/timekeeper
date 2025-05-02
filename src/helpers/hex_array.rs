use hex::{decode, encode};
use serde::{self, Deserialize, Deserializer, Serializer};

/// Serialize a 32-byte array to a hex string.
///
/// # Arguments
/// - `bytes`: The 32-byte array to serialize.
/// - `serializer`: The serializer to write to.
///
/// # Returns
/// A `Result` containing the error type used by the serializer.
pub fn serialize<T: Serializer>(bytes: &[u8; 32], serializer: T) -> Result<T::Ok, T::Error> {
    return serializer.serialize_str(&encode(bytes));
}

/// Deserialize a hex string into a 32-byte array.
///
/// # Arguments
/// - `deserializer`: The deserializer to read from.
///
/// # Returns
/// A `Result` containing the 32-byte array or an error if the input does not represent
/// a valid 32-byte array.
///
/// # Errors
/// Returns an error if the deserialized string cannot be decoded into bytes or if the
/// resulting byte vector is not exactly 32 bytes long.

pub fn deserialize<'a, T: Deserializer<'a>>(deserializer: T) -> Result<[u8; 32], T::Error> {
    let str: String = String::deserialize(deserializer)?;
    let bytes: Vec<u8> = decode(str).map_err(serde::de::Error::custom)?;

    if bytes.len() != 32 {
        return Err(serde::de::Error::custom(format!(
            "Expected 32 bytes, got {}",
            bytes.len()
        )));
    }

    let mut arr: [u8; 32] = [0u8; 32];
    arr.copy_from_slice(&bytes);

    return Ok(arr);
}
