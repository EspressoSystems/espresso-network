// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

mod api;
pub mod v0_1;
pub mod v0_2 {
    pub use super::v0_1::*;
    pub type Version = vbs::version::StaticVersion<0, 2>;
}
pub mod v0_99;
