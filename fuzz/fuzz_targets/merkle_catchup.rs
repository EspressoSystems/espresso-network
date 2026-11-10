//! Fee and v1 reward account snapshots and block frontiers from catchup peers.
//!
//! A node missing state asks a peer for it. For accounts the peer returns a
//! partial merkle tree. The node proves each requested account out of it and
//! verifies the proof against a root it already trusts, then remembers the
//! proofs into its own state and updates the balances, expecting both to
//! succeed. For the block tree the peer returns a proof of the last leaf, which
//! the node remembers into a tree it only knows the root of before appending
//! the next block.
//!
//! An honest response is decoded, its tree structure edited, and encoded
//! again, so the input reaches the merkle code instead of stopping in the field
//! element decoder. A response the node accepts must leave it holding the same
//! accounts and roots an honest one would.
//!
//! Fee catchup runs only for headers below the new protocol, and v1 reward
//! catchup only up to the epoch version. The v2 reward tree, which catchup
//! fetches whole, is not covered here.

#![no_main]

use std::{
    marker::PhantomData,
    sync::{Arc, LazyLock},
};

use alloy::primitives::{Address, U256};
use arbitrary::Arbitrary;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use committable::Commitment;
use espresso_types::{
    BLOCK_MERKLE_TREE_HEIGHT, BlockMerkleTree, FEE_MERKLE_TREE_HEIGHT, FeeAccountProof,
    FeeMerkleTree, Header,
    v0_3::{REWARD_MERKLE_TREE_V1_HEIGHT, RewardAccountProofV1, RewardMerkleTreeV1},
};
use jf_merkle_tree_compat::{
    AppendableMerkleTreeScheme, Element, ForgetableMerkleTreeScheme,
    ForgetableUniversalMerkleTreeScheme, Index, LookupResult, MerkleTreeScheme,
    internal::{MerkleNode, MerkleProof},
    prelude::Sha3Node,
};
use libfuzzer_sys::fuzz_target;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Largest number of non-empty nodes an edited response may hold, so that
/// copying a subtree into itself cannot grow one input without bound.
const MAX_NODES: usize = 1 << 12;

/// Edits applied per input. Each one walks the nodes, so this bounds the time
/// of a run.
const MAX_EDITS: usize = 64;

/// Block counts for the frontier fixtures. With arity 3 they cover a last leaf
/// in each child slot, and frontiers whose parent levels are full.
const LEAF_COUNTS: [u64; 7] = [1, 2, 3, 4, 9, 10, 28];

#[derive(Arbitrary, Debug)]
enum Input {
    FeeAccounts(AccountsResponse),
    RewardAccountsV1(AccountsResponse),
    Frontier { fixture: u8, edits: Vec<NodeEdit> },
}

#[derive(Arbitrary, Debug)]
struct AccountsResponse {
    /// Bit `i` requests the `i`th fixture account.
    request: u8,
    /// A fixture account whose proof the node already holds, and so does not
    /// request.
    known: Option<u8>,
    height: Option<u64>,
    edits: Vec<NodeEdit>,
}

/// One change to the nodes of a response. Nodes are numbered in preorder,
/// skipping empty ones, and every number wraps around the node count.
#[derive(Arbitrary, Debug)]
enum NodeEdit {
    Forget {
        node: u16,
    },
    Empty {
        node: u16,
    },
    Copy {
        from: u16,
        to: u16,
    },
    /// Like `Copy`, but `to` keeps its value, so its parent's digest still
    /// matches.
    Transplant {
        from: u16,
        to: u16,
    },
    Value {
        node: u16,
        from: u16,
    },
    Elem {
        node: u16,
        from: u16,
    },
    Pos {
        node: u16,
        from: u16,
    },
    Children {
        node: u16,
        len: u16,
    },
    /// The only edits that write into an empty slot, since numbering skips
    /// empty nodes.
    Graft {
        node: u16,
        slot: u8,
        from: u16,
    },
    Hollow {
        node: u16,
        slot: u8,
    },
}

type Node<E, I> = MerkleNode<E, I, Sha3Node>;

