use crate::{
    v0::impls::block::{usize_max_from_byte_len, usize_to_bytes},
    NamespaceId, NsPayloadBuilder, NsPayloadOwned,
};

#[test_log::test]
fn ns_payload_len() {
    let ns_id = NamespaceId::from(69_u32); // dummy

    // ordinary valid ns_payload
    {
        let ns_payload = NsPayloadOwned::entries_body(&[10, 20, 30], 30);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(txs[0].payload(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(txs[1].payload(), [10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
        assert_eq!(txs[2].payload(), [20, 21, 22, 23, 24, 25, 26, 27, 28, 29]);
    }

    // large payload has wasted space
    {
        let ns_payload = NsPayloadOwned::entries_body(&[10, 20, 30], 40);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(txs[0].payload(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(txs[1].payload(), [10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
        assert_eq!(txs[2].payload(), [20, 21, 22, 23, 24, 25, 26, 27, 28, 29]);

        // inaccessible payload bytes
        assert_eq!(ns_payload.0.len(), 56);
        assert_eq!(ns_payload.0[46..], [30, 31, 32, 33, 34, 35, 36, 37, 38, 39]);
    }

    // small payload truncates txs
    {
        // final tx partly truncated by short payload
        let ns_payload = NsPayloadOwned::entries_body(&[10, 20, 30], 25);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(txs[0].payload(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(txs[1].payload(), [10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
        assert_eq!(txs[2].payload(), [20, 21, 22, 23, 24]);

        // final tx totally truncated, next-to-final partly truncated
        let ns_payload = NsPayloadOwned::entries_body(&[10, 20, 30], 15);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(txs[0].payload(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(txs[1].payload(), [10, 11, 12, 13, 14]);
        assert!(txs[2].payload().is_empty());

        // all txs totally truncated
        let ns_payload = NsPayloadOwned::entries_body(&[10, 20, 30], 0);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert!(txs[0].payload().is_empty());
        assert!(txs[1].payload().is_empty());
        assert!(txs[2].payload().is_empty());
    }

    // small payload can't fit the whole tx table
    {
        // final tx table entry partly truncated by short payload
        let ns_payload = NsPayloadOwned::entries_total(&[10, 20, 30], tx_table_byte_len(3) - 1);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].payload().len(), 0);
        assert_eq!(txs[1].payload().len(), 0);

        // final tx table entry totally truncated, next-to-final partly truncated
        let ns_payload = NsPayloadOwned::entries_total(&[10, 20, 30], tx_table_byte_len(2) - 1);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].payload().len(), 0);

        // all tx table entries totally truncated (tx table header only)
        let ns_payload = NsPayloadOwned::entries_total(
            &[10, 20, 30],
            NsPayloadBuilder::tx_table_header_byte_len(),
        );
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 0);
    }

    // extremely small payload can't even fit the tx table header
    {
        for i in 0..NsPayloadBuilder::tx_table_header_byte_len() {
            let ns_payload = NsPayloadOwned::entries_total(&[10, 20, 30], i);
            assert_eq!(ns_payload.iter().count(), 0);
        }
    }
}

#[test_log::test]
fn negative_len_txs() {
    let ns_id = NamespaceId::from(69_u32); // dummy

    // 1 negative-length tx at the end, no overlapping tx bytes
    {
        let ns_payload = NsPayloadOwned::entries_body(&[10, 30, 20], 30);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(txs[0].payload(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(
            txs[1].payload(),
            [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29]
        );
        assert!(txs[2].payload().is_empty());
    }

    // 1 negative-length tx in the middle, overlapping tx bytes
    {
        let ns_payload = NsPayloadOwned::entries_body(&[20, 10, 30], 30);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(
            txs[0].payload(),
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
        );
        assert!(txs[1].payload().is_empty());
        assert_eq!(
            txs[2].payload(),
            [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29]
        );
    }

    // 1 negative-length tx in the middle, one tx contains all others
    {
        let ns_payload = NsPayloadOwned::entries_body(&[30, 10, 20], 30);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(
            txs[0].payload(),
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29
            ]
        );
        assert!(txs[1].payload().is_empty());
        assert_eq!(txs[2].payload(), [10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
    }

    // all txs negative-length except the first
    {
        let ns_payload = NsPayloadOwned::entries_body(&[30, 20, 10], 30);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(
            txs[0].payload(),
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29
            ]
        );
        assert!(txs[1].payload().is_empty());
        assert!(txs[2].payload().is_empty());
    }
}

#[test_log::test]
fn negative_len_txs_and_abnormal_payload_len() {
    let ns_id = NamespaceId::from(69_u32); // dummy

    // 1 negative-length tx in the middle, overlapping tx bytes
    // final tx partly truncated by short payload
    {
        let ns_payload = NsPayloadOwned::entries_body(&[20, 10, 30], 25);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(
            txs[0].payload(),
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
        );
        assert!(txs[1].payload().is_empty());
        assert_eq!(
            txs[2].payload(),
            [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24]
        );
    }

    // 1 negative-length tx in the middle, overlapping tx bytes
    // large payload has wasted space
    {
        let ns_payload = NsPayloadOwned::entries_body(&[20, 10, 30], 40);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(
            txs[0].payload(),
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
        );
        assert!(txs[1].payload().is_empty());
        assert_eq!(
            txs[2].payload(),
            [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29]
        );
    }

    // all txs negative-length except the first
    // tx partly truncated by short payload
    {
        let ns_payload = NsPayloadOwned::entries_body(&[30, 20, 10], 25);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(
            txs[0].payload(),
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24
            ]
        );
        assert!(txs[1].payload().is_empty());
        assert!(txs[2].payload().is_empty());
    }

    // all txs negative-length except the first
    // large payload has wasted space
    {
        let ns_payload = NsPayloadOwned::entries_body(&[30, 20, 10], 40);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 3);
        assert_eq!(
            txs[0].payload(),
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29
            ]
        );
        assert!(txs[1].payload().is_empty());
        assert!(txs[2].payload().is_empty());
    }
}

#[test_log::test]
fn tx_table_header() {
    let ns_id = NamespaceId::from(69_u32); // dummy

    // header declares 1 fewer txs, tx table bytes appear in tx payloads, wasted
    // payload bytes
    {
        let ns_payload = NsPayloadOwned::header_entries_body(2, &[10, 20, 30], 30);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 2);

        // first tx contains 4 bytes from final tx table entry [30,0,0,0] then 6
        // bytes from payload [0,1,2,3,4,5]
        assert_eq!(txs[0].payload(), [30, 0, 0, 0, 0, 1, 2, 3, 4, 5]);
        assert_eq!(txs[1].payload(), [6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);

        // inaccessible payload bytes
        assert_eq!(ns_payload.0.len(), 46);
        assert_eq!(
            ns_payload.0[32..],
            [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29]
        );
    }

    // header declares 0 txs, all payload bytes are wasted
    {
        let ns_payload = NsPayloadOwned::header_entries_body(0, &[10, 20, 30], 30);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 0);
    }

    // header declares 1 extra tx, payload bytes appear in tx table, payload is
    // now too small
    {
        let ns_payload = NsPayloadOwned::header_entries_body(4, &[10, 20, 30], 30);
        let txs = ns_payload.export_all_txs(&ns_id);
        assert_eq!(txs.len(), 4);

        // first tx starts after the final tx table entry containing bytes
        // [0,1,2,3]
        assert_eq!(txs[0].payload(), [4, 5, 6, 7, 8, 9, 10, 11, 12, 13]);

        assert_eq!(txs[1].payload(), [14, 15, 16, 17, 18, 19, 20, 21, 22, 23]);

        // 3rd tx is truncated by small payload size
        assert_eq!(txs[2].payload(), [24, 25, 26, 27, 28, 29]);

        // 4th tx completely truncated
        assert!(txs[3].payload().is_empty());
    }

    // header declares large number of txs, tx table cannot fit in payload, all txs 0-length
    {
        let ns_payload = NsPayloadOwned::header_entries_body(
            usize_max_from_byte_len(NsPayloadBuilder::tx_table_header_byte_len()),
            &[10, 20, 30],
            30,
        );
        let expected_payload_byte_len = 46;
        assert_eq!(ns_payload.0.len(), expected_payload_byte_len);

        let txs = ns_payload.export_all_txs(&ns_id);
        let expected_num_txs = (expected_payload_byte_len
            - NsPayloadBuilder::tx_table_header_byte_len())
            / NsPayloadBuilder::tx_table_entry_byte_len();
        assert_eq!(txs.len(), expected_num_txs);
        for tx in txs {
            assert!(tx.payload().is_empty());
        }
    }
}

fn tx_table_with_header(header: usize, entries: &[usize]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(usize_to_bytes::<
        { NsPayloadBuilder::tx_table_header_byte_len() },
    >(header));
    for entry in entries {
        bytes.extend(usize_to_bytes::<
            { NsPayloadBuilder::tx_table_entry_byte_len() },
        >(*entry));
    }
    bytes
}

fn ns_payload_body(body_byte_len: usize) -> Vec<u8> {
    (0..body_byte_len)
        .map(|i| (i % u8::MAX as usize) as u8)
        .collect()
}

fn tx_table_byte_len(num_entries: usize) -> usize {
    NsPayloadBuilder::tx_table_header_byte_len()
        + num_entries * NsPayloadBuilder::tx_table_entry_byte_len()
}

impl NsPayloadOwned {
    fn entries_body(entries: &[usize], body_byte_len: usize) -> Self {
        Self::header_entries_body(entries.len(), entries, body_byte_len)
    }
    fn header_entries_body(header: usize, entries: &[usize], body_byte_len: usize) -> Self {
        Self::header_entries_total(
            header,
            entries,
            body_byte_len + tx_table_byte_len(entries.len()),
        )
    }
    fn entries_total(entries: &[usize], total_byte_len: usize) -> Self {
        Self::header_entries_total(entries.len(), entries, total_byte_len)
    }
    fn header_entries_total(header: usize, entries: &[usize], total_byte_len: usize) -> Self {
        let mut bytes = tx_table_with_header(header, entries);
        if total_byte_len > bytes.len() {
            bytes.append(&mut ns_payload_body(total_byte_len - bytes.len()));
        } else {
            bytes.truncate(total_byte_len);
        }
        Self(bytes)
    }
}
