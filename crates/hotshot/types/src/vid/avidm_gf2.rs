// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Provides the implementation for AVID-M scheme over GF2 field.

use std::sync::LazyLock;

use hotshot_utils::anytrace::*;

pub type AvidmGf2Scheme = vid::avidm_gf2::namespaced::NsAvidmGf2Scheme;
pub type AvidmGf2Param = vid::avidm_gf2::namespaced::NsAvidmGf2Param;
pub type AvidmGf2Commitment = vid::avidm_gf2::namespaced::NsAvidmGf2Commit;
pub type AvidmGf2Share = vid::avidm_gf2::namespaced::NsAvidmGf2Share;
pub type AvidmGf2Common = vid::avidm_gf2::namespaced::NsAvidmGf2Common;

/// Benchmark only: shares covering 2/3 of the weight rebuild a block instead of 1/3, which
/// halves the encoded size. Incompatible with nodes that run without it, and safe only while
/// a node rebuilds a block before it votes (below protocol 0.8): async VID needs any f+1
/// honest shares to be enough.
static TWO_THIRDS_RECOVERY: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ESPRESSO_BENCH_VID_TWO_THIRDS").is_some());

pub fn init_avidm_gf2_param(total_weight: usize) -> Result<AvidmGf2Param> {
    let recovery_threshold = recovery_threshold(total_weight, *TWO_THIRDS_RECOVERY);
    AvidmGf2Param::new(recovery_threshold, total_weight)
        .map_err(|err| error!("Failed to initialize VID: {}", err.to_string()))
}

fn recovery_threshold(total_weight: usize, two_thirds: bool) -> usize {
    if two_thirds {
        (2 * total_weight).div_ceil(3)
    } else {
        total_weight.div_ceil(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_thirds_doubles_the_threshold() {
        // 5 equal nodes after `approximate_weights`: 201 units each.
        assert_eq!(recovery_threshold(1005, false), 335);
        assert_eq!(recovery_threshold(1005, true), 670);
        assert_eq!(recovery_threshold(1, true), 1);
    }

    #[test]
    fn two_thirds_recovers_from_two_thirds_of_the_weight_only() {
        let weights = [1u32; 6];
        let param = AvidmGf2Param::new(recovery_threshold(6, true), 6).unwrap();
        let payload: Vec<u8> = (0..=u8::MAX).collect();
        let (_, common, shares) =
            AvidmGf2Scheme::ns_disperse(&param, &weights, &payload, [0..100, 100..256]).unwrap();
        assert_eq!(
            AvidmGf2Scheme::recover(&common, &shares[2..]).unwrap(),
            payload
        );
        assert!(AvidmGf2Scheme::recover(&common, &shares[3..]).is_err());
    }
}
