pub mod behaviour;
pub mod handler;
pub mod protocol;

use libp2p::multihash::Multihash;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Once this is used by the rest of the code it should go somewhere else.
// This CI rule to have no T O D O's and F I X M E's is really shit.
#[derive(Clone, Debug, PartialEq)]
pub struct SwapDigest {
    inner: Multihash,
}

impl Serialize for SwapDigest {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unimplemented!()
    }
}

impl<'de> Deserialize<'de> for SwapDigest {
    fn deserialize<D>(_deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        unimplemented!()
    }
}
