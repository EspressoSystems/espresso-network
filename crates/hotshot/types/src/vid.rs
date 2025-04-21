// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! This module provides:
//! - an opaque constructor [`vid_scheme`] that returns a new instance of a
//!   VID scheme.
//! - type aliases [`VidCommitment`], [`VidCommon`], [`VidShare`]
//!   for [`VidScheme`] assoc types.
//!
//! Purpose: the specific choice of VID scheme is an implementation detail.
//! This crate and all downstream crates should talk to the VID scheme only
//! via the traits exposed here.

#![allow(missing_docs)]

pub mod advz;
pub mod avidm;