/// The serde layout of the jellyfish merkle tree types, whose fields are
/// private.
#[derive(Serialize, Deserialize)]
#[serde(
    bound = "E: CanonicalSerialize + CanonicalDeserialize, I: CanonicalSerialize + \
             CanonicalDeserialize"
)]
struct TreeWire<E: Element, I: Index> {
    root: Arc<Node<E, I>>,
    height: usize,
    num_leaves: u64,
    _phantom: PhantomData<()>,
}

/// The fee and v1 reward trees, which catchup fetches the same way.
trait AccountTree:
    ForgetableUniversalMerkleTreeScheme<
        Element: From<U256> + Into<U256> + Copy + CanonicalSerialize + CanonicalDeserialize,
        Index: From<Address> + CanonicalSerialize + CanonicalDeserialize,
        MembershipProof: PartialEq,
        NonMembershipProof: PartialEq,
    > + Clone
    + Serialize
    + DeserializeOwned
{
    type Proof;

    fn empty() -> Self;
    fn prove_account(&self, account: Address) -> Option<(Self::Proof, U256)>;
    fn verify_proof(proof: &Self::Proof, root: &Self::Commitment) -> anyhow::Result<U256>;
    fn remember_proof(&mut self, proof: &Self::Proof) -> anyhow::Result<()>;
}

impl AccountTree for FeeMerkleTree {
    type Proof = FeeAccountProof;

    fn empty() -> Self {
        Self::new(FEE_MERKLE_TREE_HEIGHT)
    }

    fn prove_account(&self, account: Address) -> Option<(FeeAccountProof, U256)> {
        FeeAccountProof::prove(self, account)
    }

    fn verify_proof(proof: &FeeAccountProof, root: &Self::Commitment) -> anyhow::Result<U256> {
        proof.verify(root)
    }

    fn remember_proof(&mut self, proof: &FeeAccountProof) -> anyhow::Result<()> {
        proof.remember(self)
    }
}

impl AccountTree for RewardMerkleTreeV1 {
    type Proof = RewardAccountProofV1;

    fn empty() -> Self {
        Self::new(REWARD_MERKLE_TREE_V1_HEIGHT)
    }

    fn prove_account(&self, account: Address) -> Option<(RewardAccountProofV1, U256)> {
        RewardAccountProofV1::prove(self, account)
    }

    fn verify_proof(proof: &RewardAccountProofV1, root: &Self::Commitment) -> anyhow::Result<U256> {
        proof.verify(root)
    }

    fn remember_proof(&mut self, proof: &RewardAccountProofV1) -> anyhow::Result<()> {
        proof.remember(self)
    }
}

struct Accounts<T> {
    full: T,
    /// Every fixture account with its balance, zero for the ones not in the
    /// tree.
    balances: Vec<(Address, U256)>,
}

struct Frontier {
    tree: BlockMerkleTree,
    last: Commitment<Header>,
    proof: MerkleProof<Commitment<Header>, u64, Sha3Node, 3>,
    pushed: BlockMerkleTree,
}

static FEE: LazyLock<Accounts<FeeMerkleTree>> = LazyLock::new(Accounts::new);
static REWARD_V1: LazyLock<Accounts<RewardMerkleTreeV1>> = LazyLock::new(Accounts::new);
static FRONTIERS: LazyLock<Vec<Frontier>> =
    LazyLock::new(|| LEAF_COUNTS.into_iter().map(Frontier::new).collect());

fn next_block() -> Commitment<Header> {
    Commitment::from_raw([0xff; 32])
}

fuzz_target!(
    init: {
        FEE.assert_honest_responses_accepted();
        REWARD_V1.assert_honest_responses_accepted();
        for frontier in FRONTIERS.iter() {
            assert!(frontier.check(&[]));
        }
    },
    |input: Input| {
        match input {
            Input::FeeAccounts(response) => FEE.check(&response),
            Input::RewardAccountsV1(response) => REWARD_V1.check(&response),
            Input::Frontier { fixture, edits } => {
                FRONTIERS[usize::from(fixture) % FRONTIERS.len()].check(&edits)
            },
        };
    }
);

