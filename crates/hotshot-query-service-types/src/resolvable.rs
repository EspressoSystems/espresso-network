// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

use committable::{Commitment, Committable};
use either::Either;

/// A reference to a `T` which can be resolved into a whole `T`.
pub trait Resolvable<T: Committable>: Sized {
    /// Get the underlying object if it is available without blocking.
    fn try_resolve(self) -> Result<T, Self>;
    /// Get a commitment to the underlying object.
    fn commitment(&self) -> Commitment<T>;
}

impl<T: Committable> Resolvable<T> for T {
    fn try_resolve(self) -> Result<T, Self> {
        Ok(self)
    }

    fn commitment(&self) -> Commitment<T> {
        self.commit()
    }
}

impl<T: Committable> Resolvable<T> for Either<T, Commitment<T>> {
    fn try_resolve(self) -> Result<T, Self> {
        match self {
            Either::Left(t) => Ok(t),
            Either::Right(c) => Err(Either::Right(c)),
        }
    }

    fn commitment(&self) -> Commitment<T> {
        match self {
            Either::Left(t) => t.commit(),
            Either::Right(c) => *c,
        }
    }
}
