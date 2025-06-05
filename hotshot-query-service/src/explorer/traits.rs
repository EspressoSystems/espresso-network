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

use std::fmt::Debug;

use hotshot_types::traits::node_implementation::NodeType;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    availability::{NamespaceId, QueryableHeader},
    Header,
};

/// [ExplorerHeader] is a trait that represents certain extensions to the
/// [BlockHeader] that are specific to the Block Explorer API.  This trait
/// allows for the explorer module to be defined and provide an API for any
/// consuming Block Explorer.
pub trait ExplorerHeader<Types: NodeType>: QueryableHeader<Types>
where
    Header<Types>: QueryableHeader<Types>,
{
    /// BalanceAmount is a type that represents a general balance amount.  It
    /// does not indicate how this balance is represented, just that there is
    /// a representation of it that adheres to the trait restrictions specified.
    type BalanceAmount: Clone + Debug + Serialize + DeserializeOwned + Send + Sync + PartialEq + Eq;

    /// WalletAddress is a type that represents the address of a Wallet.  It
    /// does not indicate how this address is represented, just that there is
    /// a representation of it that adheres to the trait restrictions specified.
    type WalletAddress: Clone + Debug + Serialize + DeserializeOwned + Send + Sync + PartialEq + Eq;

    /// ProposerId is a type that represents the proposer id of the block.  It
    /// does not indicate how this proposer id is represented, just that there is
    /// a representation of it that adheres to the trait restrictions specified.
    type ProposerId: Clone + Debug + Serialize + DeserializeOwned + Send + Sync + PartialEq + Eq;

    /// The proposer id of the block as stored within the block header.
    fn proposer_id(&self) -> Self::ProposerId;

    /// The wallet address of the fee info account contained within the block
    /// header.
    fn fee_info_account(&self) -> Self::WalletAddress;

    /// The balance amount of the fee info contained within the block header.
    fn fee_info_balance(&self) -> Self::BalanceAmount;

    /// The balance amount of the reward for constructing the block.
    fn reward_balance(&self) -> Self::BalanceAmount;

    /// A collection of namespace ids that are contained within the block header.
    fn namespace_ids(&self) -> Vec<NamespaceId<Types>>;
}

/// ExplorerTransaction is a trait that allows the Explorer API to be able to
/// retrieve a namespace id from a transaction.  This is necessary for the
/// Explorer API to be able to display the namespace id for a
/// TransactionSummary.
pub trait ExplorerTransaction<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
{
    /// namespace_id returns the namespace id of the individual transaction.
    fn namespace_id(&self) -> NamespaceId<Types>;

    /// payload_size returns the size of the payload of the transaction.
    fn payload_size(&self) -> u64;
}
