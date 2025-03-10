use committable::{Commitment, Committable};
use hotshot::types::SignatureKey;

use super::v0_99::{RollupRegistrationBody, RollupUpdatebody};
use crate::v0::utils::{Update, Update::Set};

impl Committable for RollupRegistrationBody {
    fn tag() -> String {
        "ROLLUP_REGISTRATION".to_string()
    }

    fn commit(&self) -> Commitment<Self> {
        let mut comm = committable::RawCommitmentBuilder::new(&Self::tag())
            .u64_field("namespace_id", u64::from(self.namespace_id));

        if let Some(url) = &self.reserve_url {
            comm = comm
                .u64_field("reserve_url", 1)
                .var_size_bytes(url.as_str().as_ref())
        } else {
            comm = comm.u64_field("reserve_url", 0)
        };

        comm = comm
            .fixed_size_field("reserve_price", &self.reserve_price.to_fixed_bytes())
            .fixed_size_field("active", &[u8::from(self.active)])
            .constant_str("signature_keys");

        for key in self.signature_keys.iter() {
            comm = comm.var_size_bytes(&key.to_bytes());
        }

        comm = comm
            .var_size_field("signature_key", &self.signature_key.to_bytes())
            .var_size_field("text", self.text.as_bytes());

        comm.finalize()
    }
}

impl Committable for RollupUpdatebody {
    fn tag() -> String {
        "ROLLUP_UPDATE".to_string()
    }

    fn commit(&self) -> Commitment<Self> {
        let mut comm = committable::RawCommitmentBuilder::new(&Self::tag())
            .u64_field("namespace_id", u64::from(self.namespace_id));

        match &self.reserve_url {
            Set(Some(url)) => {
                comm = comm
                    .u64_field("reserve_url", 2)
                    .var_size_bytes(url.as_str().as_ref())
            },
            Set(None) => comm = comm.u64_field("reserve_url", 1),
            Update::Skip => comm = comm.u64_field("reserve_url", 0),
        }

        if let Set(rp) = self.reserve_price {
            comm = comm
                .u64_field("reserve_price", 1)
                .fixed_size_bytes(&rp.to_fixed_bytes());
        } else {
            comm = comm.u64_field("reserve_price", 0);
        }

        if let Set(active) = self.active {
            comm = comm
                .u64_field("active", 1)
                .fixed_size_bytes(&[u8::from(active)]);
        } else {
            comm = comm.u64_field("active", 0);
        }

        if let Set(keys) = &self.signature_keys {
            comm = comm.u64_field("signature_keys", 1);
            for key in keys {
                comm = comm.var_size_bytes(&key.to_bytes());
            }
        } else {
            comm = comm.u64_field("signature_keys", 0);
        }

        comm = comm.var_size_field("signature_key", &self.signature_key.to_bytes());

        if let Set(text) = &self.text {
            comm = comm.u64_field("text", 1).var_size_bytes(text.as_bytes());
        } else {
            comm = comm.u64_field("text", 0);
        }

        comm.finalize()
    }
}
