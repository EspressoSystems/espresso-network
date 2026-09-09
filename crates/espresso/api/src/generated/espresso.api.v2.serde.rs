impl serde::Serialize for AdvzCommon {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.poly_commits.is_empty() {
            len += 1;
        }
        if !self.all_evals_digest.is_empty() {
            len += 1;
        }
        if self.payload_byte_len != 0 {
            len += 1;
        }
        if self.num_storage_nodes != 0 {
            len += 1;
        }
        if self.multiplicity != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AdvzCommon", len)?;
        if !self.poly_commits.is_empty() {
            struct_ser.serialize_field("polyCommits", &self.poly_commits)?;
        }
        if !self.all_evals_digest.is_empty() {
            struct_ser.serialize_field("allEvalsDigest", &self.all_evals_digest)?;
        }
        if self.payload_byte_len != 0 {
            struct_ser.serialize_field("payloadByteLen", &self.payload_byte_len)?;
        }
        if self.num_storage_nodes != 0 {
            struct_ser.serialize_field("numStorageNodes", &self.num_storage_nodes)?;
        }
        if self.multiplicity != 0 {
            struct_ser.serialize_field("multiplicity", &self.multiplicity)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AdvzCommon {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "poly_commits",
            "polyCommits",
            "all_evals_digest",
            "allEvalsDigest",
            "payload_byte_len",
            "payloadByteLen",
            "num_storage_nodes",
            "numStorageNodes",
            "multiplicity",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PolyCommits,
            AllEvalsDigest,
            PayloadByteLen,
            NumStorageNodes,
            Multiplicity,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "polyCommits" | "poly_commits" => Ok(GeneratedField::PolyCommits),
                            "allEvalsDigest" | "all_evals_digest" => Ok(GeneratedField::AllEvalsDigest),
                            "payloadByteLen" | "payload_byte_len" => Ok(GeneratedField::PayloadByteLen),
                            "numStorageNodes" | "num_storage_nodes" => Ok(GeneratedField::NumStorageNodes),
                            "multiplicity" => Ok(GeneratedField::Multiplicity),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AdvzCommon;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AdvzCommon")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AdvzCommon, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut poly_commits__ = None;
                let mut all_evals_digest__ = None;
                let mut payload_byte_len__ = None;
                let mut num_storage_nodes__ = None;
                let mut multiplicity__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PolyCommits => {
                            if poly_commits__.is_some() {
                                return Err(serde::de::Error::duplicate_field("polyCommits"));
                            }
                            poly_commits__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AllEvalsDigest => {
                            if all_evals_digest__.is_some() {
                                return Err(serde::de::Error::duplicate_field("allEvalsDigest"));
                            }
                            all_evals_digest__ = Some(map_.next_value()?);
                        }
                        GeneratedField::PayloadByteLen => {
                            if payload_byte_len__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadByteLen"));
                            }
                            payload_byte_len__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NumStorageNodes => {
                            if num_storage_nodes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("numStorageNodes"));
                            }
                            num_storage_nodes__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Multiplicity => {
                            if multiplicity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiplicity"));
                            }
                            multiplicity__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(AdvzCommon {
                    poly_commits: poly_commits__.unwrap_or_default(),
                    all_evals_digest: all_evals_digest__.unwrap_or_default(),
                    payload_byte_len: payload_byte_len__.unwrap_or_default(),
                    num_storage_nodes: num_storage_nodes__.unwrap_or_default(),
                    multiplicity: multiplicity__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AdvzCommon", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AdvzNsProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.ns_index.is_empty() {
            len += 1;
        }
        if !self.ns_payload.is_empty() {
            len += 1;
        }
        if self.ns_proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AdvzNsProof", len)?;
        if !self.ns_index.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nsIndex", pbjson::private::base64::encode(&self.ns_index).as_str())?;
        }
        if !self.ns_payload.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nsPayload", pbjson::private::base64::encode(&self.ns_payload).as_str())?;
        }
        if let Some(v) = self.ns_proof.as_ref() {
            struct_ser.serialize_field("nsProof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AdvzNsProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ns_index",
            "nsIndex",
            "ns_payload",
            "nsPayload",
            "ns_proof",
            "nsProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            NsIndex,
            NsPayload,
            NsProof,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "nsIndex" | "ns_index" => Ok(GeneratedField::NsIndex),
                            "nsPayload" | "ns_payload" => Ok(GeneratedField::NsPayload),
                            "nsProof" | "ns_proof" => Ok(GeneratedField::NsProof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AdvzNsProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AdvzNsProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AdvzNsProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut ns_index__ = None;
                let mut ns_payload__ = None;
                let mut ns_proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::NsIndex => {
                            if ns_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsIndex"));
                            }
                            ns_index__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NsPayload => {
                            if ns_payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsPayload"));
                            }
                            ns_payload__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NsProof => {
                            if ns_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsProof"));
                            }
                            ns_proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(AdvzNsProof {
                    ns_index: ns_index__.unwrap_or_default(),
                    ns_payload: ns_payload__.unwrap_or_default(),
                    ns_proof: ns_proof__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AdvzNsProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AdvzTxProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.tx_index.is_empty() {
            len += 1;
        }
        if !self.payload_num_txs.is_empty() {
            len += 1;
        }
        if self.payload_proof_num_txs.is_some() {
            len += 1;
        }
        if !self.payload_tx_table_entries.is_empty() {
            len += 1;
        }
        if self.payload_proof_tx_table_entries.is_some() {
            len += 1;
        }
        if self.payload_proof_tx.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AdvzTxProof", len)?;
        if !self.tx_index.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("txIndex", pbjson::private::base64::encode(&self.tx_index).as_str())?;
        }
        if !self.payload_num_txs.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("payloadNumTxs", pbjson::private::base64::encode(&self.payload_num_txs).as_str())?;
        }
        if let Some(v) = self.payload_proof_num_txs.as_ref() {
            struct_ser.serialize_field("payloadProofNumTxs", v)?;
        }
        if !self.payload_tx_table_entries.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("payloadTxTableEntries", pbjson::private::base64::encode(&self.payload_tx_table_entries).as_str())?;
        }
        if let Some(v) = self.payload_proof_tx_table_entries.as_ref() {
            struct_ser.serialize_field("payloadProofTxTableEntries", v)?;
        }
        if let Some(v) = self.payload_proof_tx.as_ref() {
            struct_ser.serialize_field("payloadProofTx", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AdvzTxProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tx_index",
            "txIndex",
            "payload_num_txs",
            "payloadNumTxs",
            "payload_proof_num_txs",
            "payloadProofNumTxs",
            "payload_tx_table_entries",
            "payloadTxTableEntries",
            "payload_proof_tx_table_entries",
            "payloadProofTxTableEntries",
            "payload_proof_tx",
            "payloadProofTx",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TxIndex,
            PayloadNumTxs,
            PayloadProofNumTxs,
            PayloadTxTableEntries,
            PayloadProofTxTableEntries,
            PayloadProofTx,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "txIndex" | "tx_index" => Ok(GeneratedField::TxIndex),
                            "payloadNumTxs" | "payload_num_txs" => Ok(GeneratedField::PayloadNumTxs),
                            "payloadProofNumTxs" | "payload_proof_num_txs" => Ok(GeneratedField::PayloadProofNumTxs),
                            "payloadTxTableEntries" | "payload_tx_table_entries" => Ok(GeneratedField::PayloadTxTableEntries),
                            "payloadProofTxTableEntries" | "payload_proof_tx_table_entries" => Ok(GeneratedField::PayloadProofTxTableEntries),
                            "payloadProofTx" | "payload_proof_tx" => Ok(GeneratedField::PayloadProofTx),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AdvzTxProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AdvzTxProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AdvzTxProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tx_index__ = None;
                let mut payload_num_txs__ = None;
                let mut payload_proof_num_txs__ = None;
                let mut payload_tx_table_entries__ = None;
                let mut payload_proof_tx_table_entries__ = None;
                let mut payload_proof_tx__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TxIndex => {
                            if tx_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txIndex"));
                            }
                            tx_index__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PayloadNumTxs => {
                            if payload_num_txs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadNumTxs"));
                            }
                            payload_num_txs__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PayloadProofNumTxs => {
                            if payload_proof_num_txs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadProofNumTxs"));
                            }
                            payload_proof_num_txs__ = map_.next_value()?;
                        }
                        GeneratedField::PayloadTxTableEntries => {
                            if payload_tx_table_entries__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadTxTableEntries"));
                            }
                            payload_tx_table_entries__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PayloadProofTxTableEntries => {
                            if payload_proof_tx_table_entries__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadProofTxTableEntries"));
                            }
                            payload_proof_tx_table_entries__ = map_.next_value()?;
                        }
                        GeneratedField::PayloadProofTx => {
                            if payload_proof_tx__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadProofTx"));
                            }
                            payload_proof_tx__ = map_.next_value()?;
                        }
                    }
                }
                Ok(AdvzTxProof {
                    tx_index: tx_index__.unwrap_or_default(),
                    payload_num_txs: payload_num_txs__.unwrap_or_default(),
                    payload_proof_num_txs: payload_proof_num_txs__,
                    payload_tx_table_entries: payload_tx_table_entries__.unwrap_or_default(),
                    payload_proof_tx_table_entries: payload_proof_tx_table_entries__,
                    payload_proof_tx: payload_proof_tx__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AdvzTxProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AvidmBadEncodingNsProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.ns_index != 0 {
            len += 1;
        }
        if !self.ns_commit.is_empty() {
            len += 1;
        }
        if !self.ns_mt_proof.is_empty() {
            len += 1;
        }
        if self.ns_proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AvidmBadEncodingNsProof", len)?;
        if self.ns_index != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nsIndex", ToString::to_string(&self.ns_index).as_str())?;
        }
        if !self.ns_commit.is_empty() {
            struct_ser.serialize_field("nsCommit", &self.ns_commit)?;
        }
        if !self.ns_mt_proof.is_empty() {
            struct_ser.serialize_field("nsMtProof", &self.ns_mt_proof)?;
        }
        if let Some(v) = self.ns_proof.as_ref() {
            struct_ser.serialize_field("nsProof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AvidmBadEncodingNsProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ns_index",
            "nsIndex",
            "ns_commit",
            "nsCommit",
            "ns_mt_proof",
            "nsMtProof",
            "ns_proof",
            "nsProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            NsIndex,
            NsCommit,
            NsMtProof,
            NsProof,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "nsIndex" | "ns_index" => Ok(GeneratedField::NsIndex),
                            "nsCommit" | "ns_commit" => Ok(GeneratedField::NsCommit),
                            "nsMtProof" | "ns_mt_proof" => Ok(GeneratedField::NsMtProof),
                            "nsProof" | "ns_proof" => Ok(GeneratedField::NsProof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AvidmBadEncodingNsProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AvidmBadEncodingNsProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AvidmBadEncodingNsProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut ns_index__ = None;
                let mut ns_commit__ = None;
                let mut ns_mt_proof__ = None;
                let mut ns_proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::NsIndex => {
                            if ns_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsIndex"));
                            }
                            ns_index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NsCommit => {
                            if ns_commit__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsCommit"));
                            }
                            ns_commit__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NsMtProof => {
                            if ns_mt_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsMtProof"));
                            }
                            ns_mt_proof__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NsProof => {
                            if ns_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsProof"));
                            }
                            ns_proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(AvidmBadEncodingNsProof {
                    ns_index: ns_index__.unwrap_or_default(),
                    ns_commit: ns_commit__.unwrap_or_default(),
                    ns_mt_proof: ns_mt_proof__.unwrap_or_default(),
                    ns_proof: ns_proof__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AvidmBadEncodingNsProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AvidmBadEncodingProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.recovered_poly.is_empty() {
            len += 1;
        }
        if !self.raw_shares.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AvidmBadEncodingProof", len)?;
        if !self.recovered_poly.is_empty() {
            struct_ser.serialize_field("recoveredPoly", &self.recovered_poly)?;
        }
        if !self.raw_shares.is_empty() {
            struct_ser.serialize_field("rawShares", &self.raw_shares)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AvidmBadEncodingProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "recovered_poly",
            "recoveredPoly",
            "raw_shares",
            "rawShares",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RecoveredPoly,
            RawShares,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "recoveredPoly" | "recovered_poly" => Ok(GeneratedField::RecoveredPoly),
                            "rawShares" | "raw_shares" => Ok(GeneratedField::RawShares),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AvidmBadEncodingProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AvidmBadEncodingProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AvidmBadEncodingProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut recovered_poly__ = None;
                let mut raw_shares__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RecoveredPoly => {
                            if recovered_poly__.is_some() {
                                return Err(serde::de::Error::duplicate_field("recoveredPoly"));
                            }
                            recovered_poly__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RawShares => {
                            if raw_shares__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rawShares"));
                            }
                            raw_shares__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AvidmBadEncodingProof {
                    recovered_poly: recovered_poly__.unwrap_or_default(),
                    raw_shares: raw_shares__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AvidmBadEncodingProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AvidmCommon {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.total_weights != 0 {
            len += 1;
        }
        if self.recovery_threshold != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AvidmCommon", len)?;
        if self.total_weights != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("totalWeights", ToString::to_string(&self.total_weights).as_str())?;
        }
        if self.recovery_threshold != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("recoveryThreshold", ToString::to_string(&self.recovery_threshold).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AvidmCommon {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "total_weights",
            "totalWeights",
            "recovery_threshold",
            "recoveryThreshold",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TotalWeights,
            RecoveryThreshold,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "totalWeights" | "total_weights" => Ok(GeneratedField::TotalWeights),
                            "recoveryThreshold" | "recovery_threshold" => Ok(GeneratedField::RecoveryThreshold),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AvidmCommon;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AvidmCommon")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AvidmCommon, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut total_weights__ = None;
                let mut recovery_threshold__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TotalWeights => {
                            if total_weights__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalWeights"));
                            }
                            total_weights__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RecoveryThreshold => {
                            if recovery_threshold__.is_some() {
                                return Err(serde::de::Error::duplicate_field("recoveryThreshold"));
                            }
                            recovery_threshold__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(AvidmCommon {
                    total_weights: total_weights__.unwrap_or_default(),
                    recovery_threshold: recovery_threshold__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AvidmCommon", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AvidmGf2Common {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.param.is_some() {
            len += 1;
        }
        if !self.ns_commits.is_empty() {
            len += 1;
        }
        if !self.ns_lens.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AvidmGf2Common", len)?;
        if let Some(v) = self.param.as_ref() {
            struct_ser.serialize_field("param", v)?;
        }
        if !self.ns_commits.is_empty() {
            struct_ser.serialize_field("nsCommits", &self.ns_commits)?;
        }
        if !self.ns_lens.is_empty() {
            struct_ser.serialize_field("nsLens", &self.ns_lens.iter().map(ToString::to_string).collect::<Vec<_>>())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AvidmGf2Common {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "param",
            "ns_commits",
            "nsCommits",
            "ns_lens",
            "nsLens",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Param,
            NsCommits,
            NsLens,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "param" => Ok(GeneratedField::Param),
                            "nsCommits" | "ns_commits" => Ok(GeneratedField::NsCommits),
                            "nsLens" | "ns_lens" => Ok(GeneratedField::NsLens),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AvidmGf2Common;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AvidmGf2Common")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AvidmGf2Common, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut param__ = None;
                let mut ns_commits__ = None;
                let mut ns_lens__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Param => {
                            if param__.is_some() {
                                return Err(serde::de::Error::duplicate_field("param"));
                            }
                            param__ = map_.next_value()?;
                        }
                        GeneratedField::NsCommits => {
                            if ns_commits__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsCommits"));
                            }
                            ns_commits__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NsLens => {
                            if ns_lens__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsLens"));
                            }
                            ns_lens__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                    }
                }
                Ok(AvidmGf2Common {
                    param: param__,
                    ns_commits: ns_commits__.unwrap_or_default(),
                    ns_lens: ns_lens__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AvidmGf2Common", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AvidmGf2Param {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.total_weights != 0 {
            len += 1;
        }
        if self.recovery_threshold != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AvidmGf2Param", len)?;
        if self.total_weights != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("totalWeights", ToString::to_string(&self.total_weights).as_str())?;
        }
        if self.recovery_threshold != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("recoveryThreshold", ToString::to_string(&self.recovery_threshold).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AvidmGf2Param {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "total_weights",
            "totalWeights",
            "recovery_threshold",
            "recoveryThreshold",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TotalWeights,
            RecoveryThreshold,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "totalWeights" | "total_weights" => Ok(GeneratedField::TotalWeights),
                            "recoveryThreshold" | "recovery_threshold" => Ok(GeneratedField::RecoveryThreshold),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AvidmGf2Param;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AvidmGf2Param")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AvidmGf2Param, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut total_weights__ = None;
                let mut recovery_threshold__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TotalWeights => {
                            if total_weights__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalWeights"));
                            }
                            total_weights__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RecoveryThreshold => {
                            if recovery_threshold__.is_some() {
                                return Err(serde::de::Error::duplicate_field("recoveryThreshold"));
                            }
                            recovery_threshold__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(AvidmGf2Param {
                    total_weights: total_weights__.unwrap_or_default(),
                    recovery_threshold: recovery_threshold__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AvidmGf2Param", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AvidmGf2TxProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.tx_index.is_empty() {
            len += 1;
        }
        if self.ns_proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AvidmGf2TxProof", len)?;
        if !self.tx_index.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("txIndex", pbjson::private::base64::encode(&self.tx_index).as_str())?;
        }
        if let Some(v) = self.ns_proof.as_ref() {
            struct_ser.serialize_field("nsProof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AvidmGf2TxProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tx_index",
            "txIndex",
            "ns_proof",
            "nsProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TxIndex,
            NsProof,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "txIndex" | "tx_index" => Ok(GeneratedField::TxIndex),
                            "nsProof" | "ns_proof" => Ok(GeneratedField::NsProof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AvidmGf2TxProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AvidmGf2TxProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AvidmGf2TxProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tx_index__ = None;
                let mut ns_proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TxIndex => {
                            if tx_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txIndex"));
                            }
                            tx_index__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NsProof => {
                            if ns_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsProof"));
                            }
                            ns_proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(AvidmGf2TxProof {
                    tx_index: tx_index__.unwrap_or_default(),
                    ns_proof: ns_proof__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AvidmGf2TxProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AvidmTxProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.tx_index.is_empty() {
            len += 1;
        }
        if self.ns_proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AvidmTxProof", len)?;
        if !self.tx_index.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("txIndex", pbjson::private::base64::encode(&self.tx_index).as_str())?;
        }
        if let Some(v) = self.ns_proof.as_ref() {
            struct_ser.serialize_field("nsProof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AvidmTxProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tx_index",
            "txIndex",
            "ns_proof",
            "nsProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TxIndex,
            NsProof,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "txIndex" | "tx_index" => Ok(GeneratedField::TxIndex),
                            "nsProof" | "ns_proof" => Ok(GeneratedField::NsProof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AvidmTxProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AvidmTxProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AvidmTxProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tx_index__ = None;
                let mut ns_proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TxIndex => {
                            if tx_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txIndex"));
                            }
                            tx_index__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NsProof => {
                            if ns_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsProof"));
                            }
                            ns_proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(AvidmTxProof {
                    tx_index: tx_index__.unwrap_or_default(),
                    ns_proof: ns_proof__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AvidmTxProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlsPublicKey {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.key.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.BLSPublicKey", len)?;
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlsPublicKey {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlsPublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.BLSPublicKey")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BlsPublicKey, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BlsPublicKey {
                    key: key__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.BLSPublicKey", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlockHeightResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.BlockHeightResponse", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockHeightResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlockHeightResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.BlockHeightResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BlockHeightResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(BlockHeightResponse {
                    height: height__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.BlockHeightResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlockRangeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.blocks.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.BlockRangeResponse", len)?;
        if !self.blocks.is_empty() {
            struct_ser.serialize_field("blocks", &self.blocks)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockRangeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "blocks",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Blocks,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "blocks" => Ok(GeneratedField::Blocks),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlockRangeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.BlockRangeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BlockRangeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut blocks__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Blocks => {
                            if blocks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blocks"));
                            }
                            blocks__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BlockRangeResponse {
                    blocks: blocks__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.BlockRangeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlockResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.header.is_some() {
            len += 1;
        }
        if self.payload.is_some() {
            len += 1;
        }
        if !self.hash.is_empty() {
            len += 1;
        }
        if self.size != 0 {
            len += 1;
        }
        if self.num_transactions != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.BlockResponse", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if let Some(v) = self.payload.as_ref() {
            struct_ser.serialize_field("payload", v)?;
        }
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", &self.hash)?;
        }
        if self.size != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("size", ToString::to_string(&self.size).as_str())?;
        }
        if self.num_transactions != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("numTransactions", ToString::to_string(&self.num_transactions).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "payload",
            "hash",
            "size",
            "num_transactions",
            "numTransactions",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            Payload,
            Hash,
            Size,
            NumTransactions,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "header" => Ok(GeneratedField::Header),
                            "payload" => Ok(GeneratedField::Payload),
                            "hash" => Ok(GeneratedField::Hash),
                            "size" => Ok(GeneratedField::Size),
                            "numTransactions" | "num_transactions" => Ok(GeneratedField::NumTransactions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlockResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.BlockResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BlockResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut payload__ = None;
                let mut hash__ = None;
                let mut size__ = None;
                let mut num_transactions__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ = map_.next_value()?;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Size => {
                            if size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("size"));
                            }
                            size__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NumTransactions => {
                            if num_transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("numTransactions"));
                            }
                            num_transactions__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(BlockResponse {
                    header: header__,
                    payload: payload__,
                    hash: hash__.unwrap_or_default(),
                    size: size__.unwrap_or_default(),
                    num_transactions: num_transactions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.BlockResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlockRewardResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.amount.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.BlockRewardResponse", len)?;
        if let Some(v) = self.amount.as_ref() {
            struct_ser.serialize_field("amount", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockRewardResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "amount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Amount,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "amount" => Ok(GeneratedField::Amount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlockRewardResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.BlockRewardResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BlockRewardResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut amount__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = map_.next_value()?;
                        }
                    }
                }
                Ok(BlockRewardResponse {
                    amount: amount__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.BlockRewardResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlockSummaryRangeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.summaries.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.BlockSummaryRangeResponse", len)?;
        if !self.summaries.is_empty() {
            struct_ser.serialize_field("summaries", &self.summaries)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockSummaryRangeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "summaries",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Summaries,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "summaries" => Ok(GeneratedField::Summaries),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlockSummaryRangeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.BlockSummaryRangeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BlockSummaryRangeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut summaries__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Summaries => {
                            if summaries__.is_some() {
                                return Err(serde::de::Error::duplicate_field("summaries"));
                            }
                            summaries__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BlockSummaryRangeResponse {
                    summaries: summaries__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.BlockSummaryRangeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlockSummaryResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.header.is_some() {
            len += 1;
        }
        if !self.hash.is_empty() {
            len += 1;
        }
        if self.size != 0 {
            len += 1;
        }
        if self.num_transactions != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.BlockSummaryResponse", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", &self.hash)?;
        }
        if self.size != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("size", ToString::to_string(&self.size).as_str())?;
        }
        if self.num_transactions != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("numTransactions", ToString::to_string(&self.num_transactions).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockSummaryResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "hash",
            "size",
            "num_transactions",
            "numTransactions",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            Hash,
            Size,
            NumTransactions,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "header" => Ok(GeneratedField::Header),
                            "hash" => Ok(GeneratedField::Hash),
                            "size" => Ok(GeneratedField::Size),
                            "numTransactions" | "num_transactions" => Ok(GeneratedField::NumTransactions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlockSummaryResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.BlockSummaryResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BlockSummaryResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut hash__ = None;
                let mut size__ = None;
                let mut num_transactions__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Size => {
                            if size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("size"));
                            }
                            size__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NumTransactions => {
                            if num_transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("numTransactions"));
                            }
                            num_transactions__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(BlockSummaryResponse {
                    header: header__,
                    hash: hash__.unwrap_or_default(),
                    size: size__.unwrap_or_default(),
                    num_transactions: num_transactions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.BlockSummaryResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BuilderSignature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.r.is_empty() {
            len += 1;
        }
        if !self.s.is_empty() {
            len += 1;
        }
        if self.v != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.BuilderSignature", len)?;
        if !self.r.is_empty() {
            struct_ser.serialize_field("r", &self.r)?;
        }
        if !self.s.is_empty() {
            struct_ser.serialize_field("s", &self.s)?;
        }
        if self.v != 0 {
            struct_ser.serialize_field("v", &self.v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BuilderSignature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "r",
            "s",
            "v",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            R,
            S,
            V,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "r" => Ok(GeneratedField::R),
                            "s" => Ok(GeneratedField::S),
                            "v" => Ok(GeneratedField::V),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BuilderSignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.BuilderSignature")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BuilderSignature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r__ = None;
                let mut s__ = None;
                let mut v__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::R => {
                            if r__.is_some() {
                                return Err(serde::de::Error::duplicate_field("r"));
                            }
                            r__ = Some(map_.next_value()?);
                        }
                        GeneratedField::S => {
                            if s__.is_some() {
                                return Err(serde::de::Error::duplicate_field("s"));
                            }
                            s__ = Some(map_.next_value()?);
                        }
                        GeneratedField::V => {
                            if v__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v"));
                            }
                            v__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(BuilderSignature {
                    r: r__.unwrap_or_default(),
                    s: s__.unwrap_or_default(),
                    v: v__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.BuilderSignature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Certificate2 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.data.is_some() {
            len += 1;
        }
        if !self.vote_commitment.is_empty() {
            len += 1;
        }
        if self.view_number != 0 {
            len += 1;
        }
        if self.signatures.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.Certificate2", len)?;
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        if !self.vote_commitment.is_empty() {
            struct_ser.serialize_field("voteCommitment", &self.vote_commitment)?;
        }
        if self.view_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("viewNumber", ToString::to_string(&self.view_number).as_str())?;
        }
        if let Some(v) = self.signatures.as_ref() {
            struct_ser.serialize_field("signatures", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Certificate2 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "data",
            "vote_commitment",
            "voteCommitment",
            "view_number",
            "viewNumber",
            "signatures",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Data,
            VoteCommitment,
            ViewNumber,
            Signatures,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "data" => Ok(GeneratedField::Data),
                            "voteCommitment" | "vote_commitment" => Ok(GeneratedField::VoteCommitment),
                            "viewNumber" | "view_number" => Ok(GeneratedField::ViewNumber),
                            "signatures" => Ok(GeneratedField::Signatures),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Certificate2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.Certificate2")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Certificate2, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut data__ = None;
                let mut vote_commitment__ = None;
                let mut view_number__ = None;
                let mut signatures__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map_.next_value()?;
                        }
                        GeneratedField::VoteCommitment => {
                            if vote_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("voteCommitment"));
                            }
                            vote_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ViewNumber => {
                            if view_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("viewNumber"));
                            }
                            view_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Certificate2 {
                    data: data__,
                    vote_commitment: vote_commitment__.unwrap_or_default(),
                    view_number: view_number__.unwrap_or_default(),
                    signatures: signatures__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.Certificate2", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ChainConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.chain_id.is_empty() {
            len += 1;
        }
        if self.max_block_size != 0 {
            len += 1;
        }
        if !self.base_fee.is_empty() {
            len += 1;
        }
        if self.fee_contract.is_some() {
            len += 1;
        }
        if !self.fee_recipient.is_empty() {
            len += 1;
        }
        if self.stake_table_contract.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.ChainConfig", len)?;
        if !self.chain_id.is_empty() {
            struct_ser.serialize_field("chainId", &self.chain_id)?;
        }
        if self.max_block_size != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("maxBlockSize", ToString::to_string(&self.max_block_size).as_str())?;
        }
        if !self.base_fee.is_empty() {
            struct_ser.serialize_field("baseFee", &self.base_fee)?;
        }
        if let Some(v) = self.fee_contract.as_ref() {
            struct_ser.serialize_field("feeContract", v)?;
        }
        if !self.fee_recipient.is_empty() {
            struct_ser.serialize_field("feeRecipient", &self.fee_recipient)?;
        }
        if let Some(v) = self.stake_table_contract.as_ref() {
            struct_ser.serialize_field("stakeTableContract", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ChainConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_id",
            "chainId",
            "max_block_size",
            "maxBlockSize",
            "base_fee",
            "baseFee",
            "fee_contract",
            "feeContract",
            "fee_recipient",
            "feeRecipient",
            "stake_table_contract",
            "stakeTableContract",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainId,
            MaxBlockSize,
            BaseFee,
            FeeContract,
            FeeRecipient,
            StakeTableContract,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "chainId" | "chain_id" => Ok(GeneratedField::ChainId),
                            "maxBlockSize" | "max_block_size" => Ok(GeneratedField::MaxBlockSize),
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "feeContract" | "fee_contract" => Ok(GeneratedField::FeeContract),
                            "feeRecipient" | "fee_recipient" => Ok(GeneratedField::FeeRecipient),
                            "stakeTableContract" | "stake_table_contract" => Ok(GeneratedField::StakeTableContract),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ChainConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.ChainConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ChainConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_id__ = None;
                let mut max_block_size__ = None;
                let mut base_fee__ = None;
                let mut fee_contract__ = None;
                let mut fee_recipient__ = None;
                let mut stake_table_contract__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ChainId => {
                            if chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainId"));
                            }
                            chain_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::MaxBlockSize => {
                            if max_block_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxBlockSize"));
                            }
                            max_block_size__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeContract => {
                            if fee_contract__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeContract"));
                            }
                            fee_contract__ = map_.next_value()?;
                        }
                        GeneratedField::FeeRecipient => {
                            if fee_recipient__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeRecipient"));
                            }
                            fee_recipient__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StakeTableContract => {
                            if stake_table_contract__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stakeTableContract"));
                            }
                            stake_table_contract__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ChainConfig {
                    chain_id: chain_id__.unwrap_or_default(),
                    max_block_size: max_block_size__.unwrap_or_default(),
                    base_fee: base_fee__.unwrap_or_default(),
                    fee_contract: fee_contract__,
                    fee_recipient: fee_recipient__.unwrap_or_default(),
                    stake_table_contract: stake_table_contract__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.ChainConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CirculatingSupplyEthereumResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.amount.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.CirculatingSupplyEthereumResponse", len)?;
        if !self.amount.is_empty() {
            struct_ser.serialize_field("amount", &self.amount)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CirculatingSupplyEthereumResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "amount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Amount,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "amount" => Ok(GeneratedField::Amount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CirculatingSupplyEthereumResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.CirculatingSupplyEthereumResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CirculatingSupplyEthereumResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut amount__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CirculatingSupplyEthereumResponse {
                    amount: amount__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.CirculatingSupplyEthereumResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CirculatingSupplyResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.amount.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.CirculatingSupplyResponse", len)?;
        if !self.amount.is_empty() {
            struct_ser.serialize_field("amount", &self.amount)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CirculatingSupplyResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "amount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Amount,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "amount" => Ok(GeneratedField::Amount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CirculatingSupplyResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.CirculatingSupplyResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CirculatingSupplyResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut amount__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CirculatingSupplyResponse {
                    amount: amount__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.CirculatingSupplyResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EnvResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.variables.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.EnvResponse", len)?;
        if !self.variables.is_empty() {
            struct_ser.serialize_field("variables", &self.variables)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EnvResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "variables",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Variables,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "variables" => Ok(GeneratedField::Variables),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EnvResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.EnvResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<EnvResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut variables__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Variables => {
                            if variables__.is_some() {
                                return Err(serde::de::Error::duplicate_field("variables"));
                            }
                            variables__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(EnvResponse {
                    variables: variables__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.EnvResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EnvVar {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.value.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.EnvVar", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.value.is_empty() {
            struct_ser.serialize_field("value", &self.value)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EnvVar {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "value",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Value,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "value" => Ok(GeneratedField::Value),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EnvVar;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.EnvVar")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<EnvVar, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Value => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(EnvVar {
                    name: name__.unwrap_or_default(),
                    value: value__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.EnvVar", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FeeInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.account.is_empty() {
            len += 1;
        }
        if !self.amount.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.FeeInfo", len)?;
        if !self.account.is_empty() {
            struct_ser.serialize_field("account", &self.account)?;
        }
        if !self.amount.is_empty() {
            struct_ser.serialize_field("amount", &self.amount)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FeeInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "account",
            "amount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Account,
            Amount,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "account" => Ok(GeneratedField::Account),
                            "amount" => Ok(GeneratedField::Amount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FeeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.FeeInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FeeInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut account__ = None;
                let mut amount__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Account => {
                            if account__.is_some() {
                                return Err(serde::de::Error::duplicate_field("account"));
                            }
                            account__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(FeeInfo {
                    account: account__.unwrap_or_default(),
                    amount: amount__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.FeeInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBlockHeightRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetBlockHeightRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBlockHeightRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetBlockHeightRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetBlockHeightRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBlockHeightRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetBlockHeightRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetBlockHeightRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBlockRangeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from.is_some() {
            len += 1;
        }
        if self.until.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetBlockRangeRequest", len)?;
        if let Some(v) = self.from.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.until.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("until", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBlockRangeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "until",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            Until,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "until" => Ok(GeneratedField::Until),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetBlockRangeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetBlockRangeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBlockRangeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut until__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Until => {
                            if until__.is_some() {
                                return Err(serde::de::Error::duplicate_field("until"));
                            }
                            until__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetBlockRangeRequest {
                    from: from__,
                    until: until__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetBlockRangeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBlockRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        if self.hash.is_some() {
            len += 1;
        }
        if self.payload_hash.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetBlockRequest", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.hash.as_ref() {
            struct_ser.serialize_field("hash", v)?;
        }
        if let Some(v) = self.payload_hash.as_ref() {
            struct_ser.serialize_field("payloadHash", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBlockRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "hash",
            "payload_hash",
            "payloadHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Hash,
            PayloadHash,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "hash" => Ok(GeneratedField::Hash),
                            "payloadHash" | "payload_hash" => Ok(GeneratedField::PayloadHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetBlockRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetBlockRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBlockRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut hash__ = None;
                let mut payload_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = map_.next_value()?;
                        }
                        GeneratedField::PayloadHash => {
                            if payload_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadHash"));
                            }
                            payload_hash__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetBlockRequest {
                    height: height__,
                    hash: hash__,
                    payload_hash: payload_hash__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetBlockRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBlockRewardRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.epoch.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetBlockRewardRequest", len)?;
        if let Some(v) = self.epoch.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBlockRewardRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "epoch",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Epoch,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "epoch" => Ok(GeneratedField::Epoch),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetBlockRewardRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetBlockRewardRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBlockRewardRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut epoch__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetBlockRewardRequest {
                    epoch: epoch__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetBlockRewardRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBlockSummaryRangeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from.is_some() {
            len += 1;
        }
        if self.until.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetBlockSummaryRangeRequest", len)?;
        if let Some(v) = self.from.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.until.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("until", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBlockSummaryRangeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "until",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            Until,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "until" => Ok(GeneratedField::Until),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetBlockSummaryRangeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetBlockSummaryRangeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBlockSummaryRangeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut until__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Until => {
                            if until__.is_some() {
                                return Err(serde::de::Error::duplicate_field("until"));
                            }
                            until__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetBlockSummaryRangeRequest {
                    from: from__,
                    until: until__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetBlockSummaryRangeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBlockSummaryRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetBlockSummaryRequest", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBlockSummaryRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetBlockSummaryRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetBlockSummaryRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBlockSummaryRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetBlockSummaryRequest {
                    height: height__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetBlockSummaryRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetCert2Request {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetCert2Request", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetCert2Request {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetCert2Request;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetCert2Request")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetCert2Request, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetCert2Request {
                    height: height__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetCert2Request", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetCirculatingSupplyEthereumRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetCirculatingSupplyEthereumRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetCirculatingSupplyEthereumRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetCirculatingSupplyEthereumRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetCirculatingSupplyEthereumRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetCirculatingSupplyEthereumRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetCirculatingSupplyEthereumRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetCirculatingSupplyEthereumRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetCirculatingSupplyRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetCirculatingSupplyRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetCirculatingSupplyRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetCirculatingSupplyRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetCirculatingSupplyRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetCirculatingSupplyRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetCirculatingSupplyRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetCirculatingSupplyRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetEnvRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetEnvRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetEnvRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetEnvRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetEnvRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetEnvRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetEnvRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetEnvRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetHeaderRangeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from.is_some() {
            len += 1;
        }
        if self.until.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetHeaderRangeRequest", len)?;
        if let Some(v) = self.from.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.until.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("until", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetHeaderRangeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "until",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            Until,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "until" => Ok(GeneratedField::Until),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetHeaderRangeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetHeaderRangeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetHeaderRangeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut until__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Until => {
                            if until__.is_some() {
                                return Err(serde::de::Error::duplicate_field("until"));
                            }
                            until__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetHeaderRangeRequest {
                    from: from__,
                    until: until__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetHeaderRangeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetHeaderRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        if self.hash.is_some() {
            len += 1;
        }
        if self.payload_hash.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetHeaderRequest", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.hash.as_ref() {
            struct_ser.serialize_field("hash", v)?;
        }
        if let Some(v) = self.payload_hash.as_ref() {
            struct_ser.serialize_field("payloadHash", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetHeaderRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "hash",
            "payload_hash",
            "payloadHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Hash,
            PayloadHash,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "hash" => Ok(GeneratedField::Hash),
                            "payloadHash" | "payload_hash" => Ok(GeneratedField::PayloadHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetHeaderRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetHeaderRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetHeaderRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut hash__ = None;
                let mut payload_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = map_.next_value()?;
                        }
                        GeneratedField::PayloadHash => {
                            if payload_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadHash"));
                            }
                            payload_hash__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetHeaderRequest {
                    height: height__,
                    hash: hash__,
                    payload_hash: payload_hash__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetHeaderRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetHotshotConfigRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetHotshotConfigRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetHotshotConfigRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetHotshotConfigRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetHotshotConfigRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetHotshotConfigRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetHotshotConfigRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetHotshotConfigRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetIncorrectEncodingProofRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        if self.namespace.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetIncorrectEncodingProofRequest", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.namespace.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("namespace", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetIncorrectEncodingProofRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "namespace",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Namespace,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "namespace" => Ok(GeneratedField::Namespace),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetIncorrectEncodingProofRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetIncorrectEncodingProofRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetIncorrectEncodingProofRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut namespace__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Namespace => {
                            if namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespace"));
                            }
                            namespace__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetIncorrectEncodingProofRequest {
                    height: height__,
                    namespace: namespace__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetIncorrectEncodingProofRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetLeafRangeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from.is_some() {
            len += 1;
        }
        if self.until.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetLeafRangeRequest", len)?;
        if let Some(v) = self.from.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.until.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("until", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetLeafRangeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "until",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            Until,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "until" => Ok(GeneratedField::Until),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetLeafRangeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetLeafRangeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetLeafRangeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut until__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Until => {
                            if until__.is_some() {
                                return Err(serde::de::Error::duplicate_field("until"));
                            }
                            until__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetLeafRangeRequest {
                    from: from__,
                    until: until__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetLeafRangeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetLeafRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        if self.hash.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetLeafRequest", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.hash.as_ref() {
            struct_ser.serialize_field("hash", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetLeafRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "hash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Hash,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "hash" => Ok(GeneratedField::Hash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetLeafRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetLeafRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetLeafRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetLeafRequest {
                    height: height__,
                    hash: hash__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetLeafRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetLimitsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetLimitsRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetLimitsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetLimitsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetLimitsRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetLimitsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetLimitsRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetLimitsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetMigrationStatusRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetMigrationStatusRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetMigrationStatusRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetMigrationStatusRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetMigrationStatusRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetMigrationStatusRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetMigrationStatusRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetMigrationStatusRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetNamespaceProofRangeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from.is_some() {
            len += 1;
        }
        if self.until.is_some() {
            len += 1;
        }
        if self.namespace.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetNamespaceProofRangeRequest", len)?;
        if let Some(v) = self.from.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.until.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("until", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.namespace.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("namespace", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetNamespaceProofRangeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "until",
            "namespace",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            Until,
            Namespace,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "until" => Ok(GeneratedField::Until),
                            "namespace" => Ok(GeneratedField::Namespace),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetNamespaceProofRangeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetNamespaceProofRangeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetNamespaceProofRangeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut until__ = None;
                let mut namespace__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Until => {
                            if until__.is_some() {
                                return Err(serde::de::Error::duplicate_field("until"));
                            }
                            until__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Namespace => {
                            if namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespace"));
                            }
                            namespace__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetNamespaceProofRangeRequest {
                    from: from__,
                    until: until__,
                    namespace: namespace__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetNamespaceProofRangeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetNamespaceProofRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        if self.hash.is_some() {
            len += 1;
        }
        if self.payload_hash.is_some() {
            len += 1;
        }
        if self.namespace.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetNamespaceProofRequest", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.hash.as_ref() {
            struct_ser.serialize_field("hash", v)?;
        }
        if let Some(v) = self.payload_hash.as_ref() {
            struct_ser.serialize_field("payloadHash", v)?;
        }
        if let Some(v) = self.namespace.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("namespace", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetNamespaceProofRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "hash",
            "payload_hash",
            "payloadHash",
            "namespace",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Hash,
            PayloadHash,
            Namespace,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "hash" => Ok(GeneratedField::Hash),
                            "payloadHash" | "payload_hash" => Ok(GeneratedField::PayloadHash),
                            "namespace" => Ok(GeneratedField::Namespace),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetNamespaceProofRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetNamespaceProofRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetNamespaceProofRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut hash__ = None;
                let mut payload_hash__ = None;
                let mut namespace__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = map_.next_value()?;
                        }
                        GeneratedField::PayloadHash => {
                            if payload_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadHash"));
                            }
                            payload_hash__ = map_.next_value()?;
                        }
                        GeneratedField::Namespace => {
                            if namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespace"));
                            }
                            namespace__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetNamespaceProofRequest {
                    height: height__,
                    hash: hash__,
                    payload_hash: payload_hash__,
                    namespace: namespace__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetNamespaceProofRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetNodeKeysRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetNodeKeysRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetNodeKeysRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetNodeKeysRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetNodeKeysRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetNodeKeysRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetNodeKeysRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetNodeKeysRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetPayloadRangeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from.is_some() {
            len += 1;
        }
        if self.until.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetPayloadRangeRequest", len)?;
        if let Some(v) = self.from.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.until.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("until", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetPayloadRangeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "until",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            Until,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "until" => Ok(GeneratedField::Until),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetPayloadRangeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetPayloadRangeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetPayloadRangeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut until__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Until => {
                            if until__.is_some() {
                                return Err(serde::de::Error::duplicate_field("until"));
                            }
                            until__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetPayloadRangeRequest {
                    from: from__,
                    until: until__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetPayloadRangeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetPayloadRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        if self.hash.is_some() {
            len += 1;
        }
        if self.block_hash.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetPayloadRequest", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.hash.as_ref() {
            struct_ser.serialize_field("hash", v)?;
        }
        if let Some(v) = self.block_hash.as_ref() {
            struct_ser.serialize_field("blockHash", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetPayloadRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "hash",
            "block_hash",
            "blockHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Hash,
            BlockHash,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "hash" => Ok(GeneratedField::Hash),
                            "blockHash" | "block_hash" => Ok(GeneratedField::BlockHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetPayloadRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetPayloadRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetPayloadRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut hash__ = None;
                let mut block_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = map_.next_value()?;
                        }
                        GeneratedField::BlockHash => {
                            if block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHash"));
                            }
                            block_hash__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetPayloadRequest {
                    height: height__,
                    hash: hash__,
                    block_hash: block_hash__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetPayloadRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetPayloadSizeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from.is_some() {
            len += 1;
        }
        if self.to.is_some() {
            len += 1;
        }
        if self.namespace.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetPayloadSizeRequest", len)?;
        if let Some(v) = self.from.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.to.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("to", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.namespace.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("namespace", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetPayloadSizeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "to",
            "namespace",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            To,
            Namespace,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "to" => Ok(GeneratedField::To),
                            "namespace" => Ok(GeneratedField::Namespace),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetPayloadSizeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetPayloadSizeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetPayloadSizeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut to__ = None;
                let mut namespace__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::To => {
                            if to__.is_some() {
                                return Err(serde::de::Error::duplicate_field("to"));
                            }
                            to__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Namespace => {
                            if namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespace"));
                            }
                            namespace__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetPayloadSizeRequest {
                    from: from__,
                    to: to__,
                    namespace: namespace__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetPayloadSizeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetRuntimeConfigRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetRuntimeConfigRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetRuntimeConfigRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetRuntimeConfigRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetRuntimeConfigRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetRuntimeConfigRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetRuntimeConfigRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetRuntimeConfigRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetStateCertRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.epoch.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetStateCertRequest", len)?;
        if let Some(v) = self.epoch.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetStateCertRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "epoch",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Epoch,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "epoch" => Ok(GeneratedField::Epoch),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetStateCertRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetStateCertRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetStateCertRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut epoch__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetStateCertRequest {
                    epoch: epoch__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetStateCertRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetSuccessRateRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetSuccessRateRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetSuccessRateRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetSuccessRateRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetSuccessRateRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetSuccessRateRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetSuccessRateRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetSuccessRateRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetSyncStatusRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetSyncStatusRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetSyncStatusRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetSyncStatusRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetSyncStatusRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetSyncStatusRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetSyncStatusRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetSyncStatusRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTableSizesRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetTableSizesRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTableSizesRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTableSizesRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetTableSizesRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTableSizesRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetTableSizesRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetTableSizesRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTimeSinceLastDecideRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetTimeSinceLastDecideRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTimeSinceLastDecideRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTimeSinceLastDecideRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetTimeSinceLastDecideRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTimeSinceLastDecideRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetTimeSinceLastDecideRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetTimeSinceLastDecideRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTotalIssuedSupplyRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetTotalIssuedSupplyRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTotalIssuedSupplyRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTotalIssuedSupplyRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetTotalIssuedSupplyRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTotalIssuedSupplyRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetTotalIssuedSupplyRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetTotalIssuedSupplyRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTotalMintedSupplyRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetTotalMintedSupplyRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTotalMintedSupplyRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTotalMintedSupplyRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetTotalMintedSupplyRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTotalMintedSupplyRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetTotalMintedSupplyRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetTotalMintedSupplyRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTotalRewardDistributedRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetTotalRewardDistributedRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTotalRewardDistributedRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTotalRewardDistributedRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetTotalRewardDistributedRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTotalRewardDistributedRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetTotalRewardDistributedRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetTotalRewardDistributedRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTransactionCountRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from.is_some() {
            len += 1;
        }
        if self.to.is_some() {
            len += 1;
        }
        if self.namespace.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetTransactionCountRequest", len)?;
        if let Some(v) = self.from.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.to.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("to", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.namespace.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("namespace", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTransactionCountRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "to",
            "namespace",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            To,
            Namespace,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "to" => Ok(GeneratedField::To),
                            "namespace" => Ok(GeneratedField::Namespace),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTransactionCountRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetTransactionCountRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTransactionCountRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut to__ = None;
                let mut namespace__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::To => {
                            if to__.is_some() {
                                return Err(serde::de::Error::duplicate_field("to"));
                            }
                            to__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Namespace => {
                            if namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespace"));
                            }
                            namespace__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetTransactionCountRequest {
                    from: from__,
                    to: to__,
                    namespace: namespace__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetTransactionCountRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTransactionProofRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        if self.index.is_some() {
            len += 1;
        }
        if self.hash.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetTransactionProofRequest", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.index.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("index", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.hash.as_ref() {
            struct_ser.serialize_field("hash", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTransactionProofRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "index",
            "hash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Index,
            Hash,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "index" => Ok(GeneratedField::Index),
                            "hash" => Ok(GeneratedField::Hash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTransactionProofRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetTransactionProofRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTransactionProofRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut index__ = None;
                let mut hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetTransactionProofRequest {
                    height: height__,
                    index: index__,
                    hash: hash__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetTransactionProofRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTransactionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        if self.index.is_some() {
            len += 1;
        }
        if self.hash.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetTransactionRequest", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.index.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("index", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.hash.as_ref() {
            struct_ser.serialize_field("hash", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTransactionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "index",
            "hash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Index,
            Hash,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "index" => Ok(GeneratedField::Index),
                            "hash" => Ok(GeneratedField::Hash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTransactionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetTransactionRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTransactionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut index__ = None;
                let mut hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetTransactionRequest {
                    height: height__,
                    index: index__,
                    hash: hash__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetTransactionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetVidCommonRangeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from.is_some() {
            len += 1;
        }
        if self.until.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetVidCommonRangeRequest", len)?;
        if let Some(v) = self.from.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.until.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("until", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetVidCommonRangeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "until",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            Until,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "until" => Ok(GeneratedField::Until),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetVidCommonRangeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetVidCommonRangeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetVidCommonRangeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut until__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Until => {
                            if until__.is_some() {
                                return Err(serde::de::Error::duplicate_field("until"));
                            }
                            until__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetVidCommonRangeRequest {
                    from: from__,
                    until: until__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetVidCommonRangeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetVidCommonRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height.is_some() {
            len += 1;
        }
        if self.hash.is_some() {
            len += 1;
        }
        if self.payload_hash.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetVidCommonRequest", len)?;
        if let Some(v) = self.height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.hash.as_ref() {
            struct_ser.serialize_field("hash", v)?;
        }
        if let Some(v) = self.payload_hash.as_ref() {
            struct_ser.serialize_field("payloadHash", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetVidCommonRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "hash",
            "payload_hash",
            "payloadHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Hash,
            PayloadHash,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "hash" => Ok(GeneratedField::Hash),
                            "payloadHash" | "payload_hash" => Ok(GeneratedField::PayloadHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetVidCommonRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetVidCommonRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetVidCommonRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut hash__ = None;
                let mut payload_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = map_.next_value()?;
                        }
                        GeneratedField::PayloadHash => {
                            if payload_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadHash"));
                            }
                            payload_hash__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetVidCommonRequest {
                    height: height__,
                    hash: hash__,
                    payload_hash: payload_hash__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetVidCommonRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HeaderRangeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.headers.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.HeaderRangeResponse", len)?;
        if !self.headers.is_empty() {
            struct_ser.serialize_field("headers", &self.headers)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HeaderRangeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "headers",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Headers,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "headers" => Ok(GeneratedField::Headers),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HeaderRangeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.HeaderRangeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<HeaderRangeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut headers__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Headers => {
                            if headers__.is_some() {
                                return Err(serde::de::Error::duplicate_field("headers"));
                            }
                            headers__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(HeaderRangeResponse {
                    headers: headers__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.HeaderRangeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HeaderResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.header.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.HeaderResponse", len)?;
        if let Some(v) = self.header.as_ref() {
            match v {
                header_response::Header::V1(v) => {
                    struct_ser.serialize_field("v1", v)?;
                }
                header_response::Header::V2(v) => {
                    struct_ser.serialize_field("v2", v)?;
                }
                header_response::Header::V3(v) => {
                    struct_ser.serialize_field("v3", v)?;
                }
                header_response::Header::V4(v) => {
                    struct_ser.serialize_field("v4", v)?;
                }
                header_response::Header::V5(v) => {
                    struct_ser.serialize_field("v5", v)?;
                }
                header_response::Header::V6(v) => {
                    struct_ser.serialize_field("v6", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HeaderResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "v1",
            "v2",
            "v3",
            "v4",
            "v5",
            "v6",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            V1,
            V2,
            V3,
            V4,
            V5,
            V6,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "v1" => Ok(GeneratedField::V1),
                            "v2" => Ok(GeneratedField::V2),
                            "v3" => Ok(GeneratedField::V3),
                            "v4" => Ok(GeneratedField::V4),
                            "v5" => Ok(GeneratedField::V5),
                            "v6" => Ok(GeneratedField::V6),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HeaderResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.HeaderResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<HeaderResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::V1 => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v1"));
                            }
                            header__ = map_.next_value::<::std::option::Option<_>>()?.map(header_response::Header::V1)
;
                        }
                        GeneratedField::V2 => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v2"));
                            }
                            header__ = map_.next_value::<::std::option::Option<_>>()?.map(header_response::Header::V2)
;
                        }
                        GeneratedField::V3 => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v3"));
                            }
                            header__ = map_.next_value::<::std::option::Option<_>>()?.map(header_response::Header::V3)
;
                        }
                        GeneratedField::V4 => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v4"));
                            }
                            header__ = map_.next_value::<::std::option::Option<_>>()?.map(header_response::Header::V4)
;
                        }
                        GeneratedField::V5 => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v5"));
                            }
                            header__ = map_.next_value::<::std::option::Option<_>>()?.map(header_response::Header::V5)
;
                        }
                        GeneratedField::V6 => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v6"));
                            }
                            header__ = map_.next_value::<::std::option::Option<_>>()?.map(header_response::Header::V6)
;
                        }
                    }
                }
                Ok(HeaderResponse {
                    header: header__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.HeaderResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HeaderV1 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.chain_config.is_some() {
            len += 1;
        }
        if self.height != 0 {
            len += 1;
        }
        if self.timestamp != 0 {
            len += 1;
        }
        if self.l1_head != 0 {
            len += 1;
        }
        if self.l1_finalized.is_some() {
            len += 1;
        }
        if !self.payload_commitment.is_empty() {
            len += 1;
        }
        if !self.builder_commitment.is_empty() {
            len += 1;
        }
        if self.ns_table.is_some() {
            len += 1;
        }
        if !self.block_merkle_tree_root.is_empty() {
            len += 1;
        }
        if !self.fee_merkle_tree_root.is_empty() {
            len += 1;
        }
        if self.fee_info.is_some() {
            len += 1;
        }
        if self.builder_signature.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.HeaderV1", len)?;
        if let Some(v) = self.chain_config.as_ref() {
            struct_ser.serialize_field("chainConfig", v)?;
        }
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if self.timestamp != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("timestamp", ToString::to_string(&self.timestamp).as_str())?;
        }
        if self.l1_head != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("l1Head", ToString::to_string(&self.l1_head).as_str())?;
        }
        if let Some(v) = self.l1_finalized.as_ref() {
            struct_ser.serialize_field("l1Finalized", v)?;
        }
        if !self.payload_commitment.is_empty() {
            struct_ser.serialize_field("payloadCommitment", &self.payload_commitment)?;
        }
        if !self.builder_commitment.is_empty() {
            struct_ser.serialize_field("builderCommitment", &self.builder_commitment)?;
        }
        if let Some(v) = self.ns_table.as_ref() {
            struct_ser.serialize_field("nsTable", v)?;
        }
        if !self.block_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("blockMerkleTreeRoot", &self.block_merkle_tree_root)?;
        }
        if !self.fee_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("feeMerkleTreeRoot", &self.fee_merkle_tree_root)?;
        }
        if let Some(v) = self.fee_info.as_ref() {
            struct_ser.serialize_field("feeInfo", v)?;
        }
        if let Some(v) = self.builder_signature.as_ref() {
            struct_ser.serialize_field("builderSignature", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HeaderV1 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_config",
            "chainConfig",
            "height",
            "timestamp",
            "l1_head",
            "l1Head",
            "l1_finalized",
            "l1Finalized",
            "payload_commitment",
            "payloadCommitment",
            "builder_commitment",
            "builderCommitment",
            "ns_table",
            "nsTable",
            "block_merkle_tree_root",
            "blockMerkleTreeRoot",
            "fee_merkle_tree_root",
            "feeMerkleTreeRoot",
            "fee_info",
            "feeInfo",
            "builder_signature",
            "builderSignature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainConfig,
            Height,
            Timestamp,
            L1Head,
            L1Finalized,
            PayloadCommitment,
            BuilderCommitment,
            NsTable,
            BlockMerkleTreeRoot,
            FeeMerkleTreeRoot,
            FeeInfo,
            BuilderSignature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "chainConfig" | "chain_config" => Ok(GeneratedField::ChainConfig),
                            "height" => Ok(GeneratedField::Height),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "l1Head" | "l1_head" => Ok(GeneratedField::L1Head),
                            "l1Finalized" | "l1_finalized" => Ok(GeneratedField::L1Finalized),
                            "payloadCommitment" | "payload_commitment" => Ok(GeneratedField::PayloadCommitment),
                            "builderCommitment" | "builder_commitment" => Ok(GeneratedField::BuilderCommitment),
                            "nsTable" | "ns_table" => Ok(GeneratedField::NsTable),
                            "blockMerkleTreeRoot" | "block_merkle_tree_root" => Ok(GeneratedField::BlockMerkleTreeRoot),
                            "feeMerkleTreeRoot" | "fee_merkle_tree_root" => Ok(GeneratedField::FeeMerkleTreeRoot),
                            "feeInfo" | "fee_info" => Ok(GeneratedField::FeeInfo),
                            "builderSignature" | "builder_signature" => Ok(GeneratedField::BuilderSignature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HeaderV1;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.HeaderV1")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<HeaderV1, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_config__ = None;
                let mut height__ = None;
                let mut timestamp__ = None;
                let mut l1_head__ = None;
                let mut l1_finalized__ = None;
                let mut payload_commitment__ = None;
                let mut builder_commitment__ = None;
                let mut ns_table__ = None;
                let mut block_merkle_tree_root__ = None;
                let mut fee_merkle_tree_root__ = None;
                let mut fee_info__ = None;
                let mut builder_signature__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ChainConfig => {
                            if chain_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainConfig"));
                            }
                            chain_config__ = map_.next_value()?;
                        }
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::L1Head => {
                            if l1_head__.is_some() {
                                return Err(serde::de::Error::duplicate_field("l1Head"));
                            }
                            l1_head__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::L1Finalized => {
                            if l1_finalized__.is_some() {
                                return Err(serde::de::Error::duplicate_field("l1Finalized"));
                            }
                            l1_finalized__ = map_.next_value()?;
                        }
                        GeneratedField::PayloadCommitment => {
                            if payload_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadCommitment"));
                            }
                            payload_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BuilderCommitment => {
                            if builder_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderCommitment"));
                            }
                            builder_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NsTable => {
                            if ns_table__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsTable"));
                            }
                            ns_table__ = map_.next_value()?;
                        }
                        GeneratedField::BlockMerkleTreeRoot => {
                            if block_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockMerkleTreeRoot"));
                            }
                            block_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeMerkleTreeRoot => {
                            if fee_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeMerkleTreeRoot"));
                            }
                            fee_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeInfo => {
                            if fee_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeInfo"));
                            }
                            fee_info__ = map_.next_value()?;
                        }
                        GeneratedField::BuilderSignature => {
                            if builder_signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderSignature"));
                            }
                            builder_signature__ = map_.next_value()?;
                        }
                    }
                }
                Ok(HeaderV1 {
                    chain_config: chain_config__,
                    height: height__.unwrap_or_default(),
                    timestamp: timestamp__.unwrap_or_default(),
                    l1_head: l1_head__.unwrap_or_default(),
                    l1_finalized: l1_finalized__,
                    payload_commitment: payload_commitment__.unwrap_or_default(),
                    builder_commitment: builder_commitment__.unwrap_or_default(),
                    ns_table: ns_table__,
                    block_merkle_tree_root: block_merkle_tree_root__.unwrap_or_default(),
                    fee_merkle_tree_root: fee_merkle_tree_root__.unwrap_or_default(),
                    fee_info: fee_info__,
                    builder_signature: builder_signature__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.HeaderV1", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HeaderV3 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.chain_config.is_some() {
            len += 1;
        }
        if self.height != 0 {
            len += 1;
        }
        if self.timestamp != 0 {
            len += 1;
        }
        if self.l1_head != 0 {
            len += 1;
        }
        if self.l1_finalized.is_some() {
            len += 1;
        }
        if !self.payload_commitment.is_empty() {
            len += 1;
        }
        if !self.builder_commitment.is_empty() {
            len += 1;
        }
        if self.ns_table.is_some() {
            len += 1;
        }
        if !self.block_merkle_tree_root.is_empty() {
            len += 1;
        }
        if !self.fee_merkle_tree_root.is_empty() {
            len += 1;
        }
        if self.fee_info.is_some() {
            len += 1;
        }
        if self.builder_signature.is_some() {
            len += 1;
        }
        if !self.reward_merkle_tree_root.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.HeaderV3", len)?;
        if let Some(v) = self.chain_config.as_ref() {
            struct_ser.serialize_field("chainConfig", v)?;
        }
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if self.timestamp != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("timestamp", ToString::to_string(&self.timestamp).as_str())?;
        }
        if self.l1_head != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("l1Head", ToString::to_string(&self.l1_head).as_str())?;
        }
        if let Some(v) = self.l1_finalized.as_ref() {
            struct_ser.serialize_field("l1Finalized", v)?;
        }
        if !self.payload_commitment.is_empty() {
            struct_ser.serialize_field("payloadCommitment", &self.payload_commitment)?;
        }
        if !self.builder_commitment.is_empty() {
            struct_ser.serialize_field("builderCommitment", &self.builder_commitment)?;
        }
        if let Some(v) = self.ns_table.as_ref() {
            struct_ser.serialize_field("nsTable", v)?;
        }
        if !self.block_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("blockMerkleTreeRoot", &self.block_merkle_tree_root)?;
        }
        if !self.fee_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("feeMerkleTreeRoot", &self.fee_merkle_tree_root)?;
        }
        if let Some(v) = self.fee_info.as_ref() {
            struct_ser.serialize_field("feeInfo", v)?;
        }
        if let Some(v) = self.builder_signature.as_ref() {
            struct_ser.serialize_field("builderSignature", v)?;
        }
        if !self.reward_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("rewardMerkleTreeRoot", &self.reward_merkle_tree_root)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HeaderV3 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_config",
            "chainConfig",
            "height",
            "timestamp",
            "l1_head",
            "l1Head",
            "l1_finalized",
            "l1Finalized",
            "payload_commitment",
            "payloadCommitment",
            "builder_commitment",
            "builderCommitment",
            "ns_table",
            "nsTable",
            "block_merkle_tree_root",
            "blockMerkleTreeRoot",
            "fee_merkle_tree_root",
            "feeMerkleTreeRoot",
            "fee_info",
            "feeInfo",
            "builder_signature",
            "builderSignature",
            "reward_merkle_tree_root",
            "rewardMerkleTreeRoot",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainConfig,
            Height,
            Timestamp,
            L1Head,
            L1Finalized,
            PayloadCommitment,
            BuilderCommitment,
            NsTable,
            BlockMerkleTreeRoot,
            FeeMerkleTreeRoot,
            FeeInfo,
            BuilderSignature,
            RewardMerkleTreeRoot,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "chainConfig" | "chain_config" => Ok(GeneratedField::ChainConfig),
                            "height" => Ok(GeneratedField::Height),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "l1Head" | "l1_head" => Ok(GeneratedField::L1Head),
                            "l1Finalized" | "l1_finalized" => Ok(GeneratedField::L1Finalized),
                            "payloadCommitment" | "payload_commitment" => Ok(GeneratedField::PayloadCommitment),
                            "builderCommitment" | "builder_commitment" => Ok(GeneratedField::BuilderCommitment),
                            "nsTable" | "ns_table" => Ok(GeneratedField::NsTable),
                            "blockMerkleTreeRoot" | "block_merkle_tree_root" => Ok(GeneratedField::BlockMerkleTreeRoot),
                            "feeMerkleTreeRoot" | "fee_merkle_tree_root" => Ok(GeneratedField::FeeMerkleTreeRoot),
                            "feeInfo" | "fee_info" => Ok(GeneratedField::FeeInfo),
                            "builderSignature" | "builder_signature" => Ok(GeneratedField::BuilderSignature),
                            "rewardMerkleTreeRoot" | "reward_merkle_tree_root" => Ok(GeneratedField::RewardMerkleTreeRoot),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HeaderV3;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.HeaderV3")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<HeaderV3, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_config__ = None;
                let mut height__ = None;
                let mut timestamp__ = None;
                let mut l1_head__ = None;
                let mut l1_finalized__ = None;
                let mut payload_commitment__ = None;
                let mut builder_commitment__ = None;
                let mut ns_table__ = None;
                let mut block_merkle_tree_root__ = None;
                let mut fee_merkle_tree_root__ = None;
                let mut fee_info__ = None;
                let mut builder_signature__ = None;
                let mut reward_merkle_tree_root__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ChainConfig => {
                            if chain_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainConfig"));
                            }
                            chain_config__ = map_.next_value()?;
                        }
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::L1Head => {
                            if l1_head__.is_some() {
                                return Err(serde::de::Error::duplicate_field("l1Head"));
                            }
                            l1_head__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::L1Finalized => {
                            if l1_finalized__.is_some() {
                                return Err(serde::de::Error::duplicate_field("l1Finalized"));
                            }
                            l1_finalized__ = map_.next_value()?;
                        }
                        GeneratedField::PayloadCommitment => {
                            if payload_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadCommitment"));
                            }
                            payload_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BuilderCommitment => {
                            if builder_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderCommitment"));
                            }
                            builder_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NsTable => {
                            if ns_table__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsTable"));
                            }
                            ns_table__ = map_.next_value()?;
                        }
                        GeneratedField::BlockMerkleTreeRoot => {
                            if block_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockMerkleTreeRoot"));
                            }
                            block_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeMerkleTreeRoot => {
                            if fee_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeMerkleTreeRoot"));
                            }
                            fee_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeInfo => {
                            if fee_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeInfo"));
                            }
                            fee_info__ = map_.next_value()?;
                        }
                        GeneratedField::BuilderSignature => {
                            if builder_signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderSignature"));
                            }
                            builder_signature__ = map_.next_value()?;
                        }
                        GeneratedField::RewardMerkleTreeRoot => {
                            if reward_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rewardMerkleTreeRoot"));
                            }
                            reward_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(HeaderV3 {
                    chain_config: chain_config__,
                    height: height__.unwrap_or_default(),
                    timestamp: timestamp__.unwrap_or_default(),
                    l1_head: l1_head__.unwrap_or_default(),
                    l1_finalized: l1_finalized__,
                    payload_commitment: payload_commitment__.unwrap_or_default(),
                    builder_commitment: builder_commitment__.unwrap_or_default(),
                    ns_table: ns_table__,
                    block_merkle_tree_root: block_merkle_tree_root__.unwrap_or_default(),
                    fee_merkle_tree_root: fee_merkle_tree_root__.unwrap_or_default(),
                    fee_info: fee_info__,
                    builder_signature: builder_signature__,
                    reward_merkle_tree_root: reward_merkle_tree_root__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.HeaderV3", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HeaderV4 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.chain_config.is_some() {
            len += 1;
        }
        if self.height != 0 {
            len += 1;
        }
        if self.timestamp != 0 {
            len += 1;
        }
        if self.timestamp_millis != 0 {
            len += 1;
        }
        if self.l1_head != 0 {
            len += 1;
        }
        if self.l1_finalized.is_some() {
            len += 1;
        }
        if !self.payload_commitment.is_empty() {
            len += 1;
        }
        if !self.builder_commitment.is_empty() {
            len += 1;
        }
        if self.ns_table.is_some() {
            len += 1;
        }
        if !self.block_merkle_tree_root.is_empty() {
            len += 1;
        }
        if !self.fee_merkle_tree_root.is_empty() {
            len += 1;
        }
        if self.fee_info.is_some() {
            len += 1;
        }
        if self.builder_signature.is_some() {
            len += 1;
        }
        if !self.reward_merkle_tree_root.is_empty() {
            len += 1;
        }
        if !self.total_reward_distributed.is_empty() {
            len += 1;
        }
        if self.next_stake_table_hash.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.HeaderV4", len)?;
        if let Some(v) = self.chain_config.as_ref() {
            struct_ser.serialize_field("chainConfig", v)?;
        }
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if self.timestamp != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("timestamp", ToString::to_string(&self.timestamp).as_str())?;
        }
        if self.timestamp_millis != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("timestampMillis", ToString::to_string(&self.timestamp_millis).as_str())?;
        }
        if self.l1_head != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("l1Head", ToString::to_string(&self.l1_head).as_str())?;
        }
        if let Some(v) = self.l1_finalized.as_ref() {
            struct_ser.serialize_field("l1Finalized", v)?;
        }
        if !self.payload_commitment.is_empty() {
            struct_ser.serialize_field("payloadCommitment", &self.payload_commitment)?;
        }
        if !self.builder_commitment.is_empty() {
            struct_ser.serialize_field("builderCommitment", &self.builder_commitment)?;
        }
        if let Some(v) = self.ns_table.as_ref() {
            struct_ser.serialize_field("nsTable", v)?;
        }
        if !self.block_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("blockMerkleTreeRoot", &self.block_merkle_tree_root)?;
        }
        if !self.fee_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("feeMerkleTreeRoot", &self.fee_merkle_tree_root)?;
        }
        if let Some(v) = self.fee_info.as_ref() {
            struct_ser.serialize_field("feeInfo", v)?;
        }
        if let Some(v) = self.builder_signature.as_ref() {
            struct_ser.serialize_field("builderSignature", v)?;
        }
        if !self.reward_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("rewardMerkleTreeRoot", &self.reward_merkle_tree_root)?;
        }
        if !self.total_reward_distributed.is_empty() {
            struct_ser.serialize_field("totalRewardDistributed", &self.total_reward_distributed)?;
        }
        if let Some(v) = self.next_stake_table_hash.as_ref() {
            struct_ser.serialize_field("nextStakeTableHash", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HeaderV4 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_config",
            "chainConfig",
            "height",
            "timestamp",
            "timestamp_millis",
            "timestampMillis",
            "l1_head",
            "l1Head",
            "l1_finalized",
            "l1Finalized",
            "payload_commitment",
            "payloadCommitment",
            "builder_commitment",
            "builderCommitment",
            "ns_table",
            "nsTable",
            "block_merkle_tree_root",
            "blockMerkleTreeRoot",
            "fee_merkle_tree_root",
            "feeMerkleTreeRoot",
            "fee_info",
            "feeInfo",
            "builder_signature",
            "builderSignature",
            "reward_merkle_tree_root",
            "rewardMerkleTreeRoot",
            "total_reward_distributed",
            "totalRewardDistributed",
            "next_stake_table_hash",
            "nextStakeTableHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainConfig,
            Height,
            Timestamp,
            TimestampMillis,
            L1Head,
            L1Finalized,
            PayloadCommitment,
            BuilderCommitment,
            NsTable,
            BlockMerkleTreeRoot,
            FeeMerkleTreeRoot,
            FeeInfo,
            BuilderSignature,
            RewardMerkleTreeRoot,
            TotalRewardDistributed,
            NextStakeTableHash,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "chainConfig" | "chain_config" => Ok(GeneratedField::ChainConfig),
                            "height" => Ok(GeneratedField::Height),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "timestampMillis" | "timestamp_millis" => Ok(GeneratedField::TimestampMillis),
                            "l1Head" | "l1_head" => Ok(GeneratedField::L1Head),
                            "l1Finalized" | "l1_finalized" => Ok(GeneratedField::L1Finalized),
                            "payloadCommitment" | "payload_commitment" => Ok(GeneratedField::PayloadCommitment),
                            "builderCommitment" | "builder_commitment" => Ok(GeneratedField::BuilderCommitment),
                            "nsTable" | "ns_table" => Ok(GeneratedField::NsTable),
                            "blockMerkleTreeRoot" | "block_merkle_tree_root" => Ok(GeneratedField::BlockMerkleTreeRoot),
                            "feeMerkleTreeRoot" | "fee_merkle_tree_root" => Ok(GeneratedField::FeeMerkleTreeRoot),
                            "feeInfo" | "fee_info" => Ok(GeneratedField::FeeInfo),
                            "builderSignature" | "builder_signature" => Ok(GeneratedField::BuilderSignature),
                            "rewardMerkleTreeRoot" | "reward_merkle_tree_root" => Ok(GeneratedField::RewardMerkleTreeRoot),
                            "totalRewardDistributed" | "total_reward_distributed" => Ok(GeneratedField::TotalRewardDistributed),
                            "nextStakeTableHash" | "next_stake_table_hash" => Ok(GeneratedField::NextStakeTableHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HeaderV4;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.HeaderV4")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<HeaderV4, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_config__ = None;
                let mut height__ = None;
                let mut timestamp__ = None;
                let mut timestamp_millis__ = None;
                let mut l1_head__ = None;
                let mut l1_finalized__ = None;
                let mut payload_commitment__ = None;
                let mut builder_commitment__ = None;
                let mut ns_table__ = None;
                let mut block_merkle_tree_root__ = None;
                let mut fee_merkle_tree_root__ = None;
                let mut fee_info__ = None;
                let mut builder_signature__ = None;
                let mut reward_merkle_tree_root__ = None;
                let mut total_reward_distributed__ = None;
                let mut next_stake_table_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ChainConfig => {
                            if chain_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainConfig"));
                            }
                            chain_config__ = map_.next_value()?;
                        }
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::TimestampMillis => {
                            if timestamp_millis__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestampMillis"));
                            }
                            timestamp_millis__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::L1Head => {
                            if l1_head__.is_some() {
                                return Err(serde::de::Error::duplicate_field("l1Head"));
                            }
                            l1_head__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::L1Finalized => {
                            if l1_finalized__.is_some() {
                                return Err(serde::de::Error::duplicate_field("l1Finalized"));
                            }
                            l1_finalized__ = map_.next_value()?;
                        }
                        GeneratedField::PayloadCommitment => {
                            if payload_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadCommitment"));
                            }
                            payload_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BuilderCommitment => {
                            if builder_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderCommitment"));
                            }
                            builder_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NsTable => {
                            if ns_table__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsTable"));
                            }
                            ns_table__ = map_.next_value()?;
                        }
                        GeneratedField::BlockMerkleTreeRoot => {
                            if block_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockMerkleTreeRoot"));
                            }
                            block_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeMerkleTreeRoot => {
                            if fee_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeMerkleTreeRoot"));
                            }
                            fee_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeInfo => {
                            if fee_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeInfo"));
                            }
                            fee_info__ = map_.next_value()?;
                        }
                        GeneratedField::BuilderSignature => {
                            if builder_signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderSignature"));
                            }
                            builder_signature__ = map_.next_value()?;
                        }
                        GeneratedField::RewardMerkleTreeRoot => {
                            if reward_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rewardMerkleTreeRoot"));
                            }
                            reward_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TotalRewardDistributed => {
                            if total_reward_distributed__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalRewardDistributed"));
                            }
                            total_reward_distributed__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NextStakeTableHash => {
                            if next_stake_table_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextStakeTableHash"));
                            }
                            next_stake_table_hash__ = map_.next_value()?;
                        }
                    }
                }
                Ok(HeaderV4 {
                    chain_config: chain_config__,
                    height: height__.unwrap_or_default(),
                    timestamp: timestamp__.unwrap_or_default(),
                    timestamp_millis: timestamp_millis__.unwrap_or_default(),
                    l1_head: l1_head__.unwrap_or_default(),
                    l1_finalized: l1_finalized__,
                    payload_commitment: payload_commitment__.unwrap_or_default(),
                    builder_commitment: builder_commitment__.unwrap_or_default(),
                    ns_table: ns_table__,
                    block_merkle_tree_root: block_merkle_tree_root__.unwrap_or_default(),
                    fee_merkle_tree_root: fee_merkle_tree_root__.unwrap_or_default(),
                    fee_info: fee_info__,
                    builder_signature: builder_signature__,
                    reward_merkle_tree_root: reward_merkle_tree_root__.unwrap_or_default(),
                    total_reward_distributed: total_reward_distributed__.unwrap_or_default(),
                    next_stake_table_hash: next_stake_table_hash__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.HeaderV4", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HeaderV5 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.chain_config.is_some() {
            len += 1;
        }
        if self.height != 0 {
            len += 1;
        }
        if self.timestamp != 0 {
            len += 1;
        }
        if self.timestamp_millis != 0 {
            len += 1;
        }
        if self.l1_head != 0 {
            len += 1;
        }
        if self.l1_finalized.is_some() {
            len += 1;
        }
        if !self.payload_commitment.is_empty() {
            len += 1;
        }
        if !self.builder_commitment.is_empty() {
            len += 1;
        }
        if self.ns_table.is_some() {
            len += 1;
        }
        if !self.block_merkle_tree_root.is_empty() {
            len += 1;
        }
        if !self.fee_merkle_tree_root.is_empty() {
            len += 1;
        }
        if self.fee_info.is_some() {
            len += 1;
        }
        if self.builder_signature.is_some() {
            len += 1;
        }
        if !self.reward_merkle_tree_root.is_empty() {
            len += 1;
        }
        if !self.total_reward_distributed.is_empty() {
            len += 1;
        }
        if self.next_stake_table_hash.is_some() {
            len += 1;
        }
        if !self.leader_counts.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.HeaderV5", len)?;
        if let Some(v) = self.chain_config.as_ref() {
            struct_ser.serialize_field("chainConfig", v)?;
        }
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if self.timestamp != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("timestamp", ToString::to_string(&self.timestamp).as_str())?;
        }
        if self.timestamp_millis != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("timestampMillis", ToString::to_string(&self.timestamp_millis).as_str())?;
        }
        if self.l1_head != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("l1Head", ToString::to_string(&self.l1_head).as_str())?;
        }
        if let Some(v) = self.l1_finalized.as_ref() {
            struct_ser.serialize_field("l1Finalized", v)?;
        }
        if !self.payload_commitment.is_empty() {
            struct_ser.serialize_field("payloadCommitment", &self.payload_commitment)?;
        }
        if !self.builder_commitment.is_empty() {
            struct_ser.serialize_field("builderCommitment", &self.builder_commitment)?;
        }
        if let Some(v) = self.ns_table.as_ref() {
            struct_ser.serialize_field("nsTable", v)?;
        }
        if !self.block_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("blockMerkleTreeRoot", &self.block_merkle_tree_root)?;
        }
        if !self.fee_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("feeMerkleTreeRoot", &self.fee_merkle_tree_root)?;
        }
        if let Some(v) = self.fee_info.as_ref() {
            struct_ser.serialize_field("feeInfo", v)?;
        }
        if let Some(v) = self.builder_signature.as_ref() {
            struct_ser.serialize_field("builderSignature", v)?;
        }
        if !self.reward_merkle_tree_root.is_empty() {
            struct_ser.serialize_field("rewardMerkleTreeRoot", &self.reward_merkle_tree_root)?;
        }
        if !self.total_reward_distributed.is_empty() {
            struct_ser.serialize_field("totalRewardDistributed", &self.total_reward_distributed)?;
        }
        if let Some(v) = self.next_stake_table_hash.as_ref() {
            struct_ser.serialize_field("nextStakeTableHash", v)?;
        }
        if !self.leader_counts.is_empty() {
            struct_ser.serialize_field("leaderCounts", &self.leader_counts)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HeaderV5 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_config",
            "chainConfig",
            "height",
            "timestamp",
            "timestamp_millis",
            "timestampMillis",
            "l1_head",
            "l1Head",
            "l1_finalized",
            "l1Finalized",
            "payload_commitment",
            "payloadCommitment",
            "builder_commitment",
            "builderCommitment",
            "ns_table",
            "nsTable",
            "block_merkle_tree_root",
            "blockMerkleTreeRoot",
            "fee_merkle_tree_root",
            "feeMerkleTreeRoot",
            "fee_info",
            "feeInfo",
            "builder_signature",
            "builderSignature",
            "reward_merkle_tree_root",
            "rewardMerkleTreeRoot",
            "total_reward_distributed",
            "totalRewardDistributed",
            "next_stake_table_hash",
            "nextStakeTableHash",
            "leader_counts",
            "leaderCounts",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainConfig,
            Height,
            Timestamp,
            TimestampMillis,
            L1Head,
            L1Finalized,
            PayloadCommitment,
            BuilderCommitment,
            NsTable,
            BlockMerkleTreeRoot,
            FeeMerkleTreeRoot,
            FeeInfo,
            BuilderSignature,
            RewardMerkleTreeRoot,
            TotalRewardDistributed,
            NextStakeTableHash,
            LeaderCounts,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "chainConfig" | "chain_config" => Ok(GeneratedField::ChainConfig),
                            "height" => Ok(GeneratedField::Height),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "timestampMillis" | "timestamp_millis" => Ok(GeneratedField::TimestampMillis),
                            "l1Head" | "l1_head" => Ok(GeneratedField::L1Head),
                            "l1Finalized" | "l1_finalized" => Ok(GeneratedField::L1Finalized),
                            "payloadCommitment" | "payload_commitment" => Ok(GeneratedField::PayloadCommitment),
                            "builderCommitment" | "builder_commitment" => Ok(GeneratedField::BuilderCommitment),
                            "nsTable" | "ns_table" => Ok(GeneratedField::NsTable),
                            "blockMerkleTreeRoot" | "block_merkle_tree_root" => Ok(GeneratedField::BlockMerkleTreeRoot),
                            "feeMerkleTreeRoot" | "fee_merkle_tree_root" => Ok(GeneratedField::FeeMerkleTreeRoot),
                            "feeInfo" | "fee_info" => Ok(GeneratedField::FeeInfo),
                            "builderSignature" | "builder_signature" => Ok(GeneratedField::BuilderSignature),
                            "rewardMerkleTreeRoot" | "reward_merkle_tree_root" => Ok(GeneratedField::RewardMerkleTreeRoot),
                            "totalRewardDistributed" | "total_reward_distributed" => Ok(GeneratedField::TotalRewardDistributed),
                            "nextStakeTableHash" | "next_stake_table_hash" => Ok(GeneratedField::NextStakeTableHash),
                            "leaderCounts" | "leader_counts" => Ok(GeneratedField::LeaderCounts),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HeaderV5;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.HeaderV5")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<HeaderV5, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_config__ = None;
                let mut height__ = None;
                let mut timestamp__ = None;
                let mut timestamp_millis__ = None;
                let mut l1_head__ = None;
                let mut l1_finalized__ = None;
                let mut payload_commitment__ = None;
                let mut builder_commitment__ = None;
                let mut ns_table__ = None;
                let mut block_merkle_tree_root__ = None;
                let mut fee_merkle_tree_root__ = None;
                let mut fee_info__ = None;
                let mut builder_signature__ = None;
                let mut reward_merkle_tree_root__ = None;
                let mut total_reward_distributed__ = None;
                let mut next_stake_table_hash__ = None;
                let mut leader_counts__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ChainConfig => {
                            if chain_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainConfig"));
                            }
                            chain_config__ = map_.next_value()?;
                        }
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::TimestampMillis => {
                            if timestamp_millis__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestampMillis"));
                            }
                            timestamp_millis__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::L1Head => {
                            if l1_head__.is_some() {
                                return Err(serde::de::Error::duplicate_field("l1Head"));
                            }
                            l1_head__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::L1Finalized => {
                            if l1_finalized__.is_some() {
                                return Err(serde::de::Error::duplicate_field("l1Finalized"));
                            }
                            l1_finalized__ = map_.next_value()?;
                        }
                        GeneratedField::PayloadCommitment => {
                            if payload_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadCommitment"));
                            }
                            payload_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BuilderCommitment => {
                            if builder_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderCommitment"));
                            }
                            builder_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NsTable => {
                            if ns_table__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsTable"));
                            }
                            ns_table__ = map_.next_value()?;
                        }
                        GeneratedField::BlockMerkleTreeRoot => {
                            if block_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockMerkleTreeRoot"));
                            }
                            block_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeMerkleTreeRoot => {
                            if fee_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeMerkleTreeRoot"));
                            }
                            fee_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeInfo => {
                            if fee_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeInfo"));
                            }
                            fee_info__ = map_.next_value()?;
                        }
                        GeneratedField::BuilderSignature => {
                            if builder_signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderSignature"));
                            }
                            builder_signature__ = map_.next_value()?;
                        }
                        GeneratedField::RewardMerkleTreeRoot => {
                            if reward_merkle_tree_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rewardMerkleTreeRoot"));
                            }
                            reward_merkle_tree_root__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TotalRewardDistributed => {
                            if total_reward_distributed__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalRewardDistributed"));
                            }
                            total_reward_distributed__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NextStakeTableHash => {
                            if next_stake_table_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextStakeTableHash"));
                            }
                            next_stake_table_hash__ = map_.next_value()?;
                        }
                        GeneratedField::LeaderCounts => {
                            if leader_counts__.is_some() {
                                return Err(serde::de::Error::duplicate_field("leaderCounts"));
                            }
                            leader_counts__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                    }
                }
                Ok(HeaderV5 {
                    chain_config: chain_config__,
                    height: height__.unwrap_or_default(),
                    timestamp: timestamp__.unwrap_or_default(),
                    timestamp_millis: timestamp_millis__.unwrap_or_default(),
                    l1_head: l1_head__.unwrap_or_default(),
                    l1_finalized: l1_finalized__,
                    payload_commitment: payload_commitment__.unwrap_or_default(),
                    builder_commitment: builder_commitment__.unwrap_or_default(),
                    ns_table: ns_table__,
                    block_merkle_tree_root: block_merkle_tree_root__.unwrap_or_default(),
                    fee_merkle_tree_root: fee_merkle_tree_root__.unwrap_or_default(),
                    fee_info: fee_info__,
                    builder_signature: builder_signature__,
                    reward_merkle_tree_root: reward_merkle_tree_root__.unwrap_or_default(),
                    total_reward_distributed: total_reward_distributed__.unwrap_or_default(),
                    next_stake_table_hash: next_stake_table_hash__,
                    leader_counts: leader_counts__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.HeaderV5", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HotshotConfigResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.start_threshold_numerator != 0 {
            len += 1;
        }
        if self.start_threshold_denominator != 0 {
            len += 1;
        }
        if self.num_nodes_with_stake != 0 {
            len += 1;
        }
        if self.da_staked_committee_size != 0 {
            len += 1;
        }
        if self.next_view_timeout_ms != 0 {
            len += 1;
        }
        if self.view_sync_timeout_ms != 0 {
            len += 1;
        }
        if self.builder_timeout_ms != 0 {
            len += 1;
        }
        if self.data_request_delay_ms != 0 {
            len += 1;
        }
        if !self.builder_urls.is_empty() {
            len += 1;
        }
        if self.start_proposing_view != 0 {
            len += 1;
        }
        if self.stop_proposing_view != 0 {
            len += 1;
        }
        if self.start_voting_view != 0 {
            len += 1;
        }
        if self.stop_voting_view != 0 {
            len += 1;
        }
        if self.start_proposing_time != 0 {
            len += 1;
        }
        if self.stop_proposing_time != 0 {
            len += 1;
        }
        if self.start_voting_time != 0 {
            len += 1;
        }
        if self.stop_voting_time != 0 {
            len += 1;
        }
        if self.epoch_height != 0 {
            len += 1;
        }
        if self.epoch_start_block != 0 {
            len += 1;
        }
        if self.stake_table_capacity != 0 {
            len += 1;
        }
        if self.drb_difficulty != 0 {
            len += 1;
        }
        if self.drb_upgrade_difficulty != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.HotshotConfigResponse", len)?;
        if self.start_threshold_numerator != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("startThresholdNumerator", ToString::to_string(&self.start_threshold_numerator).as_str())?;
        }
        if self.start_threshold_denominator != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("startThresholdDenominator", ToString::to_string(&self.start_threshold_denominator).as_str())?;
        }
        if self.num_nodes_with_stake != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("numNodesWithStake", ToString::to_string(&self.num_nodes_with_stake).as_str())?;
        }
        if self.da_staked_committee_size != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("daStakedCommitteeSize", ToString::to_string(&self.da_staked_committee_size).as_str())?;
        }
        if self.next_view_timeout_ms != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nextViewTimeoutMs", ToString::to_string(&self.next_view_timeout_ms).as_str())?;
        }
        if self.view_sync_timeout_ms != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("viewSyncTimeoutMs", ToString::to_string(&self.view_sync_timeout_ms).as_str())?;
        }
        if self.builder_timeout_ms != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("builderTimeoutMs", ToString::to_string(&self.builder_timeout_ms).as_str())?;
        }
        if self.data_request_delay_ms != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("dataRequestDelayMs", ToString::to_string(&self.data_request_delay_ms).as_str())?;
        }
        if !self.builder_urls.is_empty() {
            struct_ser.serialize_field("builderUrls", &self.builder_urls)?;
        }
        if self.start_proposing_view != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("startProposingView", ToString::to_string(&self.start_proposing_view).as_str())?;
        }
        if self.stop_proposing_view != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("stopProposingView", ToString::to_string(&self.stop_proposing_view).as_str())?;
        }
        if self.start_voting_view != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("startVotingView", ToString::to_string(&self.start_voting_view).as_str())?;
        }
        if self.stop_voting_view != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("stopVotingView", ToString::to_string(&self.stop_voting_view).as_str())?;
        }
        if self.start_proposing_time != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("startProposingTime", ToString::to_string(&self.start_proposing_time).as_str())?;
        }
        if self.stop_proposing_time != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("stopProposingTime", ToString::to_string(&self.stop_proposing_time).as_str())?;
        }
        if self.start_voting_time != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("startVotingTime", ToString::to_string(&self.start_voting_time).as_str())?;
        }
        if self.stop_voting_time != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("stopVotingTime", ToString::to_string(&self.stop_voting_time).as_str())?;
        }
        if self.epoch_height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epochHeight", ToString::to_string(&self.epoch_height).as_str())?;
        }
        if self.epoch_start_block != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epochStartBlock", ToString::to_string(&self.epoch_start_block).as_str())?;
        }
        if self.stake_table_capacity != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("stakeTableCapacity", ToString::to_string(&self.stake_table_capacity).as_str())?;
        }
        if self.drb_difficulty != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("drbDifficulty", ToString::to_string(&self.drb_difficulty).as_str())?;
        }
        if self.drb_upgrade_difficulty != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("drbUpgradeDifficulty", ToString::to_string(&self.drb_upgrade_difficulty).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HotshotConfigResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start_threshold_numerator",
            "startThresholdNumerator",
            "start_threshold_denominator",
            "startThresholdDenominator",
            "num_nodes_with_stake",
            "numNodesWithStake",
            "da_staked_committee_size",
            "daStakedCommitteeSize",
            "next_view_timeout_ms",
            "nextViewTimeoutMs",
            "view_sync_timeout_ms",
            "viewSyncTimeoutMs",
            "builder_timeout_ms",
            "builderTimeoutMs",
            "data_request_delay_ms",
            "dataRequestDelayMs",
            "builder_urls",
            "builderUrls",
            "start_proposing_view",
            "startProposingView",
            "stop_proposing_view",
            "stopProposingView",
            "start_voting_view",
            "startVotingView",
            "stop_voting_view",
            "stopVotingView",
            "start_proposing_time",
            "startProposingTime",
            "stop_proposing_time",
            "stopProposingTime",
            "start_voting_time",
            "startVotingTime",
            "stop_voting_time",
            "stopVotingTime",
            "epoch_height",
            "epochHeight",
            "epoch_start_block",
            "epochStartBlock",
            "stake_table_capacity",
            "stakeTableCapacity",
            "drb_difficulty",
            "drbDifficulty",
            "drb_upgrade_difficulty",
            "drbUpgradeDifficulty",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartThresholdNumerator,
            StartThresholdDenominator,
            NumNodesWithStake,
            DaStakedCommitteeSize,
            NextViewTimeoutMs,
            ViewSyncTimeoutMs,
            BuilderTimeoutMs,
            DataRequestDelayMs,
            BuilderUrls,
            StartProposingView,
            StopProposingView,
            StartVotingView,
            StopVotingView,
            StartProposingTime,
            StopProposingTime,
            StartVotingTime,
            StopVotingTime,
            EpochHeight,
            EpochStartBlock,
            StakeTableCapacity,
            DrbDifficulty,
            DrbUpgradeDifficulty,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "startThresholdNumerator" | "start_threshold_numerator" => Ok(GeneratedField::StartThresholdNumerator),
                            "startThresholdDenominator" | "start_threshold_denominator" => Ok(GeneratedField::StartThresholdDenominator),
                            "numNodesWithStake" | "num_nodes_with_stake" => Ok(GeneratedField::NumNodesWithStake),
                            "daStakedCommitteeSize" | "da_staked_committee_size" => Ok(GeneratedField::DaStakedCommitteeSize),
                            "nextViewTimeoutMs" | "next_view_timeout_ms" => Ok(GeneratedField::NextViewTimeoutMs),
                            "viewSyncTimeoutMs" | "view_sync_timeout_ms" => Ok(GeneratedField::ViewSyncTimeoutMs),
                            "builderTimeoutMs" | "builder_timeout_ms" => Ok(GeneratedField::BuilderTimeoutMs),
                            "dataRequestDelayMs" | "data_request_delay_ms" => Ok(GeneratedField::DataRequestDelayMs),
                            "builderUrls" | "builder_urls" => Ok(GeneratedField::BuilderUrls),
                            "startProposingView" | "start_proposing_view" => Ok(GeneratedField::StartProposingView),
                            "stopProposingView" | "stop_proposing_view" => Ok(GeneratedField::StopProposingView),
                            "startVotingView" | "start_voting_view" => Ok(GeneratedField::StartVotingView),
                            "stopVotingView" | "stop_voting_view" => Ok(GeneratedField::StopVotingView),
                            "startProposingTime" | "start_proposing_time" => Ok(GeneratedField::StartProposingTime),
                            "stopProposingTime" | "stop_proposing_time" => Ok(GeneratedField::StopProposingTime),
                            "startVotingTime" | "start_voting_time" => Ok(GeneratedField::StartVotingTime),
                            "stopVotingTime" | "stop_voting_time" => Ok(GeneratedField::StopVotingTime),
                            "epochHeight" | "epoch_height" => Ok(GeneratedField::EpochHeight),
                            "epochStartBlock" | "epoch_start_block" => Ok(GeneratedField::EpochStartBlock),
                            "stakeTableCapacity" | "stake_table_capacity" => Ok(GeneratedField::StakeTableCapacity),
                            "drbDifficulty" | "drb_difficulty" => Ok(GeneratedField::DrbDifficulty),
                            "drbUpgradeDifficulty" | "drb_upgrade_difficulty" => Ok(GeneratedField::DrbUpgradeDifficulty),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HotshotConfigResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.HotshotConfigResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<HotshotConfigResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start_threshold_numerator__ = None;
                let mut start_threshold_denominator__ = None;
                let mut num_nodes_with_stake__ = None;
                let mut da_staked_committee_size__ = None;
                let mut next_view_timeout_ms__ = None;
                let mut view_sync_timeout_ms__ = None;
                let mut builder_timeout_ms__ = None;
                let mut data_request_delay_ms__ = None;
                let mut builder_urls__ = None;
                let mut start_proposing_view__ = None;
                let mut stop_proposing_view__ = None;
                let mut start_voting_view__ = None;
                let mut stop_voting_view__ = None;
                let mut start_proposing_time__ = None;
                let mut stop_proposing_time__ = None;
                let mut start_voting_time__ = None;
                let mut stop_voting_time__ = None;
                let mut epoch_height__ = None;
                let mut epoch_start_block__ = None;
                let mut stake_table_capacity__ = None;
                let mut drb_difficulty__ = None;
                let mut drb_upgrade_difficulty__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::StartThresholdNumerator => {
                            if start_threshold_numerator__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startThresholdNumerator"));
                            }
                            start_threshold_numerator__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StartThresholdDenominator => {
                            if start_threshold_denominator__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startThresholdDenominator"));
                            }
                            start_threshold_denominator__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NumNodesWithStake => {
                            if num_nodes_with_stake__.is_some() {
                                return Err(serde::de::Error::duplicate_field("numNodesWithStake"));
                            }
                            num_nodes_with_stake__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DaStakedCommitteeSize => {
                            if da_staked_committee_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("daStakedCommitteeSize"));
                            }
                            da_staked_committee_size__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NextViewTimeoutMs => {
                            if next_view_timeout_ms__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextViewTimeoutMs"));
                            }
                            next_view_timeout_ms__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ViewSyncTimeoutMs => {
                            if view_sync_timeout_ms__.is_some() {
                                return Err(serde::de::Error::duplicate_field("viewSyncTimeoutMs"));
                            }
                            view_sync_timeout_ms__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BuilderTimeoutMs => {
                            if builder_timeout_ms__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderTimeoutMs"));
                            }
                            builder_timeout_ms__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DataRequestDelayMs => {
                            if data_request_delay_ms__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dataRequestDelayMs"));
                            }
                            data_request_delay_ms__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BuilderUrls => {
                            if builder_urls__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderUrls"));
                            }
                            builder_urls__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StartProposingView => {
                            if start_proposing_view__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startProposingView"));
                            }
                            start_proposing_view__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StopProposingView => {
                            if stop_proposing_view__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stopProposingView"));
                            }
                            stop_proposing_view__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StartVotingView => {
                            if start_voting_view__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startVotingView"));
                            }
                            start_voting_view__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StopVotingView => {
                            if stop_voting_view__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stopVotingView"));
                            }
                            stop_voting_view__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StartProposingTime => {
                            if start_proposing_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startProposingTime"));
                            }
                            start_proposing_time__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StopProposingTime => {
                            if stop_proposing_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stopProposingTime"));
                            }
                            stop_proposing_time__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StartVotingTime => {
                            if start_voting_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startVotingTime"));
                            }
                            start_voting_time__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StopVotingTime => {
                            if stop_voting_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stopVotingTime"));
                            }
                            stop_voting_time__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::EpochHeight => {
                            if epoch_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epochHeight"));
                            }
                            epoch_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::EpochStartBlock => {
                            if epoch_start_block__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epochStartBlock"));
                            }
                            epoch_start_block__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StakeTableCapacity => {
                            if stake_table_capacity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stakeTableCapacity"));
                            }
                            stake_table_capacity__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DrbDifficulty => {
                            if drb_difficulty__.is_some() {
                                return Err(serde::de::Error::duplicate_field("drbDifficulty"));
                            }
                            drb_difficulty__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DrbUpgradeDifficulty => {
                            if drb_upgrade_difficulty__.is_some() {
                                return Err(serde::de::Error::duplicate_field("drbUpgradeDifficulty"));
                            }
                            drb_upgrade_difficulty__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(HotshotConfigResponse {
                    start_threshold_numerator: start_threshold_numerator__.unwrap_or_default(),
                    start_threshold_denominator: start_threshold_denominator__.unwrap_or_default(),
                    num_nodes_with_stake: num_nodes_with_stake__.unwrap_or_default(),
                    da_staked_committee_size: da_staked_committee_size__.unwrap_or_default(),
                    next_view_timeout_ms: next_view_timeout_ms__.unwrap_or_default(),
                    view_sync_timeout_ms: view_sync_timeout_ms__.unwrap_or_default(),
                    builder_timeout_ms: builder_timeout_ms__.unwrap_or_default(),
                    data_request_delay_ms: data_request_delay_ms__.unwrap_or_default(),
                    builder_urls: builder_urls__.unwrap_or_default(),
                    start_proposing_view: start_proposing_view__.unwrap_or_default(),
                    stop_proposing_view: stop_proposing_view__.unwrap_or_default(),
                    start_voting_view: start_voting_view__.unwrap_or_default(),
                    stop_voting_view: stop_voting_view__.unwrap_or_default(),
                    start_proposing_time: start_proposing_time__.unwrap_or_default(),
                    stop_proposing_time: stop_proposing_time__.unwrap_or_default(),
                    start_voting_time: start_voting_time__.unwrap_or_default(),
                    stop_voting_time: stop_voting_time__.unwrap_or_default(),
                    epoch_height: epoch_height__.unwrap_or_default(),
                    epoch_start_block: epoch_start_block__.unwrap_or_default(),
                    stake_table_capacity: stake_table_capacity__.unwrap_or_default(),
                    drb_difficulty: drb_difficulty__.unwrap_or_default(),
                    drb_upgrade_difficulty: drb_upgrade_difficulty__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.HotshotConfigResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for L1BlockInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.number != 0 {
            len += 1;
        }
        if !self.timestamp.is_empty() {
            len += 1;
        }
        if !self.hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.L1BlockInfo", len)?;
        if self.number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("number", ToString::to_string(&self.number).as_str())?;
        }
        if !self.timestamp.is_empty() {
            struct_ser.serialize_field("timestamp", &self.timestamp)?;
        }
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", &self.hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for L1BlockInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "number",
            "timestamp",
            "hash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Number,
            Timestamp,
            Hash,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "number" => Ok(GeneratedField::Number),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "hash" => Ok(GeneratedField::Hash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = L1BlockInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.L1BlockInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<L1BlockInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut number__ = None;
                let mut timestamp__ = None;
                let mut hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Number => {
                            if number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("number"));
                            }
                            number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(L1BlockInfo {
                    number: number__.unwrap_or_default(),
                    timestamp: timestamp__.unwrap_or_default(),
                    hash: hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.L1BlockInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LargeRangeProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.prefix_elems.is_empty() {
            len += 1;
        }
        if !self.suffix_elems.is_empty() {
            len += 1;
        }
        if !self.prefix_bytes.is_empty() {
            len += 1;
        }
        if !self.suffix_bytes.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.LargeRangeProof", len)?;
        if !self.prefix_elems.is_empty() {
            struct_ser.serialize_field("prefixElems", &self.prefix_elems)?;
        }
        if !self.suffix_elems.is_empty() {
            struct_ser.serialize_field("suffixElems", &self.suffix_elems)?;
        }
        if !self.prefix_bytes.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("prefixBytes", pbjson::private::base64::encode(&self.prefix_bytes).as_str())?;
        }
        if !self.suffix_bytes.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("suffixBytes", pbjson::private::base64::encode(&self.suffix_bytes).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LargeRangeProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "prefix_elems",
            "prefixElems",
            "suffix_elems",
            "suffixElems",
            "prefix_bytes",
            "prefixBytes",
            "suffix_bytes",
            "suffixBytes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PrefixElems,
            SuffixElems,
            PrefixBytes,
            SuffixBytes,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "prefixElems" | "prefix_elems" => Ok(GeneratedField::PrefixElems),
                            "suffixElems" | "suffix_elems" => Ok(GeneratedField::SuffixElems),
                            "prefixBytes" | "prefix_bytes" => Ok(GeneratedField::PrefixBytes),
                            "suffixBytes" | "suffix_bytes" => Ok(GeneratedField::SuffixBytes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LargeRangeProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.LargeRangeProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<LargeRangeProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut prefix_elems__ = None;
                let mut suffix_elems__ = None;
                let mut prefix_bytes__ = None;
                let mut suffix_bytes__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PrefixElems => {
                            if prefix_elems__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prefixElems"));
                            }
                            prefix_elems__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SuffixElems => {
                            if suffix_elems__.is_some() {
                                return Err(serde::de::Error::duplicate_field("suffixElems"));
                            }
                            suffix_elems__ = Some(map_.next_value()?);
                        }
                        GeneratedField::PrefixBytes => {
                            if prefix_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prefixBytes"));
                            }
                            prefix_bytes__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::SuffixBytes => {
                            if suffix_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("suffixBytes"));
                            }
                            suffix_bytes__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(LargeRangeProof {
                    prefix_elems: prefix_elems__.unwrap_or_default(),
                    suffix_elems: suffix_elems__.unwrap_or_default(),
                    prefix_bytes: prefix_bytes__.unwrap_or_default(),
                    suffix_bytes: suffix_bytes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.LargeRangeProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Leaf2 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.view_number != 0 {
            len += 1;
        }
        if self.justify_qc.is_some() {
            len += 1;
        }
        if self.next_epoch_justify_qc.is_some() {
            len += 1;
        }
        if !self.parent_commitment.is_empty() {
            len += 1;
        }
        if self.block_header.is_some() {
            len += 1;
        }
        if self.upgrade_certificate.is_some() {
            len += 1;
        }
        if self.block_payload.is_some() {
            len += 1;
        }
        if self.view_change_evidence.is_some() {
            len += 1;
        }
        if self.next_drb_result.is_some() {
            len += 1;
        }
        if self.with_epoch {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.Leaf2", len)?;
        if self.view_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("viewNumber", ToString::to_string(&self.view_number).as_str())?;
        }
        if let Some(v) = self.justify_qc.as_ref() {
            struct_ser.serialize_field("justifyQc", v)?;
        }
        if let Some(v) = self.next_epoch_justify_qc.as_ref() {
            struct_ser.serialize_field("nextEpochJustifyQc", v)?;
        }
        if !self.parent_commitment.is_empty() {
            struct_ser.serialize_field("parentCommitment", &self.parent_commitment)?;
        }
        if let Some(v) = self.block_header.as_ref() {
            struct_ser.serialize_field("blockHeader", v)?;
        }
        if let Some(v) = self.upgrade_certificate.as_ref() {
            struct_ser.serialize_field("upgradeCertificate", v)?;
        }
        if let Some(v) = self.block_payload.as_ref() {
            struct_ser.serialize_field("blockPayload", v)?;
        }
        if let Some(v) = self.view_change_evidence.as_ref() {
            struct_ser.serialize_field("viewChangeEvidence", v)?;
        }
        if let Some(v) = self.next_drb_result.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nextDrbResult", pbjson::private::base64::encode(&v).as_str())?;
        }
        if self.with_epoch {
            struct_ser.serialize_field("withEpoch", &self.with_epoch)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Leaf2 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "view_number",
            "viewNumber",
            "justify_qc",
            "justifyQc",
            "next_epoch_justify_qc",
            "nextEpochJustifyQc",
            "parent_commitment",
            "parentCommitment",
            "block_header",
            "blockHeader",
            "upgrade_certificate",
            "upgradeCertificate",
            "block_payload",
            "blockPayload",
            "view_change_evidence",
            "viewChangeEvidence",
            "next_drb_result",
            "nextDrbResult",
            "with_epoch",
            "withEpoch",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ViewNumber,
            JustifyQc,
            NextEpochJustifyQc,
            ParentCommitment,
            BlockHeader,
            UpgradeCertificate,
            BlockPayload,
            ViewChangeEvidence,
            NextDrbResult,
            WithEpoch,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "viewNumber" | "view_number" => Ok(GeneratedField::ViewNumber),
                            "justifyQc" | "justify_qc" => Ok(GeneratedField::JustifyQc),
                            "nextEpochJustifyQc" | "next_epoch_justify_qc" => Ok(GeneratedField::NextEpochJustifyQc),
                            "parentCommitment" | "parent_commitment" => Ok(GeneratedField::ParentCommitment),
                            "blockHeader" | "block_header" => Ok(GeneratedField::BlockHeader),
                            "upgradeCertificate" | "upgrade_certificate" => Ok(GeneratedField::UpgradeCertificate),
                            "blockPayload" | "block_payload" => Ok(GeneratedField::BlockPayload),
                            "viewChangeEvidence" | "view_change_evidence" => Ok(GeneratedField::ViewChangeEvidence),
                            "nextDrbResult" | "next_drb_result" => Ok(GeneratedField::NextDrbResult),
                            "withEpoch" | "with_epoch" => Ok(GeneratedField::WithEpoch),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Leaf2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.Leaf2")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Leaf2, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut view_number__ = None;
                let mut justify_qc__ = None;
                let mut next_epoch_justify_qc__ = None;
                let mut parent_commitment__ = None;
                let mut block_header__ = None;
                let mut upgrade_certificate__ = None;
                let mut block_payload__ = None;
                let mut view_change_evidence__ = None;
                let mut next_drb_result__ = None;
                let mut with_epoch__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ViewNumber => {
                            if view_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("viewNumber"));
                            }
                            view_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::JustifyQc => {
                            if justify_qc__.is_some() {
                                return Err(serde::de::Error::duplicate_field("justifyQc"));
                            }
                            justify_qc__ = map_.next_value()?;
                        }
                        GeneratedField::NextEpochJustifyQc => {
                            if next_epoch_justify_qc__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextEpochJustifyQc"));
                            }
                            next_epoch_justify_qc__ = map_.next_value()?;
                        }
                        GeneratedField::ParentCommitment => {
                            if parent_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parentCommitment"));
                            }
                            parent_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BlockHeader => {
                            if block_header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHeader"));
                            }
                            block_header__ = map_.next_value()?;
                        }
                        GeneratedField::UpgradeCertificate => {
                            if upgrade_certificate__.is_some() {
                                return Err(serde::de::Error::duplicate_field("upgradeCertificate"));
                            }
                            upgrade_certificate__ = map_.next_value()?;
                        }
                        GeneratedField::BlockPayload => {
                            if block_payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockPayload"));
                            }
                            block_payload__ = map_.next_value()?;
                        }
                        GeneratedField::ViewChangeEvidence => {
                            if view_change_evidence__.is_some() {
                                return Err(serde::de::Error::duplicate_field("viewChangeEvidence"));
                            }
                            view_change_evidence__ = map_.next_value()?;
                        }
                        GeneratedField::NextDrbResult => {
                            if next_drb_result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextDrbResult"));
                            }
                            next_drb_result__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::WithEpoch => {
                            if with_epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("withEpoch"));
                            }
                            with_epoch__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Leaf2 {
                    view_number: view_number__.unwrap_or_default(),
                    justify_qc: justify_qc__,
                    next_epoch_justify_qc: next_epoch_justify_qc__,
                    parent_commitment: parent_commitment__.unwrap_or_default(),
                    block_header: block_header__,
                    upgrade_certificate: upgrade_certificate__,
                    block_payload: block_payload__,
                    view_change_evidence: view_change_evidence__,
                    next_drb_result: next_drb_result__,
                    with_epoch: with_epoch__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.Leaf2", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LeafRangeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.leaves.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.LeafRangeResponse", len)?;
        if !self.leaves.is_empty() {
            struct_ser.serialize_field("leaves", &self.leaves)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LeafRangeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "leaves",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Leaves,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "leaves" => Ok(GeneratedField::Leaves),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LeafRangeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.LeafRangeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<LeafRangeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut leaves__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Leaves => {
                            if leaves__.is_some() {
                                return Err(serde::de::Error::duplicate_field("leaves"));
                            }
                            leaves__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(LeafRangeResponse {
                    leaves: leaves__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.LeafRangeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LeafResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.leaf.is_some() {
            len += 1;
        }
        if self.qc.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.LeafResponse", len)?;
        if let Some(v) = self.leaf.as_ref() {
            struct_ser.serialize_field("leaf", v)?;
        }
        if let Some(v) = self.qc.as_ref() {
            struct_ser.serialize_field("qc", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LeafResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "leaf",
            "qc",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Leaf,
            Qc,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "leaf" => Ok(GeneratedField::Leaf),
                            "qc" => Ok(GeneratedField::Qc),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LeafResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.LeafResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<LeafResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut leaf__ = None;
                let mut qc__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Leaf => {
                            if leaf__.is_some() {
                                return Err(serde::de::Error::duplicate_field("leaf"));
                            }
                            leaf__ = map_.next_value()?;
                        }
                        GeneratedField::Qc => {
                            if qc__.is_some() {
                                return Err(serde::de::Error::duplicate_field("qc"));
                            }
                            qc__ = map_.next_value()?;
                        }
                    }
                }
                Ok(LeafResponse {
                    leaf: leaf__,
                    qc: qc__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.LeafResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LimitsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.small_object_range_limit != 0 {
            len += 1;
        }
        if self.large_object_range_limit != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.LimitsResponse", len)?;
        if self.small_object_range_limit != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("smallObjectRangeLimit", ToString::to_string(&self.small_object_range_limit).as_str())?;
        }
        if self.large_object_range_limit != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("largeObjectRangeLimit", ToString::to_string(&self.large_object_range_limit).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LimitsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "small_object_range_limit",
            "smallObjectRangeLimit",
            "large_object_range_limit",
            "largeObjectRangeLimit",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SmallObjectRangeLimit,
            LargeObjectRangeLimit,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "smallObjectRangeLimit" | "small_object_range_limit" => Ok(GeneratedField::SmallObjectRangeLimit),
                            "largeObjectRangeLimit" | "large_object_range_limit" => Ok(GeneratedField::LargeObjectRangeLimit),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LimitsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.LimitsResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<LimitsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut small_object_range_limit__ = None;
                let mut large_object_range_limit__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SmallObjectRangeLimit => {
                            if small_object_range_limit__.is_some() {
                                return Err(serde::de::Error::duplicate_field("smallObjectRangeLimit"));
                            }
                            small_object_range_limit__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::LargeObjectRangeLimit => {
                            if large_object_range_limit__.is_some() {
                                return Err(serde::de::Error::duplicate_field("largeObjectRangeLimit"));
                            }
                            large_object_range_limit__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(LimitsResponse {
                    small_object_range_limit: small_object_range_limit__.unwrap_or_default(),
                    large_object_range_limit: large_object_range_limit__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.LimitsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MigrationStatus {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.started_at.is_empty() {
            len += 1;
        }
        if self.completed_at.is_some() {
            len += 1;
        }
        if self.last_offset.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.MigrationStatus", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.started_at.is_empty() {
            struct_ser.serialize_field("startedAt", &self.started_at)?;
        }
        if let Some(v) = self.completed_at.as_ref() {
            struct_ser.serialize_field("completedAt", v)?;
        }
        if let Some(v) = self.last_offset.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("lastOffset", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MigrationStatus {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "started_at",
            "startedAt",
            "completed_at",
            "completedAt",
            "last_offset",
            "lastOffset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            StartedAt,
            CompletedAt,
            LastOffset,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "startedAt" | "started_at" => Ok(GeneratedField::StartedAt),
                            "completedAt" | "completed_at" => Ok(GeneratedField::CompletedAt),
                            "lastOffset" | "last_offset" => Ok(GeneratedField::LastOffset),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MigrationStatus;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.MigrationStatus")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MigrationStatus, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut started_at__ = None;
                let mut completed_at__ = None;
                let mut last_offset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StartedAt => {
                            if started_at__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startedAt"));
                            }
                            started_at__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CompletedAt => {
                            if completed_at__.is_some() {
                                return Err(serde::de::Error::duplicate_field("completedAt"));
                            }
                            completed_at__ = map_.next_value()?;
                        }
                        GeneratedField::LastOffset => {
                            if last_offset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lastOffset"));
                            }
                            last_offset__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(MigrationStatus {
                    name: name__.unwrap_or_default(),
                    started_at: started_at__.unwrap_or_default(),
                    completed_at: completed_at__,
                    last_offset: last_offset__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.MigrationStatus", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MigrationStatusResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.migrations.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.MigrationStatusResponse", len)?;
        if !self.migrations.is_empty() {
            struct_ser.serialize_field("migrations", &self.migrations)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MigrationStatusResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "migrations",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Migrations,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "migrations" => Ok(GeneratedField::Migrations),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MigrationStatusResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.MigrationStatusResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MigrationStatusResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut migrations__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Migrations => {
                            if migrations__.is_some() {
                                return Err(serde::de::Error::duplicate_field("migrations"));
                            }
                            migrations__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(MigrationStatusResponse {
                    migrations: migrations__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.MigrationStatusResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NamespaceProofRangeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.proofs.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.NamespaceProofRangeResponse", len)?;
        if !self.proofs.is_empty() {
            struct_ser.serialize_field("proofs", &self.proofs)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NamespaceProofRangeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "proofs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Proofs,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "proofs" => Ok(GeneratedField::Proofs),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NamespaceProofRangeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.NamespaceProofRangeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NamespaceProofRangeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut proofs__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Proofs => {
                            if proofs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proofs"));
                            }
                            proofs__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(NamespaceProofRangeResponse {
                    proofs: proofs__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.NamespaceProofRangeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NamespaceProofResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.proof.is_some() {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.NamespaceProofResponse", len)?;
        if let Some(v) = self.proof.as_ref() {
            struct_ser.serialize_field("proof", v)?;
        }
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NamespaceProofResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "proof",
            "transactions",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Proof,
            Transactions,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "proof" => Ok(GeneratedField::Proof),
                            "transactions" => Ok(GeneratedField::Transactions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NamespaceProofResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.NamespaceProofResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NamespaceProofResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut proof__ = None;
                let mut transactions__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Proof => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proof"));
                            }
                            proof__ = map_.next_value()?;
                        }
                        GeneratedField::Transactions => {
                            if transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactions"));
                            }
                            transactions__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(NamespaceProofResponse {
                    proof: proof__,
                    transactions: transactions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.NamespaceProofResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NodeIdentity {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.node_name.is_some() {
            len += 1;
        }
        if self.node_description.is_some() {
            len += 1;
        }
        if self.company_name.is_some() {
            len += 1;
        }
        if self.company_website.is_some() {
            len += 1;
        }
        if self.country_code.is_some() {
            len += 1;
        }
        if self.latitude.is_some() {
            len += 1;
        }
        if self.longitude.is_some() {
            len += 1;
        }
        if self.operating_system.is_some() {
            len += 1;
        }
        if self.node_type.is_some() {
            len += 1;
        }
        if self.network_type.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.NodeIdentity", len)?;
        if let Some(v) = self.node_name.as_ref() {
            struct_ser.serialize_field("nodeName", v)?;
        }
        if let Some(v) = self.node_description.as_ref() {
            struct_ser.serialize_field("nodeDescription", v)?;
        }
        if let Some(v) = self.company_name.as_ref() {
            struct_ser.serialize_field("companyName", v)?;
        }
        if let Some(v) = self.company_website.as_ref() {
            struct_ser.serialize_field("companyWebsite", v)?;
        }
        if let Some(v) = self.country_code.as_ref() {
            struct_ser.serialize_field("countryCode", v)?;
        }
        if let Some(v) = self.latitude.as_ref() {
            struct_ser.serialize_field("latitude", v)?;
        }
        if let Some(v) = self.longitude.as_ref() {
            struct_ser.serialize_field("longitude", v)?;
        }
        if let Some(v) = self.operating_system.as_ref() {
            struct_ser.serialize_field("operatingSystem", v)?;
        }
        if let Some(v) = self.node_type.as_ref() {
            struct_ser.serialize_field("nodeType", v)?;
        }
        if let Some(v) = self.network_type.as_ref() {
            struct_ser.serialize_field("networkType", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NodeIdentity {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "node_name",
            "nodeName",
            "node_description",
            "nodeDescription",
            "company_name",
            "companyName",
            "company_website",
            "companyWebsite",
            "country_code",
            "countryCode",
            "latitude",
            "longitude",
            "operating_system",
            "operatingSystem",
            "node_type",
            "nodeType",
            "network_type",
            "networkType",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            NodeName,
            NodeDescription,
            CompanyName,
            CompanyWebsite,
            CountryCode,
            Latitude,
            Longitude,
            OperatingSystem,
            NodeType,
            NetworkType,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "nodeName" | "node_name" => Ok(GeneratedField::NodeName),
                            "nodeDescription" | "node_description" => Ok(GeneratedField::NodeDescription),
                            "companyName" | "company_name" => Ok(GeneratedField::CompanyName),
                            "companyWebsite" | "company_website" => Ok(GeneratedField::CompanyWebsite),
                            "countryCode" | "country_code" => Ok(GeneratedField::CountryCode),
                            "latitude" => Ok(GeneratedField::Latitude),
                            "longitude" => Ok(GeneratedField::Longitude),
                            "operatingSystem" | "operating_system" => Ok(GeneratedField::OperatingSystem),
                            "nodeType" | "node_type" => Ok(GeneratedField::NodeType),
                            "networkType" | "network_type" => Ok(GeneratedField::NetworkType),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NodeIdentity;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.NodeIdentity")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NodeIdentity, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut node_name__ = None;
                let mut node_description__ = None;
                let mut company_name__ = None;
                let mut company_website__ = None;
                let mut country_code__ = None;
                let mut latitude__ = None;
                let mut longitude__ = None;
                let mut operating_system__ = None;
                let mut node_type__ = None;
                let mut network_type__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::NodeName => {
                            if node_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nodeName"));
                            }
                            node_name__ = map_.next_value()?;
                        }
                        GeneratedField::NodeDescription => {
                            if node_description__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nodeDescription"));
                            }
                            node_description__ = map_.next_value()?;
                        }
                        GeneratedField::CompanyName => {
                            if company_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("companyName"));
                            }
                            company_name__ = map_.next_value()?;
                        }
                        GeneratedField::CompanyWebsite => {
                            if company_website__.is_some() {
                                return Err(serde::de::Error::duplicate_field("companyWebsite"));
                            }
                            company_website__ = map_.next_value()?;
                        }
                        GeneratedField::CountryCode => {
                            if country_code__.is_some() {
                                return Err(serde::de::Error::duplicate_field("countryCode"));
                            }
                            country_code__ = map_.next_value()?;
                        }
                        GeneratedField::Latitude => {
                            if latitude__.is_some() {
                                return Err(serde::de::Error::duplicate_field("latitude"));
                            }
                            latitude__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Longitude => {
                            if longitude__.is_some() {
                                return Err(serde::de::Error::duplicate_field("longitude"));
                            }
                            longitude__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::OperatingSystem => {
                            if operating_system__.is_some() {
                                return Err(serde::de::Error::duplicate_field("operatingSystem"));
                            }
                            operating_system__ = map_.next_value()?;
                        }
                        GeneratedField::NodeType => {
                            if node_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nodeType"));
                            }
                            node_type__ = map_.next_value()?;
                        }
                        GeneratedField::NetworkType => {
                            if network_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("networkType"));
                            }
                            network_type__ = map_.next_value()?;
                        }
                    }
                }
                Ok(NodeIdentity {
                    node_name: node_name__,
                    node_description: node_description__,
                    company_name: company_name__,
                    company_website: company_website__,
                    country_code: country_code__,
                    latitude: latitude__,
                    longitude: longitude__,
                    operating_system: operating_system__,
                    node_type: node_type__,
                    network_type: network_type__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.NodeIdentity", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NodeKeysResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.eth_account.is_some() {
            len += 1;
        }
        if self.consensus_key.is_some() {
            len += 1;
        }
        if self.state_ver_key.is_some() {
            len += 1;
        }
        if self.x25519_key.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.NodeKeysResponse", len)?;
        if let Some(v) = self.eth_account.as_ref() {
            struct_ser.serialize_field("ethAccount", v)?;
        }
        if let Some(v) = self.consensus_key.as_ref() {
            struct_ser.serialize_field("consensusKey", v)?;
        }
        if let Some(v) = self.state_ver_key.as_ref() {
            struct_ser.serialize_field("stateVerKey", v)?;
        }
        if let Some(v) = self.x25519_key.as_ref() {
            struct_ser.serialize_field("x25519Key", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NodeKeysResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "eth_account",
            "ethAccount",
            "consensus_key",
            "consensusKey",
            "state_ver_key",
            "stateVerKey",
            "x25519_key",
            "x25519Key",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            EthAccount,
            ConsensusKey,
            StateVerKey,
            X25519Key,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "ethAccount" | "eth_account" => Ok(GeneratedField::EthAccount),
                            "consensusKey" | "consensus_key" => Ok(GeneratedField::ConsensusKey),
                            "stateVerKey" | "state_ver_key" => Ok(GeneratedField::StateVerKey),
                            "x25519Key" | "x25519_key" => Ok(GeneratedField::X25519Key),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NodeKeysResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.NodeKeysResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NodeKeysResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut eth_account__ = None;
                let mut consensus_key__ = None;
                let mut state_ver_key__ = None;
                let mut x25519_key__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::EthAccount => {
                            if eth_account__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ethAccount"));
                            }
                            eth_account__ = map_.next_value()?;
                        }
                        GeneratedField::ConsensusKey => {
                            if consensus_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("consensusKey"));
                            }
                            consensus_key__ = map_.next_value()?;
                        }
                        GeneratedField::StateVerKey => {
                            if state_ver_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateVerKey"));
                            }
                            state_ver_key__ = map_.next_value()?;
                        }
                        GeneratedField::X25519Key => {
                            if x25519_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("x25519Key"));
                            }
                            x25519_key__ = map_.next_value()?;
                        }
                    }
                }
                Ok(NodeKeysResponse {
                    eth_account: eth_account__,
                    consensus_key: consensus_key__,
                    state_ver_key: state_ver_key__,
                    x25519_key: x25519_key__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.NodeKeysResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NsProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.NsProof", len)?;
        if let Some(v) = self.proof.as_ref() {
            match v {
                ns_proof::Proof::V0(v) => {
                    struct_ser.serialize_field("v0", v)?;
                }
                ns_proof::Proof::V1(v) => {
                    struct_ser.serialize_field("v1", v)?;
                }
                ns_proof::Proof::V1IncorrectEncoding(v) => {
                    struct_ser.serialize_field("v1IncorrectEncoding", v)?;
                }
                ns_proof::Proof::V2(v) => {
                    struct_ser.serialize_field("v2", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NsProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "v0",
            "v1",
            "v1_incorrect_encoding",
            "v1IncorrectEncoding",
            "v2",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            V0,
            V1,
            V1IncorrectEncoding,
            V2,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "v0" => Ok(GeneratedField::V0),
                            "v1" => Ok(GeneratedField::V1),
                            "v1IncorrectEncoding" | "v1_incorrect_encoding" => Ok(GeneratedField::V1IncorrectEncoding),
                            "v2" => Ok(GeneratedField::V2),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NsProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.NsProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NsProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::V0 => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v0"));
                            }
                            proof__ = map_.next_value::<::std::option::Option<_>>()?.map(ns_proof::Proof::V0)
;
                        }
                        GeneratedField::V1 => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v1"));
                            }
                            proof__ = map_.next_value::<::std::option::Option<_>>()?.map(ns_proof::Proof::V1)
;
                        }
                        GeneratedField::V1IncorrectEncoding => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v1IncorrectEncoding"));
                            }
                            proof__ = map_.next_value::<::std::option::Option<_>>()?.map(ns_proof::Proof::V1IncorrectEncoding)
;
                        }
                        GeneratedField::V2 => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v2"));
                            }
                            proof__ = map_.next_value::<::std::option::Option<_>>()?.map(ns_proof::Proof::V2)
;
                        }
                    }
                }
                Ok(NsProof {
                    proof: proof__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.NsProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NsProofPayload {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.ns_index != 0 {
            len += 1;
        }
        if !self.ns_payload.is_empty() {
            len += 1;
        }
        if !self.ns_proof.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.NsProofPayload", len)?;
        if self.ns_index != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nsIndex", ToString::to_string(&self.ns_index).as_str())?;
        }
        if !self.ns_payload.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nsPayload", pbjson::private::base64::encode(&self.ns_payload).as_str())?;
        }
        if !self.ns_proof.is_empty() {
            struct_ser.serialize_field("nsProof", &self.ns_proof)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NsProofPayload {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ns_index",
            "nsIndex",
            "ns_payload",
            "nsPayload",
            "ns_proof",
            "nsProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            NsIndex,
            NsPayload,
            NsProof,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "nsIndex" | "ns_index" => Ok(GeneratedField::NsIndex),
                            "nsPayload" | "ns_payload" => Ok(GeneratedField::NsPayload),
                            "nsProof" | "ns_proof" => Ok(GeneratedField::NsProof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NsProofPayload;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.NsProofPayload")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NsProofPayload, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut ns_index__ = None;
                let mut ns_payload__ = None;
                let mut ns_proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::NsIndex => {
                            if ns_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsIndex"));
                            }
                            ns_index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NsPayload => {
                            if ns_payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsPayload"));
                            }
                            ns_payload__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NsProof => {
                            if ns_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsProof"));
                            }
                            ns_proof__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(NsProofPayload {
                    ns_index: ns_index__.unwrap_or_default(),
                    ns_payload: ns_payload__.unwrap_or_default(),
                    ns_proof: ns_proof__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.NsProofPayload", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NsTable {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.bytes.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.NsTable", len)?;
        if !self.bytes.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("bytes", pbjson::private::base64::encode(&self.bytes).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NsTable {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bytes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Bytes,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "bytes" => Ok(GeneratedField::Bytes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NsTable;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.NsTable")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NsTable, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bytes__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Bytes => {
                            if bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bytes"));
                            }
                            bytes__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(NsTable {
                    bytes: bytes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.NsTable", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Payload {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.raw_payload.is_empty() {
            len += 1;
        }
        if self.ns_table.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.Payload", len)?;
        if !self.raw_payload.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("rawPayload", pbjson::private::base64::encode(&self.raw_payload).as_str())?;
        }
        if let Some(v) = self.ns_table.as_ref() {
            struct_ser.serialize_field("nsTable", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Payload {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "raw_payload",
            "rawPayload",
            "ns_table",
            "nsTable",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RawPayload,
            NsTable,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "rawPayload" | "raw_payload" => Ok(GeneratedField::RawPayload),
                            "nsTable" | "ns_table" => Ok(GeneratedField::NsTable),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Payload;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.Payload")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Payload, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut raw_payload__ = None;
                let mut ns_table__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RawPayload => {
                            if raw_payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rawPayload"));
                            }
                            raw_payload__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NsTable => {
                            if ns_table__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nsTable"));
                            }
                            ns_table__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Payload {
                    raw_payload: raw_payload__.unwrap_or_default(),
                    ns_table: ns_table__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.Payload", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PayloadRangeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.payloads.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.PayloadRangeResponse", len)?;
        if !self.payloads.is_empty() {
            struct_ser.serialize_field("payloads", &self.payloads)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PayloadRangeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "payloads",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Payloads,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "payloads" => Ok(GeneratedField::Payloads),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PayloadRangeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.PayloadRangeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PayloadRangeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut payloads__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Payloads => {
                            if payloads__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloads"));
                            }
                            payloads__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PayloadRangeResponse {
                    payloads: payloads__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.PayloadRangeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PayloadResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height != 0 {
            len += 1;
        }
        if !self.block_hash.is_empty() {
            len += 1;
        }
        if !self.hash.is_empty() {
            len += 1;
        }
        if self.size != 0 {
            len += 1;
        }
        if self.data.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.PayloadResponse", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if !self.block_hash.is_empty() {
            struct_ser.serialize_field("blockHash", &self.block_hash)?;
        }
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", &self.hash)?;
        }
        if self.size != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("size", ToString::to_string(&self.size).as_str())?;
        }
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PayloadResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "block_hash",
            "blockHash",
            "hash",
            "size",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            BlockHash,
            Hash,
            Size,
            Data,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "blockHash" | "block_hash" => Ok(GeneratedField::BlockHash),
                            "hash" => Ok(GeneratedField::Hash),
                            "size" => Ok(GeneratedField::Size),
                            "data" => Ok(GeneratedField::Data),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PayloadResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.PayloadResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PayloadResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut block_hash__ = None;
                let mut hash__ = None;
                let mut size__ = None;
                let mut data__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BlockHash => {
                            if block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHash"));
                            }
                            block_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Size => {
                            if size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("size"));
                            }
                            size__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map_.next_value()?;
                        }
                    }
                }
                Ok(PayloadResponse {
                    height: height__.unwrap_or_default(),
                    block_hash: block_hash__.unwrap_or_default(),
                    hash: hash__.unwrap_or_default(),
                    size: size__.unwrap_or_default(),
                    data: data__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.PayloadResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PayloadSizeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.bytes != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.PayloadSizeResponse", len)?;
        if self.bytes != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("bytes", ToString::to_string(&self.bytes).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PayloadSizeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bytes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Bytes,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "bytes" => Ok(GeneratedField::Bytes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PayloadSizeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.PayloadSizeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PayloadSizeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bytes__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Bytes => {
                            if bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bytes"));
                            }
                            bytes__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(PayloadSizeResponse {
                    bytes: bytes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.PayloadSizeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for QuorumCertificate2 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.data.is_some() {
            len += 1;
        }
        if !self.vote_commitment.is_empty() {
            len += 1;
        }
        if self.view_number != 0 {
            len += 1;
        }
        if self.signatures.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.QuorumCertificate2", len)?;
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        if !self.vote_commitment.is_empty() {
            struct_ser.serialize_field("voteCommitment", &self.vote_commitment)?;
        }
        if self.view_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("viewNumber", ToString::to_string(&self.view_number).as_str())?;
        }
        if let Some(v) = self.signatures.as_ref() {
            struct_ser.serialize_field("signatures", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for QuorumCertificate2 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "data",
            "vote_commitment",
            "voteCommitment",
            "view_number",
            "viewNumber",
            "signatures",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Data,
            VoteCommitment,
            ViewNumber,
            Signatures,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "data" => Ok(GeneratedField::Data),
                            "voteCommitment" | "vote_commitment" => Ok(GeneratedField::VoteCommitment),
                            "viewNumber" | "view_number" => Ok(GeneratedField::ViewNumber),
                            "signatures" => Ok(GeneratedField::Signatures),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = QuorumCertificate2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.QuorumCertificate2")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<QuorumCertificate2, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut data__ = None;
                let mut vote_commitment__ = None;
                let mut view_number__ = None;
                let mut signatures__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map_.next_value()?;
                        }
                        GeneratedField::VoteCommitment => {
                            if vote_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("voteCommitment"));
                            }
                            vote_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ViewNumber => {
                            if view_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("viewNumber"));
                            }
                            view_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = map_.next_value()?;
                        }
                    }
                }
                Ok(QuorumCertificate2 {
                    data: data__,
                    vote_commitment: vote_commitment__.unwrap_or_default(),
                    view_number: view_number__.unwrap_or_default(),
                    signatures: signatures__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.QuorumCertificate2", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for QuorumData2 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.leaf_commit.is_empty() {
            len += 1;
        }
        if self.epoch.is_some() {
            len += 1;
        }
        if self.block_number.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.QuorumData2", len)?;
        if !self.leaf_commit.is_empty() {
            struct_ser.serialize_field("leafCommit", &self.leaf_commit)?;
        }
        if let Some(v) = self.epoch.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.block_number.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("blockNumber", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for QuorumData2 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "leaf_commit",
            "leafCommit",
            "epoch",
            "block_number",
            "blockNumber",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            LeafCommit,
            Epoch,
            BlockNumber,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "leafCommit" | "leaf_commit" => Ok(GeneratedField::LeafCommit),
                            "epoch" => Ok(GeneratedField::Epoch),
                            "blockNumber" | "block_number" => Ok(GeneratedField::BlockNumber),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = QuorumData2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.QuorumData2")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<QuorumData2, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut leaf_commit__ = None;
                let mut epoch__ = None;
                let mut block_number__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::LeafCommit => {
                            if leaf_commit__.is_some() {
                                return Err(serde::de::Error::duplicate_field("leafCommit"));
                            }
                            leaf_commit__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::BlockNumber => {
                            if block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockNumber"));
                            }
                            block_number__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(QuorumData2 {
                    leaf_commit: leaf_commit__.unwrap_or_default(),
                    epoch: epoch__,
                    block_number: block_number__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.QuorumData2", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for QuorumSignatures {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.signature.is_empty() {
            len += 1;
        }
        if !self.signers.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.QuorumSignatures", len)?;
        if !self.signature.is_empty() {
            struct_ser.serialize_field("signature", &self.signature)?;
        }
        if !self.signers.is_empty() {
            struct_ser.serialize_field("signers", &self.signers)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for QuorumSignatures {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signature",
            "signers",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Signature,
            Signers,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "signature" => Ok(GeneratedField::Signature),
                            "signers" => Ok(GeneratedField::Signers),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = QuorumSignatures;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.QuorumSignatures")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<QuorumSignatures, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signature__ = None;
                let mut signers__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Signers => {
                            if signers__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signers"));
                            }
                            signers__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(QuorumSignatures {
                    signature: signature__.unwrap_or_default(),
                    signers: signers__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.QuorumSignatures", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ResolvableChainConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.chain_config.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.ResolvableChainConfig", len)?;
        if let Some(v) = self.chain_config.as_ref() {
            match v {
                resolvable_chain_config::ChainConfig::Full(v) => {
                    struct_ser.serialize_field("full", v)?;
                }
                resolvable_chain_config::ChainConfig::Commitment(v) => {
                    struct_ser.serialize_field("commitment", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ResolvableChainConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "full",
            "commitment",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Full,
            Commitment,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "full" => Ok(GeneratedField::Full),
                            "commitment" => Ok(GeneratedField::Commitment),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ResolvableChainConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.ResolvableChainConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ResolvableChainConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_config__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Full => {
                            if chain_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("full"));
                            }
                            chain_config__ = map_.next_value::<::std::option::Option<_>>()?.map(resolvable_chain_config::ChainConfig::Full)
;
                        }
                        GeneratedField::Commitment => {
                            if chain_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commitment"));
                            }
                            chain_config__ = map_.next_value::<::std::option::Option<_>>()?.map(resolvable_chain_config::ChainConfig::Commitment);
                        }
                    }
                }
                Ok(ResolvableChainConfig {
                    chain_config: chain_config__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.ResolvableChainConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ResourceSyncStatus {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.missing != 0 {
            len += 1;
        }
        if !self.ranges.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.ResourceSyncStatus", len)?;
        if self.missing != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("missing", ToString::to_string(&self.missing).as_str())?;
        }
        if !self.ranges.is_empty() {
            struct_ser.serialize_field("ranges", &self.ranges)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ResourceSyncStatus {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "missing",
            "ranges",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Missing,
            Ranges,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "missing" => Ok(GeneratedField::Missing),
                            "ranges" => Ok(GeneratedField::Ranges),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ResourceSyncStatus;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.ResourceSyncStatus")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ResourceSyncStatus, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut missing__ = None;
                let mut ranges__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Missing => {
                            if missing__.is_some() {
                                return Err(serde::de::Error::duplicate_field("missing"));
                            }
                            missing__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Ranges => {
                            if ranges__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ranges"));
                            }
                            ranges__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ResourceSyncStatus {
                    missing: missing__.unwrap_or_default(),
                    ranges: ranges__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.ResourceSyncStatus", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RuntimeConfigResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.is_da {
            len += 1;
        }
        if self.identity.is_some() {
            len += 1;
        }
        if self.storage_backend != 0 {
            len += 1;
        }
        if !self.genesis_file.is_empty() {
            len += 1;
        }
        if self.public_api_url.is_some() {
            len += 1;
        }
        if !self.builder_urls.is_empty() {
            len += 1;
        }
        if !self.state_relay_server_url.is_empty() {
            len += 1;
        }
        if !self.state_peers.is_empty() {
            len += 1;
        }
        if !self.config_peers.is_empty() {
            len += 1;
        }
        if !self.orchestrator_url.is_empty() {
            len += 1;
        }
        if !self.cdn_endpoint.is_empty() {
            len += 1;
        }
        if !self.cliquenet_bind_address.is_empty() {
            len += 1;
        }
        if self.cliquenet_advertise_address.is_some() {
            len += 1;
        }
        if !self.libp2p_bind_address.is_empty() {
            len += 1;
        }
        if self.libp2p_advertise_address.is_some() {
            len += 1;
        }
        if !self.libp2p_bootstrap_nodes.is_empty() {
            len += 1;
        }
        if self.l1_provider_count != 0 {
            len += 1;
        }
        if self.l1_ws_provider_count != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.RuntimeConfigResponse", len)?;
        if self.is_da {
            struct_ser.serialize_field("isDa", &self.is_da)?;
        }
        if let Some(v) = self.identity.as_ref() {
            struct_ser.serialize_field("identity", v)?;
        }
        if self.storage_backend != 0 {
            let v = StorageBackend::try_from(self.storage_backend)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.storage_backend)))?;
            struct_ser.serialize_field("storageBackend", &v)?;
        }
        if !self.genesis_file.is_empty() {
            struct_ser.serialize_field("genesisFile", &self.genesis_file)?;
        }
        if let Some(v) = self.public_api_url.as_ref() {
            struct_ser.serialize_field("publicApiUrl", v)?;
        }
        if !self.builder_urls.is_empty() {
            struct_ser.serialize_field("builderUrls", &self.builder_urls)?;
        }
        if !self.state_relay_server_url.is_empty() {
            struct_ser.serialize_field("stateRelayServerUrl", &self.state_relay_server_url)?;
        }
        if !self.state_peers.is_empty() {
            struct_ser.serialize_field("statePeers", &self.state_peers)?;
        }
        if !self.config_peers.is_empty() {
            struct_ser.serialize_field("configPeers", &self.config_peers)?;
        }
        if !self.orchestrator_url.is_empty() {
            struct_ser.serialize_field("orchestratorUrl", &self.orchestrator_url)?;
        }
        if !self.cdn_endpoint.is_empty() {
            struct_ser.serialize_field("cdnEndpoint", &self.cdn_endpoint)?;
        }
        if !self.cliquenet_bind_address.is_empty() {
            struct_ser.serialize_field("cliquenetBindAddress", &self.cliquenet_bind_address)?;
        }
        if let Some(v) = self.cliquenet_advertise_address.as_ref() {
            struct_ser.serialize_field("cliquenetAdvertiseAddress", v)?;
        }
        if !self.libp2p_bind_address.is_empty() {
            struct_ser.serialize_field("libp2pBindAddress", &self.libp2p_bind_address)?;
        }
        if let Some(v) = self.libp2p_advertise_address.as_ref() {
            struct_ser.serialize_field("libp2pAdvertiseAddress", v)?;
        }
        if !self.libp2p_bootstrap_nodes.is_empty() {
            struct_ser.serialize_field("libp2pBootstrapNodes", &self.libp2p_bootstrap_nodes)?;
        }
        if self.l1_provider_count != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("l1ProviderCount", ToString::to_string(&self.l1_provider_count).as_str())?;
        }
        if self.l1_ws_provider_count != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("l1WsProviderCount", ToString::to_string(&self.l1_ws_provider_count).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RuntimeConfigResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "is_da",
            "isDa",
            "identity",
            "storage_backend",
            "storageBackend",
            "genesis_file",
            "genesisFile",
            "public_api_url",
            "publicApiUrl",
            "builder_urls",
            "builderUrls",
            "state_relay_server_url",
            "stateRelayServerUrl",
            "state_peers",
            "statePeers",
            "config_peers",
            "configPeers",
            "orchestrator_url",
            "orchestratorUrl",
            "cdn_endpoint",
            "cdnEndpoint",
            "cliquenet_bind_address",
            "cliquenetBindAddress",
            "cliquenet_advertise_address",
            "cliquenetAdvertiseAddress",
            "libp2p_bind_address",
            "libp2pBindAddress",
            "libp2p_advertise_address",
            "libp2pAdvertiseAddress",
            "libp2p_bootstrap_nodes",
            "libp2pBootstrapNodes",
            "l1_provider_count",
            "l1ProviderCount",
            "l1_ws_provider_count",
            "l1WsProviderCount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            IsDa,
            Identity,
            StorageBackend,
            GenesisFile,
            PublicApiUrl,
            BuilderUrls,
            StateRelayServerUrl,
            StatePeers,
            ConfigPeers,
            OrchestratorUrl,
            CdnEndpoint,
            CliquenetBindAddress,
            CliquenetAdvertiseAddress,
            Libp2pBindAddress,
            Libp2pAdvertiseAddress,
            Libp2pBootstrapNodes,
            L1ProviderCount,
            L1WsProviderCount,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "isDa" | "is_da" => Ok(GeneratedField::IsDa),
                            "identity" => Ok(GeneratedField::Identity),
                            "storageBackend" | "storage_backend" => Ok(GeneratedField::StorageBackend),
                            "genesisFile" | "genesis_file" => Ok(GeneratedField::GenesisFile),
                            "publicApiUrl" | "public_api_url" => Ok(GeneratedField::PublicApiUrl),
                            "builderUrls" | "builder_urls" => Ok(GeneratedField::BuilderUrls),
                            "stateRelayServerUrl" | "state_relay_server_url" => Ok(GeneratedField::StateRelayServerUrl),
                            "statePeers" | "state_peers" => Ok(GeneratedField::StatePeers),
                            "configPeers" | "config_peers" => Ok(GeneratedField::ConfigPeers),
                            "orchestratorUrl" | "orchestrator_url" => Ok(GeneratedField::OrchestratorUrl),
                            "cdnEndpoint" | "cdn_endpoint" => Ok(GeneratedField::CdnEndpoint),
                            "cliquenetBindAddress" | "cliquenet_bind_address" => Ok(GeneratedField::CliquenetBindAddress),
                            "cliquenetAdvertiseAddress" | "cliquenet_advertise_address" => Ok(GeneratedField::CliquenetAdvertiseAddress),
                            "libp2pBindAddress" | "libp2p_bind_address" => Ok(GeneratedField::Libp2pBindAddress),
                            "libp2pAdvertiseAddress" | "libp2p_advertise_address" => Ok(GeneratedField::Libp2pAdvertiseAddress),
                            "libp2pBootstrapNodes" | "libp2p_bootstrap_nodes" => Ok(GeneratedField::Libp2pBootstrapNodes),
                            "l1ProviderCount" | "l1_provider_count" => Ok(GeneratedField::L1ProviderCount),
                            "l1WsProviderCount" | "l1_ws_provider_count" => Ok(GeneratedField::L1WsProviderCount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RuntimeConfigResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.RuntimeConfigResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RuntimeConfigResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut is_da__ = None;
                let mut identity__ = None;
                let mut storage_backend__ = None;
                let mut genesis_file__ = None;
                let mut public_api_url__ = None;
                let mut builder_urls__ = None;
                let mut state_relay_server_url__ = None;
                let mut state_peers__ = None;
                let mut config_peers__ = None;
                let mut orchestrator_url__ = None;
                let mut cdn_endpoint__ = None;
                let mut cliquenet_bind_address__ = None;
                let mut cliquenet_advertise_address__ = None;
                let mut libp2p_bind_address__ = None;
                let mut libp2p_advertise_address__ = None;
                let mut libp2p_bootstrap_nodes__ = None;
                let mut l1_provider_count__ = None;
                let mut l1_ws_provider_count__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::IsDa => {
                            if is_da__.is_some() {
                                return Err(serde::de::Error::duplicate_field("isDa"));
                            }
                            is_da__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Identity => {
                            if identity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("identity"));
                            }
                            identity__ = map_.next_value()?;
                        }
                        GeneratedField::StorageBackend => {
                            if storage_backend__.is_some() {
                                return Err(serde::de::Error::duplicate_field("storageBackend"));
                            }
                            storage_backend__ = Some(map_.next_value::<StorageBackend>()? as i32);
                        }
                        GeneratedField::GenesisFile => {
                            if genesis_file__.is_some() {
                                return Err(serde::de::Error::duplicate_field("genesisFile"));
                            }
                            genesis_file__ = Some(map_.next_value()?);
                        }
                        GeneratedField::PublicApiUrl => {
                            if public_api_url__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publicApiUrl"));
                            }
                            public_api_url__ = map_.next_value()?;
                        }
                        GeneratedField::BuilderUrls => {
                            if builder_urls__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderUrls"));
                            }
                            builder_urls__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StateRelayServerUrl => {
                            if state_relay_server_url__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateRelayServerUrl"));
                            }
                            state_relay_server_url__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StatePeers => {
                            if state_peers__.is_some() {
                                return Err(serde::de::Error::duplicate_field("statePeers"));
                            }
                            state_peers__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ConfigPeers => {
                            if config_peers__.is_some() {
                                return Err(serde::de::Error::duplicate_field("configPeers"));
                            }
                            config_peers__ = Some(map_.next_value()?);
                        }
                        GeneratedField::OrchestratorUrl => {
                            if orchestrator_url__.is_some() {
                                return Err(serde::de::Error::duplicate_field("orchestratorUrl"));
                            }
                            orchestrator_url__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CdnEndpoint => {
                            if cdn_endpoint__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cdnEndpoint"));
                            }
                            cdn_endpoint__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CliquenetBindAddress => {
                            if cliquenet_bind_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cliquenetBindAddress"));
                            }
                            cliquenet_bind_address__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CliquenetAdvertiseAddress => {
                            if cliquenet_advertise_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cliquenetAdvertiseAddress"));
                            }
                            cliquenet_advertise_address__ = map_.next_value()?;
                        }
                        GeneratedField::Libp2pBindAddress => {
                            if libp2p_bind_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("libp2pBindAddress"));
                            }
                            libp2p_bind_address__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Libp2pAdvertiseAddress => {
                            if libp2p_advertise_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("libp2pAdvertiseAddress"));
                            }
                            libp2p_advertise_address__ = map_.next_value()?;
                        }
                        GeneratedField::Libp2pBootstrapNodes => {
                            if libp2p_bootstrap_nodes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("libp2pBootstrapNodes"));
                            }
                            libp2p_bootstrap_nodes__ = Some(map_.next_value()?);
                        }
                        GeneratedField::L1ProviderCount => {
                            if l1_provider_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("l1ProviderCount"));
                            }
                            l1_provider_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::L1WsProviderCount => {
                            if l1_ws_provider_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("l1WsProviderCount"));
                            }
                            l1_ws_provider_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(RuntimeConfigResponse {
                    is_da: is_da__.unwrap_or_default(),
                    identity: identity__,
                    storage_backend: storage_backend__.unwrap_or_default(),
                    genesis_file: genesis_file__.unwrap_or_default(),
                    public_api_url: public_api_url__,
                    builder_urls: builder_urls__.unwrap_or_default(),
                    state_relay_server_url: state_relay_server_url__.unwrap_or_default(),
                    state_peers: state_peers__.unwrap_or_default(),
                    config_peers: config_peers__.unwrap_or_default(),
                    orchestrator_url: orchestrator_url__.unwrap_or_default(),
                    cdn_endpoint: cdn_endpoint__.unwrap_or_default(),
                    cliquenet_bind_address: cliquenet_bind_address__.unwrap_or_default(),
                    cliquenet_advertise_address: cliquenet_advertise_address__,
                    libp2p_bind_address: libp2p_bind_address__.unwrap_or_default(),
                    libp2p_advertise_address: libp2p_advertise_address__,
                    libp2p_bootstrap_nodes: libp2p_bootstrap_nodes__.unwrap_or_default(),
                    l1_provider_count: l1_provider_count__.unwrap_or_default(),
                    l1_ws_provider_count: l1_ws_provider_count__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.RuntimeConfigResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SchnorrPublicKey {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.key.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.SchnorrPublicKey", len)?;
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SchnorrPublicKey {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SchnorrPublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.SchnorrPublicKey")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SchnorrPublicKey, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SchnorrPublicKey {
                    key: key__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.SchnorrPublicKey", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SmallRangeProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.proofs.is_empty() {
            len += 1;
        }
        if !self.prefix_bytes.is_empty() {
            len += 1;
        }
        if !self.suffix_bytes.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.SmallRangeProof", len)?;
        if !self.proofs.is_empty() {
            struct_ser.serialize_field("proofs", &self.proofs)?;
        }
        if !self.prefix_bytes.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("prefixBytes", pbjson::private::base64::encode(&self.prefix_bytes).as_str())?;
        }
        if !self.suffix_bytes.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("suffixBytes", pbjson::private::base64::encode(&self.suffix_bytes).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SmallRangeProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "proofs",
            "prefix_bytes",
            "prefixBytes",
            "suffix_bytes",
            "suffixBytes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Proofs,
            PrefixBytes,
            SuffixBytes,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "proofs" => Ok(GeneratedField::Proofs),
                            "prefixBytes" | "prefix_bytes" => Ok(GeneratedField::PrefixBytes),
                            "suffixBytes" | "suffix_bytes" => Ok(GeneratedField::SuffixBytes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SmallRangeProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.SmallRangeProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SmallRangeProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut proofs__ = None;
                let mut prefix_bytes__ = None;
                let mut suffix_bytes__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Proofs => {
                            if proofs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proofs"));
                            }
                            proofs__ = Some(map_.next_value()?);
                        }
                        GeneratedField::PrefixBytes => {
                            if prefix_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prefixBytes"));
                            }
                            prefix_bytes__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::SuffixBytes => {
                            if suffix_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("suffixBytes"));
                            }
                            suffix_bytes__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(SmallRangeProof {
                    proofs: proofs__.unwrap_or_default(),
                    prefix_bytes: prefix_bytes__.unwrap_or_default(),
                    suffix_bytes: suffix_bytes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.SmallRangeProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StateCertV1Response {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.epoch != 0 {
            len += 1;
        }
        if !self.light_client_state.is_empty() {
            len += 1;
        }
        if !self.next_stake_table_state.is_empty() {
            len += 1;
        }
        if !self.signatures.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.StateCertV1Response", len)?;
        if self.epoch != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&self.epoch).as_str())?;
        }
        if !self.light_client_state.is_empty() {
            struct_ser.serialize_field("lightClientState", &self.light_client_state)?;
        }
        if !self.next_stake_table_state.is_empty() {
            struct_ser.serialize_field("nextStakeTableState", &self.next_stake_table_state)?;
        }
        if !self.signatures.is_empty() {
            struct_ser.serialize_field("signatures", &self.signatures)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StateCertV1Response {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "epoch",
            "light_client_state",
            "lightClientState",
            "next_stake_table_state",
            "nextStakeTableState",
            "signatures",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Epoch,
            LightClientState,
            NextStakeTableState,
            Signatures,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "epoch" => Ok(GeneratedField::Epoch),
                            "lightClientState" | "light_client_state" => Ok(GeneratedField::LightClientState),
                            "nextStakeTableState" | "next_stake_table_state" => Ok(GeneratedField::NextStakeTableState),
                            "signatures" => Ok(GeneratedField::Signatures),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StateCertV1Response;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.StateCertV1Response")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StateCertV1Response, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut epoch__ = None;
                let mut light_client_state__ = None;
                let mut next_stake_table_state__ = None;
                let mut signatures__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::LightClientState => {
                            if light_client_state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lightClientState"));
                            }
                            light_client_state__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NextStakeTableState => {
                            if next_stake_table_state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextStakeTableState"));
                            }
                            next_stake_table_state__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StateCertV1Response {
                    epoch: epoch__.unwrap_or_default(),
                    light_client_state: light_client_state__.unwrap_or_default(),
                    next_stake_table_state: next_stake_table_state__.unwrap_or_default(),
                    signatures: signatures__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.StateCertV1Response", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StateCertV2Response {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.epoch != 0 {
            len += 1;
        }
        if !self.light_client_state.is_empty() {
            len += 1;
        }
        if !self.next_stake_table_state.is_empty() {
            len += 1;
        }
        if !self.signatures.is_empty() {
            len += 1;
        }
        if !self.auth_root.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.StateCertV2Response", len)?;
        if self.epoch != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&self.epoch).as_str())?;
        }
        if !self.light_client_state.is_empty() {
            struct_ser.serialize_field("lightClientState", &self.light_client_state)?;
        }
        if !self.next_stake_table_state.is_empty() {
            struct_ser.serialize_field("nextStakeTableState", &self.next_stake_table_state)?;
        }
        if !self.signatures.is_empty() {
            struct_ser.serialize_field("signatures", &self.signatures)?;
        }
        if !self.auth_root.is_empty() {
            struct_ser.serialize_field("authRoot", &self.auth_root)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StateCertV2Response {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "epoch",
            "light_client_state",
            "lightClientState",
            "next_stake_table_state",
            "nextStakeTableState",
            "signatures",
            "auth_root",
            "authRoot",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Epoch,
            LightClientState,
            NextStakeTableState,
            Signatures,
            AuthRoot,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "epoch" => Ok(GeneratedField::Epoch),
                            "lightClientState" | "light_client_state" => Ok(GeneratedField::LightClientState),
                            "nextStakeTableState" | "next_stake_table_state" => Ok(GeneratedField::NextStakeTableState),
                            "signatures" => Ok(GeneratedField::Signatures),
                            "authRoot" | "auth_root" => Ok(GeneratedField::AuthRoot),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StateCertV2Response;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.StateCertV2Response")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StateCertV2Response, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut epoch__ = None;
                let mut light_client_state__ = None;
                let mut next_stake_table_state__ = None;
                let mut signatures__ = None;
                let mut auth_root__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::LightClientState => {
                            if light_client_state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lightClientState"));
                            }
                            light_client_state__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NextStakeTableState => {
                            if next_stake_table_state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextStakeTableState"));
                            }
                            next_stake_table_state__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AuthRoot => {
                            if auth_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authRoot"));
                            }
                            auth_root__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StateCertV2Response {
                    epoch: epoch__.unwrap_or_default(),
                    light_client_state: light_client_state__.unwrap_or_default(),
                    next_stake_table_state: next_stake_table_state__.unwrap_or_default(),
                    signatures: signatures__.unwrap_or_default(),
                    auth_root: auth_root__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.StateCertV2Response", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StateSignatureV1 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.key.is_empty() {
            len += 1;
        }
        if !self.signature.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.StateSignatureV1", len)?;
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        if !self.signature.is_empty() {
            struct_ser.serialize_field("signature", &self.signature)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StateSignatureV1 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
            "signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
            Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            "signature" => Ok(GeneratedField::Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StateSignatureV1;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.StateSignatureV1")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StateSignatureV1, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                let mut signature__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StateSignatureV1 {
                    key: key__.unwrap_or_default(),
                    signature: signature__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.StateSignatureV1", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StateSignatureV2 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.key.is_empty() {
            len += 1;
        }
        if !self.lcv3_signature.is_empty() {
            len += 1;
        }
        if !self.lcv2_signature.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.StateSignatureV2", len)?;
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        if !self.lcv3_signature.is_empty() {
            struct_ser.serialize_field("lcv3Signature", &self.lcv3_signature)?;
        }
        if !self.lcv2_signature.is_empty() {
            struct_ser.serialize_field("lcv2Signature", &self.lcv2_signature)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StateSignatureV2 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
            "lcv3_signature",
            "lcv3Signature",
            "lcv2_signature",
            "lcv2Signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
            Lcv3Signature,
            Lcv2Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            "lcv3Signature" | "lcv3_signature" => Ok(GeneratedField::Lcv3Signature),
                            "lcv2Signature" | "lcv2_signature" => Ok(GeneratedField::Lcv2Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StateSignatureV2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.StateSignatureV2")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StateSignatureV2, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                let mut lcv3_signature__ = None;
                let mut lcv2_signature__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Lcv3Signature => {
                            if lcv3_signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lcv3Signature"));
                            }
                            lcv3_signature__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Lcv2Signature => {
                            if lcv2_signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lcv2Signature"));
                            }
                            lcv2_signature__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StateSignatureV2 {
                    key: key__.unwrap_or_default(),
                    lcv3_signature: lcv3_signature__.unwrap_or_default(),
                    lcv2_signature: lcv2_signature__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.StateSignatureV2", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StorageBackend {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "STORAGE_BACKEND_UNSPECIFIED",
            Self::Sql => "STORAGE_BACKEND_SQL",
            Self::Fs => "STORAGE_BACKEND_FS",
            Self::FsDefault => "STORAGE_BACKEND_FS_DEFAULT",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for StorageBackend {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "STORAGE_BACKEND_UNSPECIFIED",
            "STORAGE_BACKEND_SQL",
            "STORAGE_BACKEND_FS",
            "STORAGE_BACKEND_FS_DEFAULT",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StorageBackend;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "STORAGE_BACKEND_UNSPECIFIED" => Ok(StorageBackend::Unspecified),
                    "STORAGE_BACKEND_SQL" => Ok(StorageBackend::Sql),
                    "STORAGE_BACKEND_FS" => Ok(StorageBackend::Fs),
                    "STORAGE_BACKEND_FS_DEFAULT" => Ok(StorageBackend::FsDefault),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for StreamFromRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.StreamFromRequest", len)?;
        if self.from != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&self.from).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamFromRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StreamFromRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.StreamFromRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StreamFromRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(StreamFromRequest {
                    from: from__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.StreamFromRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StreamNamespaceProofsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from != 0 {
            len += 1;
        }
        if self.namespace.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.StreamNamespaceProofsRequest", len)?;
        if self.from != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&self.from).as_str())?;
        }
        if let Some(v) = self.namespace.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("namespace", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamNamespaceProofsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "namespace",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            Namespace,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "namespace" => Ok(GeneratedField::Namespace),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StreamNamespaceProofsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.StreamNamespaceProofsRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StreamNamespaceProofsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut namespace__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Namespace => {
                            if namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespace"));
                            }
                            namespace__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(StreamNamespaceProofsRequest {
                    from: from__.unwrap_or_default(),
                    namespace: namespace__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.StreamNamespaceProofsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StreamTransactionsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.from != 0 {
            len += 1;
        }
        if self.namespace.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.StreamTransactionsRequest", len)?;
        if self.from != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("from", ToString::to_string(&self.from).as_str())?;
        }
        if let Some(v) = self.namespace.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("namespace", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamTransactionsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "from",
            "namespace",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            From,
            Namespace,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "from" => Ok(GeneratedField::From),
                            "namespace" => Ok(GeneratedField::Namespace),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StreamTransactionsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.StreamTransactionsRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StreamTransactionsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut from__ = None;
                let mut namespace__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Namespace => {
                            if namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespace"));
                            }
                            namespace__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(StreamTransactionsRequest {
                    from: from__.unwrap_or_default(),
                    namespace: namespace__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.StreamTransactionsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SuccessRateResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.rate != 0. {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.SuccessRateResponse", len)?;
        if self.rate != 0. {
            struct_ser.serialize_field("rate", &self.rate)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SuccessRateResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rate",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Rate,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "rate" => Ok(GeneratedField::Rate),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SuccessRateResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.SuccessRateResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SuccessRateResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rate__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Rate => {
                            if rate__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rate"));
                            }
                            rate__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(SuccessRateResponse {
                    rate: rate__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.SuccessRateResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SyncStatus {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "SYNC_STATUS_UNSPECIFIED",
            Self::Present => "SYNC_STATUS_PRESENT",
            Self::Missing => "SYNC_STATUS_MISSING",
            Self::Pruned => "SYNC_STATUS_PRUNED",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for SyncStatus {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "SYNC_STATUS_UNSPECIFIED",
            "SYNC_STATUS_PRESENT",
            "SYNC_STATUS_MISSING",
            "SYNC_STATUS_PRUNED",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SyncStatus;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "SYNC_STATUS_UNSPECIFIED" => Ok(SyncStatus::Unspecified),
                    "SYNC_STATUS_PRESENT" => Ok(SyncStatus::Present),
                    "SYNC_STATUS_MISSING" => Ok(SyncStatus::Missing),
                    "SYNC_STATUS_PRUNED" => Ok(SyncStatus::Pruned),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for SyncStatusRange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.start != 0 {
            len += 1;
        }
        if self.end != 0 {
            len += 1;
        }
        if self.status != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.SyncStatusRange", len)?;
        if self.start != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("start", ToString::to_string(&self.start).as_str())?;
        }
        if self.end != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("end", ToString::to_string(&self.end).as_str())?;
        }
        if self.status != 0 {
            let v = SyncStatus::try_from(self.status)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.status)))?;
            struct_ser.serialize_field("status", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SyncStatusRange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start",
            "end",
            "status",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Start,
            End,
            Status,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "start" => Ok(GeneratedField::Start),
                            "end" => Ok(GeneratedField::End),
                            "status" => Ok(GeneratedField::Status),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SyncStatusRange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.SyncStatusRange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SyncStatusRange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start__ = None;
                let mut end__ = None;
                let mut status__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Start => {
                            if start__.is_some() {
                                return Err(serde::de::Error::duplicate_field("start"));
                            }
                            start__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::End => {
                            if end__.is_some() {
                                return Err(serde::de::Error::duplicate_field("end"));
                            }
                            end__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Status => {
                            if status__.is_some() {
                                return Err(serde::de::Error::duplicate_field("status"));
                            }
                            status__ = Some(map_.next_value::<SyncStatus>()? as i32);
                        }
                    }
                }
                Ok(SyncStatusRange {
                    start: start__.unwrap_or_default(),
                    end: end__.unwrap_or_default(),
                    status: status__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.SyncStatusRange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SyncStatusResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.blocks.is_some() {
            len += 1;
        }
        if self.leaves.is_some() {
            len += 1;
        }
        if self.vid_common.is_some() {
            len += 1;
        }
        if self.pruned_height.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.SyncStatusResponse", len)?;
        if let Some(v) = self.blocks.as_ref() {
            struct_ser.serialize_field("blocks", v)?;
        }
        if let Some(v) = self.leaves.as_ref() {
            struct_ser.serialize_field("leaves", v)?;
        }
        if let Some(v) = self.vid_common.as_ref() {
            struct_ser.serialize_field("vidCommon", v)?;
        }
        if let Some(v) = self.pruned_height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("prunedHeight", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SyncStatusResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "blocks",
            "leaves",
            "vid_common",
            "vidCommon",
            "pruned_height",
            "prunedHeight",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Blocks,
            Leaves,
            VidCommon,
            PrunedHeight,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "blocks" => Ok(GeneratedField::Blocks),
                            "leaves" => Ok(GeneratedField::Leaves),
                            "vidCommon" | "vid_common" => Ok(GeneratedField::VidCommon),
                            "prunedHeight" | "pruned_height" => Ok(GeneratedField::PrunedHeight),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SyncStatusResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.SyncStatusResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SyncStatusResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut blocks__ = None;
                let mut leaves__ = None;
                let mut vid_common__ = None;
                let mut pruned_height__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Blocks => {
                            if blocks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blocks"));
                            }
                            blocks__ = map_.next_value()?;
                        }
                        GeneratedField::Leaves => {
                            if leaves__.is_some() {
                                return Err(serde::de::Error::duplicate_field("leaves"));
                            }
                            leaves__ = map_.next_value()?;
                        }
                        GeneratedField::VidCommon => {
                            if vid_common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("vidCommon"));
                            }
                            vid_common__ = map_.next_value()?;
                        }
                        GeneratedField::PrunedHeight => {
                            if pruned_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prunedHeight"));
                            }
                            pruned_height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(SyncStatusResponse {
                    blocks: blocks__,
                    leaves: leaves__,
                    vid_common: vid_common__,
                    pruned_height: pruned_height__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.SyncStatusResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TableSize {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.table_name.is_empty() {
            len += 1;
        }
        if self.row_count != 0 {
            len += 1;
        }
        if self.total_size_bytes.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TableSize", len)?;
        if !self.table_name.is_empty() {
            struct_ser.serialize_field("tableName", &self.table_name)?;
        }
        if self.row_count != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("rowCount", ToString::to_string(&self.row_count).as_str())?;
        }
        if let Some(v) = self.total_size_bytes.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("totalSizeBytes", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TableSize {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "table_name",
            "tableName",
            "row_count",
            "rowCount",
            "total_size_bytes",
            "totalSizeBytes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TableName,
            RowCount,
            TotalSizeBytes,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "tableName" | "table_name" => Ok(GeneratedField::TableName),
                            "rowCount" | "row_count" => Ok(GeneratedField::RowCount),
                            "totalSizeBytes" | "total_size_bytes" => Ok(GeneratedField::TotalSizeBytes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TableSize;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TableSize")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TableSize, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut table_name__ = None;
                let mut row_count__ = None;
                let mut total_size_bytes__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TableName => {
                            if table_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tableName"));
                            }
                            table_name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RowCount => {
                            if row_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rowCount"));
                            }
                            row_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::TotalSizeBytes => {
                            if total_size_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalSizeBytes"));
                            }
                            total_size_bytes__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(TableSize {
                    table_name: table_name__.unwrap_or_default(),
                    row_count: row_count__.unwrap_or_default(),
                    total_size_bytes: total_size_bytes__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TableSize", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TableSizesResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.tables.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TableSizesResponse", len)?;
        if !self.tables.is_empty() {
            struct_ser.serialize_field("tables", &self.tables)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TableSizesResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tables",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Tables,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "tables" => Ok(GeneratedField::Tables),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TableSizesResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TableSizesResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TableSizesResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tables__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Tables => {
                            if tables__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tables"));
                            }
                            tables__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TableSizesResponse {
                    tables: tables__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TableSizesResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TimeSinceLastDecideResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.seconds != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TimeSinceLastDecideResponse", len)?;
        if self.seconds != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("seconds", ToString::to_string(&self.seconds).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TimeSinceLastDecideResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "seconds",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Seconds,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "seconds" => Ok(GeneratedField::Seconds),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TimeSinceLastDecideResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TimeSinceLastDecideResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TimeSinceLastDecideResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut seconds__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Seconds => {
                            if seconds__.is_some() {
                                return Err(serde::de::Error::duplicate_field("seconds"));
                            }
                            seconds__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(TimeSinceLastDecideResponse {
                    seconds: seconds__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TimeSinceLastDecideResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TimeoutCertificate2 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.data.is_some() {
            len += 1;
        }
        if !self.vote_commitment.is_empty() {
            len += 1;
        }
        if self.view_number != 0 {
            len += 1;
        }
        if self.signatures.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TimeoutCertificate2", len)?;
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        if !self.vote_commitment.is_empty() {
            struct_ser.serialize_field("voteCommitment", &self.vote_commitment)?;
        }
        if self.view_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("viewNumber", ToString::to_string(&self.view_number).as_str())?;
        }
        if let Some(v) = self.signatures.as_ref() {
            struct_ser.serialize_field("signatures", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TimeoutCertificate2 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "data",
            "vote_commitment",
            "voteCommitment",
            "view_number",
            "viewNumber",
            "signatures",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Data,
            VoteCommitment,
            ViewNumber,
            Signatures,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "data" => Ok(GeneratedField::Data),
                            "voteCommitment" | "vote_commitment" => Ok(GeneratedField::VoteCommitment),
                            "viewNumber" | "view_number" => Ok(GeneratedField::ViewNumber),
                            "signatures" => Ok(GeneratedField::Signatures),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TimeoutCertificate2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TimeoutCertificate2")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TimeoutCertificate2, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut data__ = None;
                let mut vote_commitment__ = None;
                let mut view_number__ = None;
                let mut signatures__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map_.next_value()?;
                        }
                        GeneratedField::VoteCommitment => {
                            if vote_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("voteCommitment"));
                            }
                            vote_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ViewNumber => {
                            if view_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("viewNumber"));
                            }
                            view_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = map_.next_value()?;
                        }
                    }
                }
                Ok(TimeoutCertificate2 {
                    data: data__,
                    vote_commitment: vote_commitment__.unwrap_or_default(),
                    view_number: view_number__.unwrap_or_default(),
                    signatures: signatures__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TimeoutCertificate2", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TimeoutData2 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.view != 0 {
            len += 1;
        }
        if self.epoch.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TimeoutData2", len)?;
        if self.view != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("view", ToString::to_string(&self.view).as_str())?;
        }
        if let Some(v) = self.epoch.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TimeoutData2 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "view",
            "epoch",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            View,
            Epoch,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "view" => Ok(GeneratedField::View),
                            "epoch" => Ok(GeneratedField::Epoch),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TimeoutData2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TimeoutData2")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TimeoutData2, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut view__ = None;
                let mut epoch__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::View => {
                            if view__.is_some() {
                                return Err(serde::de::Error::duplicate_field("view"));
                            }
                            view__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(TimeoutData2 {
                    view: view__.unwrap_or_default(),
                    epoch: epoch__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TimeoutData2", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TotalIssuedSupplyResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.amount.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TotalIssuedSupplyResponse", len)?;
        if !self.amount.is_empty() {
            struct_ser.serialize_field("amount", &self.amount)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TotalIssuedSupplyResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "amount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Amount,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "amount" => Ok(GeneratedField::Amount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TotalIssuedSupplyResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TotalIssuedSupplyResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TotalIssuedSupplyResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut amount__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TotalIssuedSupplyResponse {
                    amount: amount__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TotalIssuedSupplyResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TotalMintedSupplyResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.amount.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TotalMintedSupplyResponse", len)?;
        if !self.amount.is_empty() {
            struct_ser.serialize_field("amount", &self.amount)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TotalMintedSupplyResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "amount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Amount,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "amount" => Ok(GeneratedField::Amount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TotalMintedSupplyResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TotalMintedSupplyResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TotalMintedSupplyResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut amount__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TotalMintedSupplyResponse {
                    amount: amount__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TotalMintedSupplyResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TotalRewardDistributedResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.amount.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TotalRewardDistributedResponse", len)?;
        if !self.amount.is_empty() {
            struct_ser.serialize_field("amount", &self.amount)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TotalRewardDistributedResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "amount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Amount,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "amount" => Ok(GeneratedField::Amount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TotalRewardDistributedResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TotalRewardDistributedResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TotalRewardDistributedResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut amount__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TotalRewardDistributedResponse {
                    amount: amount__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TotalRewardDistributedResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Transaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.namespace != 0 {
            len += 1;
        }
        if !self.payload.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.Transaction", len)?;
        if self.namespace != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("namespace", ToString::to_string(&self.namespace).as_str())?;
        }
        if !self.payload.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("payload", pbjson::private::base64::encode(&self.payload).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Transaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "namespace",
            "payload",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Namespace,
            Payload,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "namespace" => Ok(GeneratedField::Namespace),
                            "payload" => Ok(GeneratedField::Payload),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Transaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.Transaction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Transaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut namespace__ = None;
                let mut payload__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Namespace => {
                            if namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespace"));
                            }
                            namespace__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Transaction {
                    namespace: namespace__.unwrap_or_default(),
                    payload: payload__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.Transaction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionCountResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.count != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TransactionCountResponse", len)?;
        if self.count != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("count", ToString::to_string(&self.count).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionCountResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "count",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Count,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "count" => Ok(GeneratedField::Count),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionCountResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TransactionCountResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransactionCountResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut count__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Count => {
                            if count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("count"));
                            }
                            count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(TransactionCountResponse {
                    count: count__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TransactionCountResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.transaction.is_some() {
            len += 1;
        }
        if !self.hash.is_empty() {
            len += 1;
        }
        if self.index != 0 {
            len += 1;
        }
        if !self.block_hash.is_empty() {
            len += 1;
        }
        if self.block_height != 0 {
            len += 1;
        }
        if self.namespace != 0 {
            len += 1;
        }
        if self.pos_in_namespace != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TransactionResponse", len)?;
        if let Some(v) = self.transaction.as_ref() {
            struct_ser.serialize_field("transaction", v)?;
        }
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", &self.hash)?;
        }
        if self.index != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("index", ToString::to_string(&self.index).as_str())?;
        }
        if !self.block_hash.is_empty() {
            struct_ser.serialize_field("blockHash", &self.block_hash)?;
        }
        if self.block_height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("blockHeight", ToString::to_string(&self.block_height).as_str())?;
        }
        if self.namespace != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("namespace", ToString::to_string(&self.namespace).as_str())?;
        }
        if self.pos_in_namespace != 0 {
            struct_ser.serialize_field("posInNamespace", &self.pos_in_namespace)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transaction",
            "hash",
            "index",
            "block_hash",
            "blockHash",
            "block_height",
            "blockHeight",
            "namespace",
            "pos_in_namespace",
            "posInNamespace",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Transaction,
            Hash,
            Index,
            BlockHash,
            BlockHeight,
            Namespace,
            PosInNamespace,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "transaction" => Ok(GeneratedField::Transaction),
                            "hash" => Ok(GeneratedField::Hash),
                            "index" => Ok(GeneratedField::Index),
                            "blockHash" | "block_hash" => Ok(GeneratedField::BlockHash),
                            "blockHeight" | "block_height" => Ok(GeneratedField::BlockHeight),
                            "namespace" => Ok(GeneratedField::Namespace),
                            "posInNamespace" | "pos_in_namespace" => Ok(GeneratedField::PosInNamespace),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TransactionResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransactionResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transaction__ = None;
                let mut hash__ = None;
                let mut index__ = None;
                let mut block_hash__ = None;
                let mut block_height__ = None;
                let mut namespace__ = None;
                let mut pos_in_namespace__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Transaction => {
                            if transaction__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transaction"));
                            }
                            transaction__ = map_.next_value()?;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BlockHash => {
                            if block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHash"));
                            }
                            block_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BlockHeight => {
                            if block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHeight"));
                            }
                            block_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Namespace => {
                            if namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespace"));
                            }
                            namespace__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PosInNamespace => {
                            if pos_in_namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("posInNamespace"));
                            }
                            pos_in_namespace__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(TransactionResponse {
                    transaction: transaction__,
                    hash: hash__.unwrap_or_default(),
                    index: index__.unwrap_or_default(),
                    block_hash: block_hash__.unwrap_or_default(),
                    block_height: block_height__.unwrap_or_default(),
                    namespace: namespace__.unwrap_or_default(),
                    pos_in_namespace: pos_in_namespace__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TransactionResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionWithProofResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.transaction.is_some() {
            len += 1;
        }
        if !self.hash.is_empty() {
            len += 1;
        }
        if self.index != 0 {
            len += 1;
        }
        if !self.block_hash.is_empty() {
            len += 1;
        }
        if self.block_height != 0 {
            len += 1;
        }
        if self.namespace != 0 {
            len += 1;
        }
        if self.pos_in_namespace != 0 {
            len += 1;
        }
        if self.proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TransactionWithProofResponse", len)?;
        if let Some(v) = self.transaction.as_ref() {
            struct_ser.serialize_field("transaction", v)?;
        }
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", &self.hash)?;
        }
        if self.index != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("index", ToString::to_string(&self.index).as_str())?;
        }
        if !self.block_hash.is_empty() {
            struct_ser.serialize_field("blockHash", &self.block_hash)?;
        }
        if self.block_height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("blockHeight", ToString::to_string(&self.block_height).as_str())?;
        }
        if self.namespace != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("namespace", ToString::to_string(&self.namespace).as_str())?;
        }
        if self.pos_in_namespace != 0 {
            struct_ser.serialize_field("posInNamespace", &self.pos_in_namespace)?;
        }
        if let Some(v) = self.proof.as_ref() {
            struct_ser.serialize_field("proof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionWithProofResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transaction",
            "hash",
            "index",
            "block_hash",
            "blockHash",
            "block_height",
            "blockHeight",
            "namespace",
            "pos_in_namespace",
            "posInNamespace",
            "proof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Transaction,
            Hash,
            Index,
            BlockHash,
            BlockHeight,
            Namespace,
            PosInNamespace,
            Proof,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "transaction" => Ok(GeneratedField::Transaction),
                            "hash" => Ok(GeneratedField::Hash),
                            "index" => Ok(GeneratedField::Index),
                            "blockHash" | "block_hash" => Ok(GeneratedField::BlockHash),
                            "blockHeight" | "block_height" => Ok(GeneratedField::BlockHeight),
                            "namespace" => Ok(GeneratedField::Namespace),
                            "posInNamespace" | "pos_in_namespace" => Ok(GeneratedField::PosInNamespace),
                            "proof" => Ok(GeneratedField::Proof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionWithProofResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TransactionWithProofResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransactionWithProofResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transaction__ = None;
                let mut hash__ = None;
                let mut index__ = None;
                let mut block_hash__ = None;
                let mut block_height__ = None;
                let mut namespace__ = None;
                let mut pos_in_namespace__ = None;
                let mut proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Transaction => {
                            if transaction__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transaction"));
                            }
                            transaction__ = map_.next_value()?;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BlockHash => {
                            if block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHash"));
                            }
                            block_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BlockHeight => {
                            if block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHeight"));
                            }
                            block_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Namespace => {
                            if namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespace"));
                            }
                            namespace__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PosInNamespace => {
                            if pos_in_namespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("posInNamespace"));
                            }
                            pos_in_namespace__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Proof => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proof"));
                            }
                            proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(TransactionWithProofResponse {
                    transaction: transaction__,
                    hash: hash__.unwrap_or_default(),
                    index: index__.unwrap_or_default(),
                    block_hash: block_hash__.unwrap_or_default(),
                    block_height: block_height__.unwrap_or_default(),
                    namespace: namespace__.unwrap_or_default(),
                    pos_in_namespace: pos_in_namespace__.unwrap_or_default(),
                    proof: proof__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TransactionWithProofResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TxProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.TxProof", len)?;
        if let Some(v) = self.proof.as_ref() {
            match v {
                tx_proof::Proof::V0(v) => {
                    struct_ser.serialize_field("v0", v)?;
                }
                tx_proof::Proof::V1(v) => {
                    struct_ser.serialize_field("v1", v)?;
                }
                tx_proof::Proof::V2(v) => {
                    struct_ser.serialize_field("v2", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TxProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "v0",
            "v1",
            "v2",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            V0,
            V1,
            V2,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "v0" => Ok(GeneratedField::V0),
                            "v1" => Ok(GeneratedField::V1),
                            "v2" => Ok(GeneratedField::V2),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TxProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.TxProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TxProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::V0 => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v0"));
                            }
                            proof__ = map_.next_value::<::std::option::Option<_>>()?.map(tx_proof::Proof::V0)
;
                        }
                        GeneratedField::V1 => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v1"));
                            }
                            proof__ = map_.next_value::<::std::option::Option<_>>()?.map(tx_proof::Proof::V1)
;
                        }
                        GeneratedField::V2 => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v2"));
                            }
                            proof__ = map_.next_value::<::std::option::Option<_>>()?.map(tx_proof::Proof::V2)
;
                        }
                    }
                }
                Ok(TxProof {
                    proof: proof__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.TxProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UpgradeCertificate {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.data.is_some() {
            len += 1;
        }
        if !self.vote_commitment.is_empty() {
            len += 1;
        }
        if self.view_number != 0 {
            len += 1;
        }
        if self.signatures.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.UpgradeCertificate", len)?;
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        if !self.vote_commitment.is_empty() {
            struct_ser.serialize_field("voteCommitment", &self.vote_commitment)?;
        }
        if self.view_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("viewNumber", ToString::to_string(&self.view_number).as_str())?;
        }
        if let Some(v) = self.signatures.as_ref() {
            struct_ser.serialize_field("signatures", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UpgradeCertificate {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "data",
            "vote_commitment",
            "voteCommitment",
            "view_number",
            "viewNumber",
            "signatures",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Data,
            VoteCommitment,
            ViewNumber,
            Signatures,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "data" => Ok(GeneratedField::Data),
                            "voteCommitment" | "vote_commitment" => Ok(GeneratedField::VoteCommitment),
                            "viewNumber" | "view_number" => Ok(GeneratedField::ViewNumber),
                            "signatures" => Ok(GeneratedField::Signatures),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UpgradeCertificate;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.UpgradeCertificate")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UpgradeCertificate, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut data__ = None;
                let mut vote_commitment__ = None;
                let mut view_number__ = None;
                let mut signatures__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map_.next_value()?;
                        }
                        GeneratedField::VoteCommitment => {
                            if vote_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("voteCommitment"));
                            }
                            vote_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ViewNumber => {
                            if view_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("viewNumber"));
                            }
                            view_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = map_.next_value()?;
                        }
                    }
                }
                Ok(UpgradeCertificate {
                    data: data__,
                    vote_commitment: vote_commitment__.unwrap_or_default(),
                    view_number: view_number__.unwrap_or_default(),
                    signatures: signatures__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.UpgradeCertificate", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UpgradeProposalData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.old_version.is_some() {
            len += 1;
        }
        if self.new_version.is_some() {
            len += 1;
        }
        if self.decide_by != 0 {
            len += 1;
        }
        if !self.new_version_hash.is_empty() {
            len += 1;
        }
        if self.old_version_last_view != 0 {
            len += 1;
        }
        if self.new_version_first_view != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.UpgradeProposalData", len)?;
        if let Some(v) = self.old_version.as_ref() {
            struct_ser.serialize_field("oldVersion", v)?;
        }
        if let Some(v) = self.new_version.as_ref() {
            struct_ser.serialize_field("newVersion", v)?;
        }
        if self.decide_by != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("decideBy", ToString::to_string(&self.decide_by).as_str())?;
        }
        if !self.new_version_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("newVersionHash", pbjson::private::base64::encode(&self.new_version_hash).as_str())?;
        }
        if self.old_version_last_view != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("oldVersionLastView", ToString::to_string(&self.old_version_last_view).as_str())?;
        }
        if self.new_version_first_view != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("newVersionFirstView", ToString::to_string(&self.new_version_first_view).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UpgradeProposalData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "old_version",
            "oldVersion",
            "new_version",
            "newVersion",
            "decide_by",
            "decideBy",
            "new_version_hash",
            "newVersionHash",
            "old_version_last_view",
            "oldVersionLastView",
            "new_version_first_view",
            "newVersionFirstView",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            OldVersion,
            NewVersion,
            DecideBy,
            NewVersionHash,
            OldVersionLastView,
            NewVersionFirstView,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "oldVersion" | "old_version" => Ok(GeneratedField::OldVersion),
                            "newVersion" | "new_version" => Ok(GeneratedField::NewVersion),
                            "decideBy" | "decide_by" => Ok(GeneratedField::DecideBy),
                            "newVersionHash" | "new_version_hash" => Ok(GeneratedField::NewVersionHash),
                            "oldVersionLastView" | "old_version_last_view" => Ok(GeneratedField::OldVersionLastView),
                            "newVersionFirstView" | "new_version_first_view" => Ok(GeneratedField::NewVersionFirstView),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UpgradeProposalData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.UpgradeProposalData")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UpgradeProposalData, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut old_version__ = None;
                let mut new_version__ = None;
                let mut decide_by__ = None;
                let mut new_version_hash__ = None;
                let mut old_version_last_view__ = None;
                let mut new_version_first_view__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::OldVersion => {
                            if old_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("oldVersion"));
                            }
                            old_version__ = map_.next_value()?;
                        }
                        GeneratedField::NewVersion => {
                            if new_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("newVersion"));
                            }
                            new_version__ = map_.next_value()?;
                        }
                        GeneratedField::DecideBy => {
                            if decide_by__.is_some() {
                                return Err(serde::de::Error::duplicate_field("decideBy"));
                            }
                            decide_by__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NewVersionHash => {
                            if new_version_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("newVersionHash"));
                            }
                            new_version_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::OldVersionLastView => {
                            if old_version_last_view__.is_some() {
                                return Err(serde::de::Error::duplicate_field("oldVersionLastView"));
                            }
                            old_version_last_view__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NewVersionFirstView => {
                            if new_version_first_view__.is_some() {
                                return Err(serde::de::Error::duplicate_field("newVersionFirstView"));
                            }
                            new_version_first_view__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(UpgradeProposalData {
                    old_version: old_version__,
                    new_version: new_version__,
                    decide_by: decide_by__.unwrap_or_default(),
                    new_version_hash: new_version_hash__.unwrap_or_default(),
                    old_version_last_view: old_version_last_view__.unwrap_or_default(),
                    new_version_first_view: new_version_first_view__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.UpgradeProposalData", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Version {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.major != 0 {
            len += 1;
        }
        if self.minor != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.Version", len)?;
        if self.major != 0 {
            struct_ser.serialize_field("major", &self.major)?;
        }
        if self.minor != 0 {
            struct_ser.serialize_field("minor", &self.minor)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Version {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "major",
            "minor",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Major,
            Minor,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "major" => Ok(GeneratedField::Major),
                            "minor" => Ok(GeneratedField::Minor),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Version;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.Version")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Version, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut major__ = None;
                let mut minor__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Major => {
                            if major__.is_some() {
                                return Err(serde::de::Error::duplicate_field("major"));
                            }
                            major__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Minor => {
                            if minor__.is_some() {
                                return Err(serde::de::Error::duplicate_field("minor"));
                            }
                            minor__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Version {
                    major: major__.unwrap_or_default(),
                    minor: minor__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.Version", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for VidCommonRangeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.vid_common.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.VidCommonRangeResponse", len)?;
        if !self.vid_common.is_empty() {
            struct_ser.serialize_field("vidCommon", &self.vid_common)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for VidCommonRangeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "vid_common",
            "vidCommon",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            VidCommon,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "vidCommon" | "vid_common" => Ok(GeneratedField::VidCommon),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = VidCommonRangeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.VidCommonRangeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<VidCommonRangeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut vid_common__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::VidCommon => {
                            if vid_common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("vidCommon"));
                            }
                            vid_common__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(VidCommonRangeResponse {
                    vid_common: vid_common__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.VidCommonRangeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for VidCommonResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height != 0 {
            len += 1;
        }
        if !self.block_hash.is_empty() {
            len += 1;
        }
        if !self.payload_hash.is_empty() {
            len += 1;
        }
        if self.common.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.VidCommonResponse", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if !self.block_hash.is_empty() {
            struct_ser.serialize_field("blockHash", &self.block_hash)?;
        }
        if !self.payload_hash.is_empty() {
            struct_ser.serialize_field("payloadHash", &self.payload_hash)?;
        }
        if let Some(v) = self.common.as_ref() {
            match v {
                vid_common_response::Common::V0(v) => {
                    struct_ser.serialize_field("v0", v)?;
                }
                vid_common_response::Common::V1(v) => {
                    struct_ser.serialize_field("v1", v)?;
                }
                vid_common_response::Common::V2(v) => {
                    struct_ser.serialize_field("v2", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for VidCommonResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "block_hash",
            "blockHash",
            "payload_hash",
            "payloadHash",
            "v0",
            "v1",
            "v2",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            BlockHash,
            PayloadHash,
            V0,
            V1,
            V2,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "height" => Ok(GeneratedField::Height),
                            "blockHash" | "block_hash" => Ok(GeneratedField::BlockHash),
                            "payloadHash" | "payload_hash" => Ok(GeneratedField::PayloadHash),
                            "v0" => Ok(GeneratedField::V0),
                            "v1" => Ok(GeneratedField::V1),
                            "v2" => Ok(GeneratedField::V2),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = VidCommonResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.VidCommonResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<VidCommonResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut block_hash__ = None;
                let mut payload_hash__ = None;
                let mut common__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BlockHash => {
                            if block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHash"));
                            }
                            block_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::PayloadHash => {
                            if payload_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadHash"));
                            }
                            payload_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::V0 => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v0"));
                            }
                            common__ = map_.next_value::<::std::option::Option<_>>()?.map(vid_common_response::Common::V0)
;
                        }
                        GeneratedField::V1 => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v1"));
                            }
                            common__ = map_.next_value::<::std::option::Option<_>>()?.map(vid_common_response::Common::V1)
;
                        }
                        GeneratedField::V2 => {
                            if common__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v2"));
                            }
                            common__ = map_.next_value::<::std::option::Option<_>>()?.map(vid_common_response::Common::V2)
;
                        }
                    }
                }
                Ok(VidCommonResponse {
                    height: height__.unwrap_or_default(),
                    block_hash: block_hash__.unwrap_or_default(),
                    payload_hash: payload_hash__.unwrap_or_default(),
                    common: common__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.VidCommonResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ViewChangeEvidence2 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.evidence.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.ViewChangeEvidence2", len)?;
        if let Some(v) = self.evidence.as_ref() {
            match v {
                view_change_evidence2::Evidence::Timeout(v) => {
                    struct_ser.serialize_field("timeout", v)?;
                }
                view_change_evidence2::Evidence::ViewSync(v) => {
                    struct_ser.serialize_field("viewSync", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ViewChangeEvidence2 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "timeout",
            "view_sync",
            "viewSync",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Timeout,
            ViewSync,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "timeout" => Ok(GeneratedField::Timeout),
                            "viewSync" | "view_sync" => Ok(GeneratedField::ViewSync),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ViewChangeEvidence2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.ViewChangeEvidence2")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ViewChangeEvidence2, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut evidence__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Timeout => {
                            if evidence__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timeout"));
                            }
                            evidence__ = map_.next_value::<::std::option::Option<_>>()?.map(view_change_evidence2::Evidence::Timeout)
;
                        }
                        GeneratedField::ViewSync => {
                            if evidence__.is_some() {
                                return Err(serde::de::Error::duplicate_field("viewSync"));
                            }
                            evidence__ = map_.next_value::<::std::option::Option<_>>()?.map(view_change_evidence2::Evidence::ViewSync)
;
                        }
                    }
                }
                Ok(ViewChangeEvidence2 {
                    evidence: evidence__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.ViewChangeEvidence2", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ViewSyncFinalizeCertificate2 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.data.is_some() {
            len += 1;
        }
        if !self.vote_commitment.is_empty() {
            len += 1;
        }
        if self.view_number != 0 {
            len += 1;
        }
        if self.signatures.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.ViewSyncFinalizeCertificate2", len)?;
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        if !self.vote_commitment.is_empty() {
            struct_ser.serialize_field("voteCommitment", &self.vote_commitment)?;
        }
        if self.view_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("viewNumber", ToString::to_string(&self.view_number).as_str())?;
        }
        if let Some(v) = self.signatures.as_ref() {
            struct_ser.serialize_field("signatures", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ViewSyncFinalizeCertificate2 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "data",
            "vote_commitment",
            "voteCommitment",
            "view_number",
            "viewNumber",
            "signatures",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Data,
            VoteCommitment,
            ViewNumber,
            Signatures,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "data" => Ok(GeneratedField::Data),
                            "voteCommitment" | "vote_commitment" => Ok(GeneratedField::VoteCommitment),
                            "viewNumber" | "view_number" => Ok(GeneratedField::ViewNumber),
                            "signatures" => Ok(GeneratedField::Signatures),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ViewSyncFinalizeCertificate2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.ViewSyncFinalizeCertificate2")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ViewSyncFinalizeCertificate2, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut data__ = None;
                let mut vote_commitment__ = None;
                let mut view_number__ = None;
                let mut signatures__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map_.next_value()?;
                        }
                        GeneratedField::VoteCommitment => {
                            if vote_commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("voteCommitment"));
                            }
                            vote_commitment__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ViewNumber => {
                            if view_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("viewNumber"));
                            }
                            view_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ViewSyncFinalizeCertificate2 {
                    data: data__,
                    vote_commitment: vote_commitment__.unwrap_or_default(),
                    view_number: view_number__.unwrap_or_default(),
                    signatures: signatures__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.ViewSyncFinalizeCertificate2", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ViewSyncFinalizeData2 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.relay != 0 {
            len += 1;
        }
        if self.round != 0 {
            len += 1;
        }
        if self.epoch.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.ViewSyncFinalizeData2", len)?;
        if self.relay != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("relay", ToString::to_string(&self.relay).as_str())?;
        }
        if self.round != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("round", ToString::to_string(&self.round).as_str())?;
        }
        if let Some(v) = self.epoch.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ViewSyncFinalizeData2 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "relay",
            "round",
            "epoch",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Relay,
            Round,
            Epoch,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "relay" => Ok(GeneratedField::Relay),
                            "round" => Ok(GeneratedField::Round),
                            "epoch" => Ok(GeneratedField::Epoch),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ViewSyncFinalizeData2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.ViewSyncFinalizeData2")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ViewSyncFinalizeData2, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut relay__ = None;
                let mut round__ = None;
                let mut epoch__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Relay => {
                            if relay__.is_some() {
                                return Err(serde::de::Error::duplicate_field("relay"));
                            }
                            relay__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Round => {
                            if round__.is_some() {
                                return Err(serde::de::Error::duplicate_field("round"));
                            }
                            round__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(ViewSyncFinalizeData2 {
                    relay: relay__.unwrap_or_default(),
                    round: round__.unwrap_or_default(),
                    epoch: epoch__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.ViewSyncFinalizeData2", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Vote2Data {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.leaf_commit.is_empty() {
            len += 1;
        }
        if self.epoch != 0 {
            len += 1;
        }
        if self.block_number != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.Vote2Data", len)?;
        if !self.leaf_commit.is_empty() {
            struct_ser.serialize_field("leafCommit", &self.leaf_commit)?;
        }
        if self.epoch != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&self.epoch).as_str())?;
        }
        if self.block_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("blockNumber", ToString::to_string(&self.block_number).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Vote2Data {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "leaf_commit",
            "leafCommit",
            "epoch",
            "block_number",
            "blockNumber",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            LeafCommit,
            Epoch,
            BlockNumber,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "leafCommit" | "leaf_commit" => Ok(GeneratedField::LeafCommit),
                            "epoch" => Ok(GeneratedField::Epoch),
                            "blockNumber" | "block_number" => Ok(GeneratedField::BlockNumber),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Vote2Data;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.Vote2Data")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Vote2Data, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut leaf_commit__ = None;
                let mut epoch__ = None;
                let mut block_number__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::LeafCommit => {
                            if leaf_commit__.is_some() {
                                return Err(serde::de::Error::duplicate_field("leafCommit"));
                            }
                            leaf_commit__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BlockNumber => {
                            if block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockNumber"));
                            }
                            block_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Vote2Data {
                    leaf_commit: leaf_commit__.unwrap_or_default(),
                    epoch: epoch__.unwrap_or_default(),
                    block_number: block_number__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.Vote2Data", FIELDS, GeneratedVisitor)
    }
}
