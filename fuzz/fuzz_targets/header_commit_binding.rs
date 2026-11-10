//! Block headers as a query server returns them.
//!
//! A client that holds a header commitment, from a certificate or a merkle
//! proof, accepts any header that hashes to it. So a header commitment must
//! bind every field a client acts on: two headers with the same commitment
//! must be the same header, up to the fields known to sit outside it.
//!
//! Each input edits the JSON of a reference header from `data/v*`, decodes it,
//! and checks it against the reference. Edits work on the decoded JSON rather
//! than its bytes, so a commitment can move to another TaggedBase64 tag with a
//! valid checksum, which byte edits almost never produce.

#![no_main]

use std::sync::LazyLock;

use arbitrary::Arbitrary;
use committable::Committable;
use espresso_types::Header;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use tagged_base64::TaggedBase64;

const REFERENCE_JSON: [&str; 7] = [
    include_str!("../../data/v1/header.json"),
    include_str!("../../data/v2/header.json"),
    include_str!("../../data/v3/header.json"),
    include_str!("../../data/v4/header.json"),
    include_str!("../../data/v5/header.json"),
    include_str!("../../data/v6/header.json"),
    include_str!("../../data/v7/header.json"),
];

/// Tags of the TaggedBase64 values in a header, so a retag can turn one into
/// another.
const TAGS: [&str; 5] = [
    "HASH",
    "AvidMCommit",
    "AvidmGf2Commit",
    "MERKLE_COMM",
    "FIELD",
];

#[derive(Arbitrary, Debug)]
struct Input {
    reference: u8,
    edits: Vec<JsonEdit>,
}

/// Leaf numbers index the scalar values of the reference header's JSON, and
/// wrap around their count.
#[derive(Arbitrary, Debug)]
enum JsonEdit {
    Retag { leaf: u16, tag: u8 },
    FlipBody { leaf: u16, at: u8, xor: u8 },
    SetNumber { leaf: u16, value: u64 },
    Null { leaf: u16 },
    Copy { from: u16, to: u16 },
}

struct Reference {
    header: Header,
    json: Value,
    /// JSON pointers to every scalar in `json`.
    leaves: Vec<String>,
}

static REFERENCES: LazyLock<Vec<Reference>> =
    LazyLock::new(|| REFERENCE_JSON.into_iter().map(Reference::new).collect());

fuzz_target!(
    init: {
        for reference in REFERENCES.iter() {
            assert_round_trips(&reference.header);
        }
    },
    |input: Input| {
        let reference = &REFERENCES[usize::from(input.reference) % REFERENCES.len()];
        let mut json = reference.json.clone();
        for edit in &input.edits {
            reference.apply(&mut json, edit);
        }
        let Ok(header) = serde_json::from_value::<Header>(json) else {
            return;
        };
        assert_round_trips(&header);
        if header.commit() == reference.header.commit() {
            assert_eq!(
                committed_json(&header),
                committed_json(&reference.header),
                "two headers share a commitment"
            );
        }
    }
);

fn assert_round_trips(header: &Header) {
    let json: Header = serde_json::from_value(serde_json::to_value(header).unwrap())
        .expect("a header decodes from its JSON");
    assert_eq!(&json, header, "JSON round trip changed the header");
    let binary: Header = bincode::deserialize(&bincode::serialize(header).unwrap())
        .expect("a header decodes from its bincode");
    assert_eq!(&binary, header, "bincode round trip changed the header");
}

/// The header's JSON without the builder signature, which the commitment
/// leaves out by design.
fn committed_json(header: &Header) -> Value {
    let mut json = serde_json::to_value(header).unwrap();
    remove_key(&mut json, "builder_signature");
    json
}

fn remove_key(json: &mut Value, key: &str) {
    match json {
        Value::Object(map) => {
            map.remove(key);
            map.values_mut().for_each(|value| remove_key(value, key));
        },
        Value::Array(values) => values.iter_mut().for_each(|value| remove_key(value, key)),
        _ => {},
    }
}

impl Reference {
    fn new(text: &str) -> Self {
        let json: Value = serde_json::from_str(text).unwrap();
        let header = serde_json::from_value(json.clone()).unwrap();
        let mut leaves = Vec::new();
        collect_leaves(&json, String::new(), &mut leaves);
        Self {
            header,
            json,
            leaves,
        }
    }

    fn apply(&self, json: &mut Value, edit: &JsonEdit) {
        let leaf = |i: u16| self.leaves[usize::from(i) % self.leaves.len()].as_str();
        match *edit {
            JsonEdit::Retag { leaf: i, tag } => {
                edit_tagged(json, leaf(i), |tb64| {
                    TaggedBase64::new(TAGS[usize::from(tag) % TAGS.len()], &tb64.value()).ok()
                });
            },
            JsonEdit::FlipBody { leaf: i, at, xor } => {
                edit_tagged(json, leaf(i), |tb64| {
                    let mut body = tb64.value();
                    if body.is_empty() {
                        return None;
                    }
                    let at = usize::from(at) % body.len();
                    body[at] ^= xor;
                    TaggedBase64::new(&tb64.tag(), &body).ok()
                });
            },
            JsonEdit::SetNumber { leaf: i, value } => {
                if let Some(slot) = json.pointer_mut(leaf(i)) {
                    *slot = value.into();
                }
            },
            JsonEdit::Null { leaf: i } => {
                if let Some(slot) = json.pointer_mut(leaf(i)) {
                    *slot = Value::Null;
                }
            },
            JsonEdit::Copy { from, to } => {
                let Some(value) = json.pointer(leaf(from)).cloned() else {
                    return;
                };
                if let Some(slot) = json.pointer_mut(leaf(to)) {
                    *slot = value;
                }
            },
        }
    }
}

/// Replace the TaggedBase64 string at `pointer` with `edit` of it, if there is
/// one and `edit` gives a value.
fn edit_tagged(
    json: &mut Value,
    pointer: &str,
    edit: impl FnOnce(&TaggedBase64) -> Option<TaggedBase64>,
) {
    let Some(Value::String(text)) = json.pointer_mut(pointer) else {
        return;
    };
    if let Some(edited) = TaggedBase64::parse(text).ok().and_then(|tb64| edit(&tb64)) {
        *text = edited.to_string();
    }
}

fn collect_leaves(json: &Value, pointer: String, leaves: &mut Vec<String>) {
    match json {
        Value::Object(map) => {
            for (key, value) in map {
                let key = key.replace('~', "~0").replace('/', "~1");
                collect_leaves(value, format!("{pointer}/{key}"), leaves);
            }
        },
        Value::Array(values) => {
            for (i, value) in values.iter().enumerate() {
                collect_leaves(value, format!("{pointer}/{i}"), leaves);
            }
        },
        _ => leaves.push(pointer),
    }
}
