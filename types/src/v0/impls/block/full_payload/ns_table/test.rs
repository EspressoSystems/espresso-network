use hotshot::traits::BlockPayload;
use rand::{Rng, RngCore};
use sequencer_utils::test_utils::setup_test;

use crate::{
    v0::impls::block::{
        test::ValidTest,
        uint_bytes::{u32_max_from_byte_len, usize_max_from_byte_len, usize_to_bytes},
    },
    v0_1::{
        NsTableBuilder,
        NsTableValidationError::{self, *},
        NS_ID_BYTE_LEN, NS_OFFSET_BYTE_LEN, NUM_NSS_BYTE_LEN,
    },
    NamespaceId, NsTable, Payload,
};

#[test]
fn random_valid() {
    setup_test();
    let mut rng = jf_utils::test_rng();

    for num_entries in 0..20 {
        expect_valid(&random_valid_ns_table(num_entries, &mut rng));
    }
}

#[test]
fn ns_table_from_bytes() {
    let bytes = Vec::from([0; NUM_NSS_BYTE_LEN]);
    let ns_table = NsTable::from_bytes_unchecked(&bytes);
    expect_valid(&ns_table);
}

#[test]
fn ns_table_byte_len() {
    setup_test();
    let mut rng = jf_utils::test_rng();

    // Extremely small byte lengths should get rejected.
    {
        let mut ns_table = NsTable { bytes: Vec::new() };
        expect_invalid(&ns_table, InvalidByteLen);
        expect_num_bytes_invalid(&mut ns_table, NsTableBuilder::header_byte_len(), &mut rng);
    }

    // Add enough bytes for a new entry.
    {
        let mut ns_table = random_valid_ns_table(20, &mut rng);
        expect_num_bytes_invalid(&mut ns_table, NsTableBuilder::entry_byte_len(), &mut rng);
    }

    // Helper fn: add 1 byte to the `ns_table` `num_bytes` times. Expect
    // invalidity in all but the final time.
    fn expect_num_bytes_invalid<R>(ns_table: &mut NsTable, num_bytes: usize, rng: &mut R)
    where
        R: RngCore,
    {
        for i in 0..num_bytes {
            ns_table.bytes.push(rng.r#gen());
            if i == num_bytes - 1 {
                break; // final iteration: no error expected
            }
            expect_invalid(ns_table, InvalidByteLen);
        }
        expect_invalid(ns_table, InvalidHeader);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn payload_byte_len() {
    setup_test();
    let test_case = vec![vec![5, 8, 8], vec![7, 9, 11], vec![10, 5, 8]];
    let mut rng = jf_utils::test_rng();
    let test = ValidTest::from_tx_lengths(test_case, &mut rng);
    let mut block =
        Payload::from_transactions(test.all_txs(), &Default::default(), &Default::default())
            .await
            .unwrap()
            .0;
    let payload_byte_len = block.byte_len();
    let final_offset = block
        .ns_table()
        .read_ns_offset_unchecked(&block.ns_table().iter().last().unwrap());

    // final offset matches payload byte len
    block.ns_table().validate(&payload_byte_len).unwrap();

    // Helper closure fn: modify the final offset of `block`'s namespace table
    // by adding `diff` to it. Assert failure.
    let mut modify_final_offset = |diff: isize| {
        let ns_table_byte_len = block.ns_table().bytes.len();
        let old_final_offset: isize = final_offset.try_into().unwrap();
        let new_final_offset: usize = (old_final_offset + diff).try_into().unwrap();

        block.ns_table_mut().bytes[ns_table_byte_len - NS_OFFSET_BYTE_LEN..]
            .copy_from_slice(&usize_to_bytes::<NS_OFFSET_BYTE_LEN>(new_final_offset));
        assert_eq!(
            block.ns_table().validate(&payload_byte_len).unwrap_err(),
            InvalidFinalOffset
        );
    };

    // final offset exceeds payload byte len
    modify_final_offset(1);

    // final offset less than payload byte len
    modify_final_offset(-1);

    // zero-length payload
    let empty_block = Payload::from_transactions([], &Default::default(), &Default::default())
        .await
        .unwrap()
        .0;
    assert_eq!(empty_block.ns_table().len().0, 0);
    assert_eq!(
        empty_block.ns_table().bytes,
        usize_to_bytes::<NUM_NSS_BYTE_LEN>(0)
    );
    empty_block
        .ns_table()
        .validate(&empty_block.byte_len())
        .unwrap();

    // empty namespace table with nonempty payload
    *block.ns_table_mut() = empty_block.ns_table().clone();
    assert_eq!(
        block.ns_table().validate(&payload_byte_len).unwrap_err(),
        ExpectNonemptyNsTable
    );
}

#[test]
fn monotonic_increase() {
    setup_test();

    // Duplicate namespace ID
    two_entries_ns_table((5, 5), (5, 6), Some(DuplicateNamespaceId));

    // Decreasing namespace ID
    two_entries_ns_table((5, 5), (4, 6), None);

    // Duplicate offset
    two_entries_ns_table((5, 5), (6, 5), Some(NonIncreasingEntries));

    // Decreasing offset
    two_entries_ns_table((5, 5), (6, 4), Some(NonIncreasingEntries));

    // Zero namespace ID
    two_entries_ns_table((0, 5), (6, 6), None);

    // Zero offset
    two_entries_ns_table((5, 0), (6, 6), Some(NonIncreasingEntries));

    // Helper fn: build a 2-entry NsTable, assert failure
    fn two_entries_ns_table(
        entry1: (u32, usize),
        entry2: (u32, usize),
        expect_err: Option<NsTableValidationError>,
    ) {
        let mut ns_table_builder = NsTableBuilder::new();
        ns_table_builder.append_entry(NamespaceId::from(entry1.0), entry1.1);
        ns_table_builder.append_entry(NamespaceId::from(entry2.0), entry2.1);
        let ns_table = ns_table_builder.into_ns_table();
        if let Some(err) = expect_err {
            expect_invalid(&ns_table, err);
        } else {
            expect_valid(&ns_table);
        }
    }
}

// TODO this test obsolete after
// https://github.com/EspressoSystems/espresso-sequencer/issues/1604
#[test]
fn header() {
    setup_test();
    let mut rng = jf_utils::test_rng();

    for num_entries in 0..20 {
        let mut ns_table = random_valid_ns_table(num_entries, &mut rng);
        if num_entries != 0 {
            set_header(&mut ns_table, 0);
            set_header(&mut ns_table, num_entries - 1);
        }
        set_header(&mut ns_table, num_entries + 1);
        set_header(&mut ns_table, usize_max_from_byte_len(NUM_NSS_BYTE_LEN));
    }

    // Helper fn: set the header of `ns_table` to declare `num_nss` entries,
    // assert failure.
    fn set_header(ns_table: &mut NsTable, num_nss: usize) {
        ns_table.bytes[..NUM_NSS_BYTE_LEN]
            .copy_from_slice(&usize_to_bytes::<NUM_NSS_BYTE_LEN>(num_nss));
        expect_invalid(ns_table, InvalidHeader);
    }
}

fn random_valid_ns_table<R>(num_entries: usize, rng: &mut R) -> NsTable
where
    R: RngCore,
{
    let (offset_max_increment, ns_id_max_increment) = if num_entries == 0 {
        (0, 0)
    } else {
        let num_entries_u32: u32 = num_entries.try_into().unwrap();
        (
            usize_max_from_byte_len(NS_OFFSET_BYTE_LEN) / num_entries,
            u32_max_from_byte_len(NS_ID_BYTE_LEN) / num_entries_u32,
        )
    };

    let mut ns_id = 0;
    let mut offset = 0;
    let mut ns_table_builder = NsTableBuilder::new();
    for _ in 0..num_entries {
        // ns_id, offset must increase monotonically
        ns_id += rng.gen_range(1..=ns_id_max_increment);
        offset += rng.gen_range(1..=offset_max_increment);
        ns_table_builder.append_entry(NamespaceId::from(ns_id), offset);
    }
    ns_table_builder.into_ns_table()
}

fn expect_valid(ns_table: &NsTable) {
    // `validate` should succeed
    ns_table.validate_deserialization_invariants().unwrap();

    // serde round-trip should succeed
    let serde_bytes = bincode::serialize(ns_table).unwrap();
    let ns_table_serde: NsTable = bincode::deserialize(&serde_bytes).unwrap();
    assert_eq!(&ns_table_serde, ns_table);
}

fn expect_invalid(ns_table: &NsTable, err: NsTableValidationError) {
    use serde::de::Error;

    // `validate` should fail
    assert_eq!(
        ns_table.validate_deserialization_invariants().unwrap_err(),
        err
    );

    // serde round-trip should fail
    //
    // need to use `to_string` because `bincode::Error`` is not `Eq`
    let serde_bytes = bincode::serialize(ns_table).unwrap();
    assert_eq!(
        bincode::deserialize::<NsTable>(&serde_bytes)
            .unwrap_err()
            .to_string(),
        bincode::Error::custom(err).to_string(),
    );
}
