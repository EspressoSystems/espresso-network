// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Provides the implementation for AVID-M scheme over GF2 field.

use hotshot_utils::anytrace::*;

pub type AvidmGf2Scheme = vid::avidm_gf2::namespaced::NsAvidmGf2Scheme;
pub type AvidmGf2Param = vid::avidm_gf2::namespaced::NsAvidmGf2Param;
pub type AvidmGf2Commitment = vid::avidm_gf2::namespaced::NsAvidmGf2Commit;
pub type AvidmGf2Share = vid::avidm_gf2::namespaced::NsAvidmGf2Share;
pub type AvidmGf2Common = vid::avidm_gf2::namespaced::NsAvidmGf2Common;

pub fn init_avidm_gf2_param(total_weight: usize) -> Result<AvidmGf2Param> {
    let recovery_threshold = total_weight.div_ceil(3);
    AvidmGf2Param::new(recovery_threshold, total_weight)
        .map_err(|err| error!("Failed to initialize VID: {}", err.to_string()))
}
