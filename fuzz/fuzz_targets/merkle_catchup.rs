//! Fee and reward account snapshots and block frontiers from catchup peers.
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
//! element decoder. A response the node accepts must leave it in the same
//! state an honest one would.

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
    BLOCK_MERKLE_TREE_HEIGHT, BlockMerkleTree, FEE_MERKLE_TREE_HEIGHT, FeeAccount, FeeAccountProof,
    FeeAmount, FeeMerkleTree, Header,
    v0_3::{
        REWARD_MERKLE_TREE_V1_HEIGHT, RewardAccountProofV1, RewardAccountV1, RewardAmount,
        RewardMerkleTreeV1,
    },
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

/// Block counts for the frontier fixtures. With arity 3 they cover a last leaf
/// in each child slot, and frontiers whose parent levels are full.
const LEAF_COUNTS: [u64; 7] = [1, 2, 3, 4, 9, 10, 28];

#[derive(Arbitrary, Debug)]
enum Input {
    FeeAccounts(AccountsResponse),
    RewardAccounts(AccountsResponse),
    Frontier(FrontierResponse),
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

#[derive(Arbitrary, Debug)]
struct FrontierResponse {
    fixture: u8,
    pos: Option<u64>,
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
    Children {
        node: u16,
        len: u8,
    },
    Graft {
        node: u16,
        slot: u8,
        from: u16,
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
trait AccountTree: ForgetableUniversalMerkleTreeScheme + Clone {
    type Proof;

    fn empty() -> Self;
    fn index(account: Address) -> Self::Index;
    fn amount(value: U256) -> Self::Element;
    fn value(amount: &Self::Element) -> U256;
    fn prove_account(&self, account: Address) -> Option<(Self::Proof, U256)>;
    fn verify_proof(proof: &Self::Proof, root: &Self::Commitment) -> anyhow::Result<U256>;
    fn remember_proof(&mut self, proof: &Self::Proof) -> anyhow::Result<()>;
    fn edited(&self, height: Option<u64>, edits: &[NodeEdit]) -> Option<Self>;
}

impl AccountTree for FeeMerkleTree {
    type Proof = FeeAccountProof;

    fn empty() -> Self {
        Self::new(FEE_MERKLE_TREE_HEIGHT)
    }

    fn index(account: Address) -> FeeAccount {
        FeeAccount(account)
    }

    fn amount(value: U256) -> FeeAmount {
        FeeAmount(value)
    }

    fn value(amount: &FeeAmount) -> U256 {
        amount.0
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

    fn edited(&self, height: Option<u64>, edits: &[NodeEdit]) -> Option<Self> {
        edit_tree::<FeeAmount, FeeAccount, _>(self, height, edits)
    }
}

impl AccountTree for RewardMerkleTreeV1 {
    type Proof = RewardAccountProofV1;

    fn empty() -> Self {
        Self::new(REWARD_MERKLE_TREE_V1_HEIGHT)
    }

    fn index(account: Address) -> RewardAccountV1 {
        RewardAccountV1(account)
    }

    fn amount(value: U256) -> RewardAmount {
        RewardAmount(value)
    }

    fn value(amount: &RewardAmount) -> U256 {
        amount.0
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

    fn edited(&self, height: Option<u64>, edits: &[NodeEdit]) -> Option<Self> {
        edit_tree::<RewardAmount, RewardAccountV1, _>(self, height, edits)
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
static REWARD: LazyLock<Accounts<RewardMerkleTreeV1>> = LazyLock::new(Accounts::new);
static FRONTIERS: LazyLock<Vec<Frontier>> =
    LazyLock::new(|| LEAF_COUNTS.into_iter().map(Frontier::new).collect());

fn next_block() -> Commitment<Header> {
    Commitment::from_raw([0xff; 32])
}

fuzz_target!(
    init: {
        FEE.assert_honest_responses_accepted();
        REWARD.assert_honest_responses_accepted();
        for frontier in FRONTIERS.iter() {
            assert!(frontier.check(&FrontierResponse { fixture: 0, pos: None, edits: Vec::new() }));
        }
    },
    |input: Input| {
        match input {
            Input::FeeAccounts(response) => FEE.check(&response),
            Input::RewardAccounts(response) => REWARD.check(&response),
            Input::Frontier(response) => {
                FRONTIERS[usize::from(response.fixture) % FRONTIERS.len()].check(&response)
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
            full.update_with(T::index(account), |_| Some(T::amount(balance)))
                .unwrap();
            balances.push((account, balance));
        }
        balances.extend(absent.map(|account| (account, U256::ZERO)));
        Self { full, balances }
    }

    fn assert_honest_responses_accepted(&self) {
        for request in 1..=u8::MAX {
            assert!(self.check(&AccountsResponse {
                request,
                known: None,
                height: None,
                edits: Vec::new(),
            }));
        }
    }

    /// Run `response` through catchup as the node does. Returns whether the
    /// node accepts it.
    fn check(&self, response: &AccountsResponse) -> bool {
        let known = response
            .known
            .map(|i| self.balances[usize::from(i) % self.balances.len()].0);
        let requested: Vec<(Address, U256)> = self
            .balances
            .iter()
            .enumerate()
            .filter(|&(i, &(account, _))| response.request >> i & 1 == 1 && Some(account) != known)
            .map(|(_, &entry)| entry)
            .collect();
        if requested.is_empty() {
            return false;
        }

        let root = self.full.commitment();
        let Some(snapshot) = self
            .snapshot(requested.iter().map(|(account, _)| *account))
            .edited(response.height, &response.edits)
        else {
            return false;
        };
        let Some(proofs) = requested
            .iter()
            .map(|&(account, balance)| {
                let (proof, _) = snapshot.prove_account(account)?;
                let proved = T::verify_proof(&proof, &root).ok()?;
                assert_eq!(proved, balance, "proved a wrong balance for {account}");
                Some(proof)
            })
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };

        let mut local = T::from_commitment(&root);
        let mut honest = self.full.clone();
        if let Some(account) = known {
            let (proof, _) = self.full.prove_account(account).unwrap();
            local.remember_proof(&proof).unwrap();
        }
        for proof in &proofs {
            local
                .remember_proof(proof)
                .expect("a verified proof is remembered");
        }
        for (account, _) in &requested {
            for tree in [&mut local, &mut honest] {
                let credited = tree.update_with(T::index(*account), |balance| {
                    Some(T::amount(
                        balance.map_or(U256::ZERO, T::value) + U256::from(1),
                    ))
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
        true
    }

    /// The partial tree an honest peer returns for `accounts`.
    fn snapshot(&self, accounts: impl IntoIterator<Item = Address>) -> T {
        let mut snapshot = T::from_commitment(self.full.commitment());
        for account in accounts {
            let index = T::index(account);
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

    /// Run `response` through catchup as the node does. Returns whether the
    /// node accepts it.
    fn check(&self, response: &FrontierResponse) -> bool {
        let mut proof = self.proof.clone();
        if let Some(pos) = response.pos {
            proof.pos = pos;
        }
        let mut nodes: Vec<_> = proof.proof.into_iter().map(Arc::new).collect();
        if !apply_edits(&mut nodes, &response.edits) {
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
        assert!(
            local.commitment() == self.tree.commitment(),
            "remembering the frontier changed the root"
        );
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
/// node would.
fn edit_tree<E, I, T>(tree: &T, height: Option<u64>, edits: &[NodeEdit]) -> Option<T>
where
    E: Element + CanonicalSerialize + CanonicalDeserialize,
    I: Index + CanonicalSerialize + CanonicalDeserialize,
    T: Serialize + DeserializeOwned,
{
    let mut wire: TreeWire<E, I> =
        bincode::deserialize(&bincode::serialize(tree).unwrap()).unwrap();
    if let Some(height) = height {
        wire.height = height as usize;
    }
    if !apply_edits(std::slice::from_mut(&mut wire.root), edits) {
        return None;
    }
    bincode::deserialize(&bincode::serialize(&wire).unwrap()).ok()
}

/// Apply `edits` in order. Returns false if they grew the nodes past
/// [`MAX_NODES`].
fn apply_edits<E: Element, I: Index>(roots: &mut [Arc<Node<E, I>>], edits: &[NodeEdit]) -> bool {
    for edit in edits {
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
                let from = nth(roots, at(from)).clone();
                *nth_mut(roots, at(to)) = from;
            },
            NodeEdit::Transplant { from, to } => {
                let mut from = nth(roots, at(from)).clone();
                let to = nth_mut(roots, at(to));
                set_value(&mut from, to.value());
                *to = from;
            },
            NodeEdit::Value { node, from } => {
                let value = nth(roots, at(from)).value();
                set_value(nth_mut(roots, at(node)), value);
            },
            NodeEdit::Elem { node, from } => {
                let MerkleNode::Leaf { elem: from, .. } = &**nth(roots, at(from)) else {
                    continue;
                };
                let from = from.clone();
                if let MerkleNode::Leaf { elem, .. } = Arc::make_mut(nth_mut(roots, at(node))) {
                    *elem = from;
                }
            },
            NodeEdit::Children { node, len } => {
                if let MerkleNode::Branch { children, .. } = Arc::make_mut(nth_mut(roots, at(node)))
                {
                    children.resize(usize::from(len), Arc::new(MerkleNode::Empty));
                }
            },
            NodeEdit::Graft { node, slot, from } => {
                let from = nth(roots, at(from)).clone();
                if let MerkleNode::Branch { children, .. } = Arc::make_mut(nth_mut(roots, at(node)))
                    && !children.is_empty()
                {
                    let slot = usize::from(slot) % children.len();
                    children[slot] = from;
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

fn nth<E: Element, I: Index>(roots: &[Arc<Node<E, I>>], n: usize) -> &Arc<Node<E, I>> {
    find(roots, &mut { n }).expect("n is below the node count")
}

fn nth_mut<E: Element, I: Index>(roots: &mut [Arc<Node<E, I>>], n: usize) -> &mut Arc<Node<E, I>> {
    find_mut(roots, &mut { n }).expect("n is below the node count")
}

fn find<'a, E: Element, I: Index>(
    nodes: &'a [Arc<Node<E, I>>],
    n: &mut usize,
) -> Option<&'a Arc<Node<E, I>>> {
    for node in nodes {
        if matches!(**node, MerkleNode::Empty) {
            continue;
        }
        if *n == 0 {
            return Some(node);
        }
        *n -= 1;
        if let MerkleNode::Branch { children, .. } = &**node
            && let Some(found) = find(children, n)
        {
            return Some(found);
        }
    }
    None
}

fn find_mut<'a, E: Element, I: Index>(
    nodes: &'a mut [Arc<Node<E, I>>],
    n: &mut usize,
) -> Option<&'a mut Arc<Node<E, I>>> {
    for node in nodes {
        if matches!(**node, MerkleNode::Empty) {
            continue;
        }
        if *n == 0 {
            return Some(node);
        }
        *n -= 1;
        if let MerkleNode::Branch { children, .. } = Arc::make_mut(node)
            && let Some(found) = find_mut(children, n)
        {
            return Some(found);
        }
    }
    None
}
