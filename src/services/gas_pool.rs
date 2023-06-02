use eyre::Result;
use sui_types::base_types::{ObjectRef, SequenceNumber, ObjectID, ObjectDigest};

/// TODO: implement the logic of gas pool management
pub fn get_gas_object() -> Result<ObjectRef> {
  Ok((
    ObjectID::from_hex_literal("0x0")?,
    SequenceNumber::from_u64(0),
    ObjectDigest::random(),
  ))
}