impl<T: AccountTree> Accounts<T> {
    fn new() -> Self {
        // The traversal path of an account starts at its last byte, so these
        // share a path with the first account down to different depths.
        let shared = |at: usize, byte: u8| {
            let mut account = [0x11; 20];
            account[at] = byte;
            Address::from(account)
        };
        let present = [
            shared(0, 0x11),
            shared(0, 0x22),
            shared(10, 0x22),
            shared(19, 0x22),
            Address::repeat_byte(0x33),
            Address::repeat_byte(0x44),
        ];
        let absent = [shared(1, 0x22), Address::repeat_byte(0x55)];

        let mut full = T::empty();
        let mut balances = Vec::new();
        for (i, account) in present.into_iter().enumerate() {
            let balance = U256::from(1000 * (i + 1));
            full.update_with(T::Index::from(account), |_| Some(balance.into()))
                .unwrap();
            balances.push((account, balance));
        }
        balances.extend(absent.map(|account| (account, U256::ZERO)));
        Self { full, balances }
    }

    /// Every request from a node that knows no account, and a request for
    /// everything else from a node that knows each one in turn.
    fn assert_honest_responses_accepted(&self) {
        let unknown = (1..=u8::MAX).map(|request| (request, None));
        let known = (0..self.balances.len() as u8).map(|i| (u8::MAX, Some(i)));
        for (request, known) in unknown.chain(known) {
            let response = AccountsResponse {
                request,
                known,
                height: None,
                edits: Vec::new(),
            };
            assert!(
                self.check(&response),
                "an honest response is rejected: {response:?}"
            );
        }
    }

    /// Run `response` through catchup as the node does. Returns whether the
    /// node accepts it.
    fn check(&self, response: &AccountsResponse) -> bool {
        let requested = self.requested(response);
        if requested.is_empty() {
            return false;
        }
        let Some(proofs) = self.fetch(&requested, response) else {
            return false;
        };
        self.catch_up(self.known(response), &requested, &proofs);
        true
    }

    fn known(&self, response: &AccountsResponse) -> Option<Address> {
        response
            .known
            .map(|i| self.balances[usize::from(i) % self.balances.len()].0)
    }

    /// The fixture accounts `response` requests, which never include the one
    /// the node already knows.
    fn requested(&self, response: &AccountsResponse) -> Vec<(Address, U256)> {
        let known = self.known(response);
        self.balances
            .iter()
            .enumerate()
            .filter(|&(i, &(account, _))| {
                (response.request >> i) & 1 == 1 && Some(account) != known
            })
            .map(|(_, &entry)| entry)
            .collect()
    }

    /// The proofs the node takes from the edited response, as the catchup
    /// providers check them.
    fn fetch(
        &self,
        requested: &[(Address, U256)],
        response: &AccountsResponse,
    ) -> Option<Vec<T::Proof>> {
        let root = self.full.commitment();
        let snapshot = edit_tree::<T::Element, T::Index, T>(
            &self.snapshot(requested.iter().map(|(account, _)| *account)),
            response.height,
            &response.edits,
        )?;
        requested
            .iter()
            .map(|&(account, balance)| {
                let (proof, _) = snapshot.prove_account(account)?;
                let proved = T::verify_proof(&proof, &root).ok()?;
                assert_eq!(proved, balance, "proved a wrong balance for {account}");
                Some(proof)
            })
            .collect()
    }

    /// Remember `proofs` into a node that already holds `known`, and update
    /// every balance it holds, as state validation does. The node must end up
    /// with the honest accounts and root.
    fn catch_up(&self, known: Option<Address>, requested: &[(Address, U256)], proofs: &[T::Proof]) {
        let mut local = T::from_commitment(self.full.commitment());
        if let Some(account) = known {
            let (proof, _) = self.full.prove_account(account).unwrap();
            local.remember_proof(&proof).unwrap();
        }
        for proof in proofs {
            local
                .remember_proof(proof)
                .expect("a verified proof is remembered");
        }

        let mut honest = self.full.clone();
        let held = requested.iter().map(|(account, _)| *account).chain(known);
        for account in held {
            let index = T::Index::from(account);
            assert!(
                local.universal_lookup(&index) == honest.universal_lookup(&index),
                "remembered {account} differently from the honest tree"
            );
            for tree in [&mut local, &mut honest] {
                let credited = tree.update_with(&index, |balance| {
                    let balance: U256 = balance.copied().map_or(U256::ZERO, Into::into);
                    Some((balance + U256::from(1)).into())
                });
                assert!(
                    matches!(
                        credited,
                        Ok(LookupResult::Ok(..) | LookupResult::NotFound(()))
                    ),
                    "cannot credit {account} after catching up"
                );
            }
        }
        assert!(
            local.commitment() == honest.commitment(),
            "state after catching up differs from the honest state"
        );
    }

