// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Provides the implementation for AVID-M scheme

use hotshot_utils::anytrace::*;

pub type AvidMScheme = vid::avid_m::namespaced::NsAvidMScheme;
pub type AvidMParam = vid::avid_m::namespaced::NsAvidMParam;
pub type AvidMCommitment = vid::avid_m::namespaced::NsAvidMCommit;
pub type AvidMShare = vid::avid_m::namespaced::NsAvidMShare;
pub type AvidMCommon = AvidMParam;

pub fn init_avidm_param(num_storage_nodes: usize) -> Result<AvidMParam> {
    let recovery_threshold = 1 << num_storage_nodes.ilog2();
    AvidMParam::new(recovery_threshold, num_storage_nodes)
        .map_err(|err| error!("Failed to initialize VID: {}", err.to_string()))
}