    /// The partial tree an honest peer returns for `accounts`.
    fn snapshot(&self, accounts: impl IntoIterator<Item = Address>) -> T {
        let mut snapshot = T::from_commitment(self.full.commitment());
        for account in accounts {
            let index = T::Index::from(account);
            match self.full.universal_lookup(&index) {
                LookupResult::Ok(elem, proof) => snapshot.remember(index, elem, proof).unwrap(),
                LookupResult::NotFound(proof) => {
                    snapshot.non_membership_remember(index, proof).unwrap()
                },
                LookupResult::NotInMemory => unreachable!("the full tree holds every account"),
            }
        }
        snapshot
    }
}

impl Frontier {
    fn new(num_leaves: u64) -> Self {
        let blocks: Vec<_> = (0..num_leaves)
            .map(|i| Commitment::from_raw([i as u8; 32]))
            .collect();
        let tree = BlockMerkleTree::from_elems(Some(BLOCK_MERKLE_TREE_HEIGHT), &blocks).unwrap();
        let (last, proof) = tree.lookup(num_leaves - 1).expect_ok().unwrap();
        let mut pushed = tree.clone();
        pushed.push(next_block()).unwrap();
        Self {
            tree,
            last,
            proof,
            pushed,
        }
    }

    /// Run the frontier with `edits` through catchup as the node does. Returns
    /// whether the node accepts it.
    fn check(&self, edits: &[NodeEdit]) -> bool {
        let mut proof = self.proof.clone();
        let mut nodes: Vec<_> = proof.proof.into_iter().map(Arc::new).collect();
        if !apply_node_edits(&mut nodes, edits) {
            return false;
        }
        proof.proof = nodes.into_iter().map(Arc::unwrap_or_clone).collect();

        let mut local = BlockMerkleTree::from_commitment(self.tree.commitment());
        let Some(elem) = proof.elem() else {
            return false;
        };
        if local
            .remember(local.num_leaves() - 1, elem, &proof)
            .is_err()
        {
            return false;
        }
        let (last, _) = local
            .lookup(local.num_leaves() - 1)
            .expect_ok()
            .expect("the remembered frontier holds the last block");
        assert_eq!(last, self.last, "remembered the wrong last block");
        local
            .push(next_block())
            .expect("a block is appended after the frontier");
        assert!(
            local.commitment() == self.pushed.commitment(),
            "appending after the frontier gives a different root"
        );
        true
    }
}

/// Re-encode `tree` with `height` and `edits` applied, and decode it as the
/// node would. Returns `None` if the edits grew it past [`MAX_NODES`].
fn edit_tree<E, I, T>(tree: &T, height: Option<u64>, edits: &[NodeEdit]) -> Option<T>
where
    E: Element + CanonicalSerialize + CanonicalDeserialize,
    I: Index + CanonicalSerialize + CanonicalDeserialize,
    T: Serialize + DeserializeOwned,
{
    let mut wire: TreeWire<E, I> = bincode::deserialize(&bincode::serialize(tree).unwrap())
        .expect("TreeWire mirrors the jellyfish tree layout");
    if let Some(height) = height {
        wire.height = height as usize;
    }
    if !apply_node_edits(std::slice::from_mut(&mut wire.root), edits) {
        return None;
    }
    Some(
        bincode::deserialize(&bincode::serialize(&wire).unwrap())
            .expect("TreeWire mirrors the jellyfish tree layout"),
    )
}

/// Apply up to [`MAX_EDITS`] of `edits` in order. Returns false if they grew
/// the nodes past [`MAX_NODES`].
fn apply_node_edits<E: Element, I: Index>(
    roots: &mut [Arc<Node<E, I>>],
    edits: &[NodeEdit],
) -> bool {
    for edit in edits.iter().take(MAX_EDITS) {
        let count = count(roots);
        if count == 0 {
            return true;
        }
        if count > MAX_NODES {
            return false;
        }
        let at = |node: u16| usize::from(node) % count;
        match *edit {
            NodeEdit::Forget { node } => {
                let node = nth_mut(roots, at(node));
                *node = Arc::new(MerkleNode::ForgettenSubtree {
                    value: node.value(),
                });
            },
            NodeEdit::Empty { node } => *nth_mut(roots, at(node)) = Arc::new(MerkleNode::Empty),
            NodeEdit::Copy { from, to } => {
                let from = nth_mut(roots, at(from)).clone();
                *nth_mut(roots, at(to)) = from;
            },
            NodeEdit::Transplant { from, to } => {
                let mut from = nth_mut(roots, at(from)).clone();
                let to = nth_mut(roots, at(to));
                set_value(&mut from, to.value());
                *to = from;
            },
            NodeEdit::Value { node, from } => {
                let value = nth_mut(roots, at(from)).value();
                set_value(nth_mut(roots, at(node)), value);
            },
            NodeEdit::Elem { node, from } => {
                let MerkleNode::Leaf { elem: from, .. } = &**nth_mut(roots, at(from)) else {
                    continue;
                };
                let from = from.clone();
                if let MerkleNode::Leaf { elem, .. } = Arc::make_mut(nth_mut(roots, at(node))) {
                    *elem = from;
                }
            },
            NodeEdit::Pos { node, from } => {
                let MerkleNode::Leaf { pos: from, .. } = &**nth_mut(roots, at(from)) else {
                    continue;
                };
                let from = from.clone();
                if let MerkleNode::Leaf { pos, .. } = Arc::make_mut(nth_mut(roots, at(node))) {
                    *pos = from;
                }
            },
            NodeEdit::Children { node, len } => {
                if let MerkleNode::Branch { children, .. } = Arc::make_mut(nth_mut(roots, at(node)))
                {
                    children.resize(usize::from(len), Arc::new(MerkleNode::Empty));
                }
            },
            NodeEdit::Graft { node, slot, from } => {
                let from = nth_mut(roots, at(from)).clone();
                if let MerkleNode::Branch { children, .. } = Arc::make_mut(nth_mut(roots, at(node)))
                    && !children.is_empty()
                {
                    let slot = usize::from(slot) % children.len();
                    children[slot] = from;
                }
            },
            NodeEdit::Hollow { node, slot } => {
                if let MerkleNode::Branch { children, .. } = Arc::make_mut(nth_mut(roots, at(node)))
                    && !children.is_empty()
                {
                    let slot = usize::from(slot) % children.len();
                    children[slot] = Arc::new(MerkleNode::ForgettenSubtree {
                        value: children[slot].value(),
                    });
                }
            },
        }
    }
    count(roots) <= MAX_NODES
}

fn set_value<E: Element, I: Index>(node: &mut Arc<Node<E, I>>, new: Sha3Node) {
    match Arc::make_mut(node) {
        MerkleNode::Empty => {},
        MerkleNode::Branch { value, .. }
        | MerkleNode::Leaf { value, .. }
        | MerkleNode::ForgettenSubtree { value } => *value = new,
    }
}

fn count<E: Element, I: Index>(nodes: &[Arc<Node<E, I>>]) -> usize {
    nodes
        .iter()
        .map(|node| match &**node {
            MerkleNode::Empty => 0,
            MerkleNode::Branch { children, .. } => 1 + count(children),
            MerkleNode::Leaf { .. } | MerkleNode::ForgettenSubtree { .. } => 1,
        })
        .sum()
}

/// The `n`th non-empty node in preorder. Only the branches on the way to it
/// are made unique, so a shared subtree elsewhere is not copied.
fn nth_mut<E: Element, I: Index>(
    nodes: &mut [Arc<Node<E, I>>],
    mut n: usize,
) -> &mut Arc<Node<E, I>> {
    for node in nodes {
        let size = count(std::slice::from_ref(node));
        if n >= size {
            n -= size;
            continue;
        }
        if n == 0 {
            return node;
        }
        let MerkleNode::Branch { children, .. } = Arc::make_mut(node) else {
            unreachable!("only a branch counts more than one node");
        };
        return nth_mut(children, n - 1);
    }
    unreachable!("n is below the node count")
}
