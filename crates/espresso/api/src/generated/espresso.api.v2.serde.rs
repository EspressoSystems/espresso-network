impl serde::Serialize for AdvzMerkleNode {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.node.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AdvzMerkleNode", len)?;
        if let Some(v) = self.node.as_ref() {
            match v {
                advz_merkle_node::Node::Leaf(v) => {
                    struct_ser.serialize_field("leaf", v)?;
                }
                advz_merkle_node::Node::Branch(v) => {
                    struct_ser.serialize_field("branch", v)?;
                }
                advz_merkle_node::Node::ForgottenSubtree(v) => {
                    struct_ser.serialize_field("forgottenSubtree", v)?;
                }
                advz_merkle_node::Node::Empty(v) => {
                    struct_ser.serialize_field("empty", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AdvzMerkleNode {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "leaf",
            "branch",
            "forgotten_subtree",
            "forgottenSubtree",
            "empty",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Leaf,
            Branch,
            ForgottenSubtree,
            Empty,
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
                            "branch" => Ok(GeneratedField::Branch),
                            "forgottenSubtree" | "forgotten_subtree" => Ok(GeneratedField::ForgottenSubtree),
                            "empty" => Ok(GeneratedField::Empty),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AdvzMerkleNode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AdvzMerkleNode")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AdvzMerkleNode, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut node__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Leaf => {
                            if node__.is_some() {
                                return Err(serde::de::Error::duplicate_field("leaf"));
                            }
                            node__ = map_.next_value::<::std::option::Option<_>>()?.map(advz_merkle_node::Node::Leaf)
;
                        }
                        GeneratedField::Branch => {
                            if node__.is_some() {
                                return Err(serde::de::Error::duplicate_field("branch"));
                            }
                            node__ = map_.next_value::<::std::option::Option<_>>()?.map(advz_merkle_node::Node::Branch)
;
                        }
                        GeneratedField::ForgottenSubtree => {
                            if node__.is_some() {
                                return Err(serde::de::Error::duplicate_field("forgottenSubtree"));
                            }
                            node__ = map_.next_value::<::std::option::Option<_>>()?.map(advz_merkle_node::Node::ForgottenSubtree)
;
                        }
                        GeneratedField::Empty => {
                            if node__.is_some() {
                                return Err(serde::de::Error::duplicate_field("empty"));
                            }
                            node__ = map_.next_value::<::std::option::Option<_>>()?.map(advz_merkle_node::Node::Empty)
;
                        }
                    }
                }
                Ok(AdvzMerkleNode {
                    node: node__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AdvzMerkleNode", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AdvzMerkleNodeBranch {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.children.is_empty() {
            len += 1;
        }
        if !self.value.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AdvzMerkleNodeBranch", len)?;
        if !self.children.is_empty() {
            struct_ser.serialize_field("children", &self.children)?;
        }
        if !self.value.is_empty() {
            struct_ser.serialize_field("value", &self.value)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AdvzMerkleNodeBranch {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "children",
            "value",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Children,
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
                            "children" => Ok(GeneratedField::Children),
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
            type Value = AdvzMerkleNodeBranch;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AdvzMerkleNodeBranch")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AdvzMerkleNodeBranch, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut children__ = None;
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Children => {
                            if children__.is_some() {
                                return Err(serde::de::Error::duplicate_field("children"));
                            }
                            children__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Value => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AdvzMerkleNodeBranch {
                    children: children__.unwrap_or_default(),
                    value: value__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AdvzMerkleNodeBranch", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AdvzMerkleNodeEmpty {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.AdvzMerkleNodeEmpty", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AdvzMerkleNodeEmpty {
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
            type Value = AdvzMerkleNodeEmpty;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AdvzMerkleNodeEmpty")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AdvzMerkleNodeEmpty, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(AdvzMerkleNodeEmpty {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AdvzMerkleNodeEmpty", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AdvzMerkleNodeForgottenSubtree {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.value.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AdvzMerkleNodeForgottenSubtree", len)?;
        if !self.value.is_empty() {
            struct_ser.serialize_field("value", &self.value)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AdvzMerkleNodeForgottenSubtree {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "value",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
            type Value = AdvzMerkleNodeForgottenSubtree;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AdvzMerkleNodeForgottenSubtree")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AdvzMerkleNodeForgottenSubtree, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Value => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AdvzMerkleNodeForgottenSubtree {
                    value: value__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AdvzMerkleNodeForgottenSubtree", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AdvzMerkleNodeLeaf {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.elem.is_empty() {
            len += 1;
        }
        if !self.pos.is_empty() {
            len += 1;
        }
        if !self.value.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AdvzMerkleNodeLeaf", len)?;
        if !self.elem.is_empty() {
            struct_ser.serialize_field("elem", &self.elem)?;
        }
        if !self.pos.is_empty() {
            struct_ser.serialize_field("pos", &self.pos)?;
        }
        if !self.value.is_empty() {
            struct_ser.serialize_field("value", &self.value)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AdvzMerkleNodeLeaf {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "elem",
            "pos",
            "value",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Elem,
            Pos,
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
                            "elem" => Ok(GeneratedField::Elem),
                            "pos" => Ok(GeneratedField::Pos),
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
            type Value = AdvzMerkleNodeLeaf;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AdvzMerkleNodeLeaf")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AdvzMerkleNodeLeaf, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut elem__ = None;
                let mut pos__ = None;
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Elem => {
                            if elem__.is_some() {
                                return Err(serde::de::Error::duplicate_field("elem"));
                            }
                            elem__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Pos => {
                            if pos__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pos"));
                            }
                            pos__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Value => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AdvzMerkleNodeLeaf {
                    elem: elem__.unwrap_or_default(),
                    pos: pos__.unwrap_or_default(),
                    value: value__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AdvzMerkleNodeLeaf", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AdvzMerkleProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.pos.is_empty() {
            len += 1;
        }
        if !self.proof.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AdvzMerkleProof", len)?;
        if !self.pos.is_empty() {
            struct_ser.serialize_field("pos", &self.pos)?;
        }
        if !self.proof.is_empty() {
            struct_ser.serialize_field("proof", &self.proof)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AdvzMerkleProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "pos",
            "proof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Pos,
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
                            "pos" => Ok(GeneratedField::Pos),
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
            type Value = AdvzMerkleProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AdvzMerkleProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AdvzMerkleProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut pos__ = None;
                let mut proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Pos => {
                            if pos__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pos"));
                            }
                            pos__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Proof => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proof"));
                            }
                            proof__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AdvzMerkleProof {
                    pos: pos__.unwrap_or_default(),
                    proof: proof__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AdvzMerkleProof", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AdvzVidShare {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.aggregate_proofs.is_empty() {
            len += 1;
        }
        if !self.evals.is_empty() {
            len += 1;
        }
        if self.evals_proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AdvzVidShare", len)?;
        if !self.aggregate_proofs.is_empty() {
            struct_ser.serialize_field("aggregateProofs", &self.aggregate_proofs)?;
        }
        if !self.evals.is_empty() {
            struct_ser.serialize_field("evals", &self.evals)?;
        }
        if let Some(v) = self.evals_proof.as_ref() {
            struct_ser.serialize_field("evalsProof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AdvzVidShare {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "aggregate_proofs",
            "aggregateProofs",
            "evals",
            "evals_proof",
            "evalsProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AggregateProofs,
            Evals,
            EvalsProof,
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
                            "aggregateProofs" | "aggregate_proofs" => Ok(GeneratedField::AggregateProofs),
                            "evals" => Ok(GeneratedField::Evals),
                            "evalsProof" | "evals_proof" => Ok(GeneratedField::EvalsProof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AdvzVidShare;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AdvzVidShare")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AdvzVidShare, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut aggregate_proofs__ = None;
                let mut evals__ = None;
                let mut evals_proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::AggregateProofs => {
                            if aggregate_proofs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("aggregateProofs"));
                            }
                            aggregate_proofs__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Evals => {
                            if evals__.is_some() {
                                return Err(serde::de::Error::duplicate_field("evals"));
                            }
                            evals__ = Some(map_.next_value()?);
                        }
                        GeneratedField::EvalsProof => {
                            if evals_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("evalsProof"));
                            }
                            evals_proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(AdvzVidShare {
                    aggregate_proofs: aggregate_proofs__.unwrap_or_default(),
                    evals: evals__.unwrap_or_default(),
                    evals_proof: evals_proof__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AdvzVidShare", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AllValidatorsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.validators.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AllValidatorsResponse", len)?;
        if !self.validators.is_empty() {
            struct_ser.serialize_field("validators", &self.validators)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AllValidatorsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "validators",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Validators,
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
                            "validators" => Ok(GeneratedField::Validators),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AllValidatorsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AllValidatorsResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AllValidatorsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut validators__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Validators => {
                            if validators__.is_some() {
                                return Err(serde::de::Error::duplicate_field("validators"));
                            }
                            validators__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AllValidatorsResponse {
                    validators: validators__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AllValidatorsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AvidmGf2VidShare {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.range.is_some() {
            len += 1;
        }
        if !self.payload.is_empty() {
            len += 1;
        }
        if !self.mt_proofs.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AvidmGf2VidShare", len)?;
        if let Some(v) = self.range.as_ref() {
            struct_ser.serialize_field("range", v)?;
        }
        if !self.payload.is_empty() {
            struct_ser.serialize_field("payload", &self.payload.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if !self.mt_proofs.is_empty() {
            struct_ser.serialize_field("mtProofs", &self.mt_proofs)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AvidmGf2VidShare {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "range",
            "payload",
            "mt_proofs",
            "mtProofs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Range,
            Payload,
            MtProofs,
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
                            "range" => Ok(GeneratedField::Range),
                            "payload" => Ok(GeneratedField::Payload),
                            "mtProofs" | "mt_proofs" => Ok(GeneratedField::MtProofs),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AvidmGf2VidShare;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AvidmGf2VidShare")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AvidmGf2VidShare, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut range__ = None;
                let mut payload__ = None;
                let mut mt_proofs__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Range => {
                            if range__.is_some() {
                                return Err(serde::de::Error::duplicate_field("range"));
                            }
                            range__ = map_.next_value()?;
                        }
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::MtProofs => {
                            if mt_proofs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mtProofs"));
                            }
                            mt_proofs__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AvidmGf2VidShare {
                    range: range__,
                    payload: payload__.unwrap_or_default(),
                    mt_proofs: mt_proofs__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AvidmGf2VidShare", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AvidmShareContent {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.range.is_some() {
            len += 1;
        }
        if !self.payload.is_empty() {
            len += 1;
        }
        if !self.mt_proofs.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AvidmShareContent", len)?;
        if let Some(v) = self.range.as_ref() {
            struct_ser.serialize_field("range", v)?;
        }
        if !self.payload.is_empty() {
            struct_ser.serialize_field("payload", &self.payload)?;
        }
        if !self.mt_proofs.is_empty() {
            struct_ser.serialize_field("mtProofs", &self.mt_proofs)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AvidmShareContent {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "range",
            "payload",
            "mt_proofs",
            "mtProofs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Range,
            Payload,
            MtProofs,
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
                            "range" => Ok(GeneratedField::Range),
                            "payload" => Ok(GeneratedField::Payload),
                            "mtProofs" | "mt_proofs" => Ok(GeneratedField::MtProofs),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AvidmShareContent;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AvidmShareContent")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AvidmShareContent, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut range__ = None;
                let mut payload__ = None;
                let mut mt_proofs__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Range => {
                            if range__.is_some() {
                                return Err(serde::de::Error::duplicate_field("range"));
                            }
                            range__ = map_.next_value()?;
                        }
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ = Some(map_.next_value()?);
                        }
                        GeneratedField::MtProofs => {
                            if mt_proofs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mtProofs"));
                            }
                            mt_proofs__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AvidmShareContent {
                    range: range__,
                    payload: payload__.unwrap_or_default(),
                    mt_proofs: mt_proofs__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AvidmShareContent", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AvidmVidShare {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.index != 0 {
            len += 1;
        }
        if self.payload_byte_len != 0 {
            len += 1;
        }
        if self.content.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.AvidmVidShare", len)?;
        if self.index != 0 {
            struct_ser.serialize_field("index", &self.index)?;
        }
        if self.payload_byte_len != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("payloadByteLen", ToString::to_string(&self.payload_byte_len).as_str())?;
        }
        if let Some(v) = self.content.as_ref() {
            struct_ser.serialize_field("content", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AvidmVidShare {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "index",
            "payload_byte_len",
            "payloadByteLen",
            "content",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Index,
            PayloadByteLen,
            Content,
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
                            "index" => Ok(GeneratedField::Index),
                            "payloadByteLen" | "payload_byte_len" => Ok(GeneratedField::PayloadByteLen),
                            "content" => Ok(GeneratedField::Content),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AvidmVidShare;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.AvidmVidShare")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AvidmVidShare, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut index__ = None;
                let mut payload_byte_len__ = None;
                let mut content__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PayloadByteLen => {
                            if payload_byte_len__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadByteLen"));
                            }
                            payload_byte_len__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Content => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("content"));
                            }
                            content__ = map_.next_value()?;
                        }
                    }
                }
                Ok(AvidmVidShare {
                    index: index__.unwrap_or_default(),
                    payload_byte_len: payload_byte_len__.unwrap_or_default(),
                    content: content__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.AvidmVidShare", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for Delegator {
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
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.Delegator", len)?;
        if !self.account.is_empty() {
            struct_ser.serialize_field("account", &self.account)?;
        }
        if !self.amount.is_empty() {
            struct_ser.serialize_field("amount", &self.amount)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Delegator {
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
            type Value = Delegator;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.Delegator")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Delegator, V::Error>
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
                Ok(Delegator {
                    account: account__.unwrap_or_default(),
                    amount: amount__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.Delegator", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for GetAllValidatorsRequest {
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
        if self.offset != 0 {
            len += 1;
        }
        if self.limit != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetAllValidatorsRequest", len)?;
        if self.epoch != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&self.epoch).as_str())?;
        }
        if self.offset != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("offset", ToString::to_string(&self.offset).as_str())?;
        }
        if self.limit != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("limit", ToString::to_string(&self.limit).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetAllValidatorsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "epoch",
            "offset",
            "limit",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Epoch,
            Offset,
            Limit,
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
                            "offset" => Ok(GeneratedField::Offset),
                            "limit" => Ok(GeneratedField::Limit),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetAllValidatorsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetAllValidatorsRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetAllValidatorsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut epoch__ = None;
                let mut offset__ = None;
                let mut limit__ = None;
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
                        GeneratedField::Offset => {
                            if offset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("offset"));
                            }
                            offset__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Limit => {
                            if limit__.is_some() {
                                return Err(serde::de::Error::duplicate_field("limit"));
                            }
                            limit__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(GetAllValidatorsRequest {
                    epoch: epoch__.unwrap_or_default(),
                    offset: offset__.unwrap_or_default(),
                    limit: limit__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetAllValidatorsRequest", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for GetHeaderWindowRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.start_time.is_some() {
            len += 1;
        }
        if self.start_height.is_some() {
            len += 1;
        }
        if self.start_hash.is_some() {
            len += 1;
        }
        if self.end != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetHeaderWindowRequest", len)?;
        if let Some(v) = self.start_time.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("startTime", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.start_height.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("startHeight", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.start_hash.as_ref() {
            struct_ser.serialize_field("startHash", v)?;
        }
        if self.end != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("end", ToString::to_string(&self.end).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetHeaderWindowRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start_time",
            "startTime",
            "start_height",
            "startHeight",
            "start_hash",
            "startHash",
            "end",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartTime,
            StartHeight,
            StartHash,
            End,
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
                            "startTime" | "start_time" => Ok(GeneratedField::StartTime),
                            "startHeight" | "start_height" => Ok(GeneratedField::StartHeight),
                            "startHash" | "start_hash" => Ok(GeneratedField::StartHash),
                            "end" => Ok(GeneratedField::End),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetHeaderWindowRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetHeaderWindowRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetHeaderWindowRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start_time__ = None;
                let mut start_height__ = None;
                let mut start_hash__ = None;
                let mut end__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::StartTime => {
                            if start_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startTime"));
                            }
                            start_time__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::StartHeight => {
                            if start_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startHeight"));
                            }
                            start_height__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::StartHash => {
                            if start_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startHash"));
                            }
                            start_hash__ = map_.next_value()?;
                        }
                        GeneratedField::End => {
                            if end__.is_some() {
                                return Err(serde::de::Error::duplicate_field("end"));
                            }
                            end__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(GetHeaderWindowRequest {
                    start_time: start_time__,
                    start_height: start_height__,
                    start_hash: start_hash__,
                    end: end__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetHeaderWindowRequest", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for GetNodeBlockHeightRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetNodeBlockHeightRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetNodeBlockHeightRequest {
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
            type Value = GetNodeBlockHeightRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetNodeBlockHeightRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetNodeBlockHeightRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetNodeBlockHeightRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetNodeBlockHeightRequest", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for GetNodeLimitsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("espresso.api.v2.GetNodeLimitsRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetNodeLimitsRequest {
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
            type Value = GetNodeLimitsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetNodeLimitsRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetNodeLimitsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetNodeLimitsRequest {
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetNodeLimitsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetParticipationRequest {
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
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetParticipationRequest", len)?;
        if let Some(v) = self.epoch.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetParticipationRequest {
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
            type Value = GetParticipationRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetParticipationRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetParticipationRequest, V::Error>
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
                Ok(GetParticipationRequest {
                    epoch: epoch__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetParticipationRequest", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for GetStakeTableRequest {
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
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetStakeTableRequest", len)?;
        if let Some(v) = self.epoch.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetStakeTableRequest {
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
            type Value = GetStakeTableRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetStakeTableRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetStakeTableRequest, V::Error>
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
                Ok(GetStakeTableRequest {
                    epoch: epoch__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetStakeTableRequest", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for GetValidatorsRequest {
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
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetValidatorsRequest", len)?;
        if self.epoch != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&self.epoch).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetValidatorsRequest {
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
            type Value = GetValidatorsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetValidatorsRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetValidatorsRequest, V::Error>
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
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(GetValidatorsRequest {
                    epoch: epoch__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetValidatorsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetVidShareRequest {
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
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.GetVidShareRequest", len)?;
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
impl<'de> serde::Deserialize<'de> for GetVidShareRequest {
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
            type Value = GetVidShareRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.GetVidShareRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetVidShareRequest, V::Error>
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
                Ok(GetVidShareRequest {
                    height: height__,
                    hash: hash__,
                    payload_hash: payload_hash__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.GetVidShareRequest", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for HeaderWindowResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.window.is_empty() {
            len += 1;
        }
        if self.prev.is_some() {
            len += 1;
        }
        if self.next.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.HeaderWindowResponse", len)?;
        if !self.window.is_empty() {
            struct_ser.serialize_field("window", &self.window)?;
        }
        if let Some(v) = self.prev.as_ref() {
            struct_ser.serialize_field("prev", v)?;
        }
        if let Some(v) = self.next.as_ref() {
            struct_ser.serialize_field("next", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HeaderWindowResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "window",
            "prev",
            "next",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Window,
            Prev,
            Next,
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
                            "window" => Ok(GeneratedField::Window),
                            "prev" => Ok(GeneratedField::Prev),
                            "next" => Ok(GeneratedField::Next),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HeaderWindowResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.HeaderWindowResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<HeaderWindowResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut window__ = None;
                let mut prev__ = None;
                let mut next__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Window => {
                            if window__.is_some() {
                                return Err(serde::de::Error::duplicate_field("window"));
                            }
                            window__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Prev => {
                            if prev__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prev"));
                            }
                            prev__ = map_.next_value()?;
                        }
                        GeneratedField::Next => {
                            if next__.is_some() {
                                return Err(serde::de::Error::duplicate_field("next"));
                            }
                            next__ = map_.next_value()?;
                        }
                    }
                }
                Ok(HeaderWindowResponse {
                    window: window__.unwrap_or_default(),
                    prev: prev__,
                    next: next__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.HeaderWindowResponse", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for NodeBlockHeightResponse {
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
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.NodeBlockHeightResponse", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NodeBlockHeightResponse {
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
            type Value = NodeBlockHeightResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.NodeBlockHeightResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NodeBlockHeightResponse, V::Error>
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
                Ok(NodeBlockHeightResponse {
                    height: height__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.NodeBlockHeightResponse", FIELDS, GeneratedVisitor)
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
        if self.icon_14x14_1x.is_some() {
            len += 1;
        }
        if self.icon_14x14_2x.is_some() {
            len += 1;
        }
        if self.icon_14x14_3x.is_some() {
            len += 1;
        }
        if self.icon_24x24_1x.is_some() {
            len += 1;
        }
        if self.icon_24x24_2x.is_some() {
            len += 1;
        }
        if self.icon_24x24_3x.is_some() {
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
        if let Some(v) = self.icon_14x14_1x.as_ref() {
            struct_ser.serialize_field("icon14x141x", v)?;
        }
        if let Some(v) = self.icon_14x14_2x.as_ref() {
            struct_ser.serialize_field("icon14x142x", v)?;
        }
        if let Some(v) = self.icon_14x14_3x.as_ref() {
            struct_ser.serialize_field("icon14x143x", v)?;
        }
        if let Some(v) = self.icon_24x24_1x.as_ref() {
            struct_ser.serialize_field("icon24x241x", v)?;
        }
        if let Some(v) = self.icon_24x24_2x.as_ref() {
            struct_ser.serialize_field("icon24x242x", v)?;
        }
        if let Some(v) = self.icon_24x24_3x.as_ref() {
            struct_ser.serialize_field("icon24x243x", v)?;
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
            "icon_14x14_1x",
            "icon14x141x",
            "icon_14x14_2x",
            "icon14x142x",
            "icon_14x14_3x",
            "icon14x143x",
            "icon_24x24_1x",
            "icon24x241x",
            "icon_24x24_2x",
            "icon24x242x",
            "icon_24x24_3x",
            "icon24x243x",
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
            Icon14x141x,
            Icon14x142x,
            Icon14x143x,
            Icon24x241x,
            Icon24x242x,
            Icon24x243x,
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
                            "icon14x141x" | "icon_14x14_1x" => Ok(GeneratedField::Icon14x141x),
                            "icon14x142x" | "icon_14x14_2x" => Ok(GeneratedField::Icon14x142x),
                            "icon14x143x" | "icon_14x14_3x" => Ok(GeneratedField::Icon14x143x),
                            "icon24x241x" | "icon_24x24_1x" => Ok(GeneratedField::Icon24x241x),
                            "icon24x242x" | "icon_24x24_2x" => Ok(GeneratedField::Icon24x242x),
                            "icon24x243x" | "icon_24x24_3x" => Ok(GeneratedField::Icon24x243x),
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
                let mut icon_14x14_1x__ = None;
                let mut icon_14x14_2x__ = None;
                let mut icon_14x14_3x__ = None;
                let mut icon_24x24_1x__ = None;
                let mut icon_24x24_2x__ = None;
                let mut icon_24x24_3x__ = None;
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
                        GeneratedField::Icon14x141x => {
                            if icon_14x14_1x__.is_some() {
                                return Err(serde::de::Error::duplicate_field("icon14x141x"));
                            }
                            icon_14x14_1x__ = map_.next_value()?;
                        }
                        GeneratedField::Icon14x142x => {
                            if icon_14x14_2x__.is_some() {
                                return Err(serde::de::Error::duplicate_field("icon14x142x"));
                            }
                            icon_14x14_2x__ = map_.next_value()?;
                        }
                        GeneratedField::Icon14x143x => {
                            if icon_14x14_3x__.is_some() {
                                return Err(serde::de::Error::duplicate_field("icon14x143x"));
                            }
                            icon_14x14_3x__ = map_.next_value()?;
                        }
                        GeneratedField::Icon24x241x => {
                            if icon_24x24_1x__.is_some() {
                                return Err(serde::de::Error::duplicate_field("icon24x241x"));
                            }
                            icon_24x24_1x__ = map_.next_value()?;
                        }
                        GeneratedField::Icon24x242x => {
                            if icon_24x24_2x__.is_some() {
                                return Err(serde::de::Error::duplicate_field("icon24x242x"));
                            }
                            icon_24x24_2x__ = map_.next_value()?;
                        }
                        GeneratedField::Icon24x243x => {
                            if icon_24x24_3x__.is_some() {
                                return Err(serde::de::Error::duplicate_field("icon24x243x"));
                            }
                            icon_24x24_3x__ = map_.next_value()?;
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
                    icon_14x14_1x: icon_14x14_1x__,
                    icon_14x14_2x: icon_14x14_2x__,
                    icon_14x14_3x: icon_14x14_3x__,
                    icon_24x24_1x: icon_24x24_1x__,
                    icon_24x24_2x: icon_24x24_2x__,
                    icon_24x24_3x: icon_24x24_3x__,
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
impl serde::Serialize for NodeLimitsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.window_limit != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.NodeLimitsResponse", len)?;
        if self.window_limit != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("windowLimit", ToString::to_string(&self.window_limit).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NodeLimitsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "window_limit",
            "windowLimit",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            WindowLimit,
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
                            "windowLimit" | "window_limit" => Ok(GeneratedField::WindowLimit),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NodeLimitsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.NodeLimitsResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NodeLimitsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut window_limit__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::WindowLimit => {
                            if window_limit__.is_some() {
                                return Err(serde::de::Error::duplicate_field("windowLimit"));
                            }
                            window_limit__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(NodeLimitsResponse {
                    window_limit: window_limit__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.NodeLimitsResponse", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for ParticipationEntry {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.key.is_some() {
            len += 1;
        }
        if self.participation != 0. {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.ParticipationEntry", len)?;
        if let Some(v) = self.key.as_ref() {
            struct_ser.serialize_field("key", v)?;
        }
        if self.participation != 0. {
            struct_ser.serialize_field("participation", &self.participation)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ParticipationEntry {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
            "participation",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
            Participation,
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
                            "participation" => Ok(GeneratedField::Participation),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ParticipationEntry;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.ParticipationEntry")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ParticipationEntry, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                let mut participation__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = map_.next_value()?;
                        }
                        GeneratedField::Participation => {
                            if participation__.is_some() {
                                return Err(serde::de::Error::duplicate_field("participation"));
                            }
                            participation__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(ParticipationEntry {
                    key: key__,
                    participation: participation__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.ParticipationEntry", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ParticipationResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.participation.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.ParticipationResponse", len)?;
        if !self.participation.is_empty() {
            struct_ser.serialize_field("participation", &self.participation)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ParticipationResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "participation",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Participation,
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
                            "participation" => Ok(GeneratedField::Participation),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ParticipationResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.ParticipationResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ParticipationResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut participation__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Participation => {
                            if participation__.is_some() {
                                return Err(serde::de::Error::duplicate_field("participation"));
                            }
                            participation__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ParticipationResponse {
                    participation: participation__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.ParticipationResponse", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for PeerConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.stake_table_entry.is_some() {
            len += 1;
        }
        if self.state_ver_key.is_some() {
            len += 1;
        }
        if self.connect_info.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.PeerConfig", len)?;
        if let Some(v) = self.stake_table_entry.as_ref() {
            struct_ser.serialize_field("stakeTableEntry", v)?;
        }
        if let Some(v) = self.state_ver_key.as_ref() {
            struct_ser.serialize_field("stateVerKey", v)?;
        }
        if let Some(v) = self.connect_info.as_ref() {
            struct_ser.serialize_field("connectInfo", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PeerConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "stake_table_entry",
            "stakeTableEntry",
            "state_ver_key",
            "stateVerKey",
            "connect_info",
            "connectInfo",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StakeTableEntry,
            StateVerKey,
            ConnectInfo,
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
                            "stakeTableEntry" | "stake_table_entry" => Ok(GeneratedField::StakeTableEntry),
                            "stateVerKey" | "state_ver_key" => Ok(GeneratedField::StateVerKey),
                            "connectInfo" | "connect_info" => Ok(GeneratedField::ConnectInfo),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PeerConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.PeerConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PeerConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut stake_table_entry__ = None;
                let mut state_ver_key__ = None;
                let mut connect_info__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::StakeTableEntry => {
                            if stake_table_entry__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stakeTableEntry"));
                            }
                            stake_table_entry__ = map_.next_value()?;
                        }
                        GeneratedField::StateVerKey => {
                            if state_ver_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateVerKey"));
                            }
                            state_ver_key__ = map_.next_value()?;
                        }
                        GeneratedField::ConnectInfo => {
                            if connect_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("connectInfo"));
                            }
                            connect_info__ = map_.next_value()?;
                        }
                    }
                }
                Ok(PeerConfig {
                    stake_table_entry: stake_table_entry__,
                    state_ver_key: state_ver_key__,
                    connect_info: connect_info__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.PeerConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PeerConnectInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.p2p_addr.is_empty() {
            len += 1;
        }
        if !self.x25519_key.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.PeerConnectInfo", len)?;
        if !self.p2p_addr.is_empty() {
            struct_ser.serialize_field("p2pAddr", &self.p2p_addr)?;
        }
        if !self.x25519_key.is_empty() {
            struct_ser.serialize_field("x25519Key", &self.x25519_key)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PeerConnectInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "p2p_addr",
            "p2pAddr",
            "x25519_key",
            "x25519Key",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            P2pAddr,
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
                            "p2pAddr" | "p2p_addr" => Ok(GeneratedField::P2pAddr),
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
            type Value = PeerConnectInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.PeerConnectInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PeerConnectInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut p2p_addr__ = None;
                let mut x25519_key__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::P2pAddr => {
                            if p2p_addr__.is_some() {
                                return Err(serde::de::Error::duplicate_field("p2pAddr"));
                            }
                            p2p_addr__ = Some(map_.next_value()?);
                        }
                        GeneratedField::X25519Key => {
                            if x25519_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("x25519Key"));
                            }
                            x25519_key__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PeerConnectInfo {
                    p2p_addr: p2p_addr__.unwrap_or_default(),
                    x25519_key: x25519_key__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.PeerConnectInfo", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for ShardRange {
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
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.ShardRange", len)?;
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
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ShardRange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start",
            "end",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Start,
            End,
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
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ShardRange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.ShardRange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ShardRange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start__ = None;
                let mut end__ = None;
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
                    }
                }
                Ok(ShardRange {
                    start: start__.unwrap_or_default(),
                    end: end__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.ShardRange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StakeTableEntry {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.stake_key.is_some() {
            len += 1;
        }
        if !self.stake_amount.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.StakeTableEntry", len)?;
        if let Some(v) = self.stake_key.as_ref() {
            struct_ser.serialize_field("stakeKey", v)?;
        }
        if !self.stake_amount.is_empty() {
            struct_ser.serialize_field("stakeAmount", &self.stake_amount)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StakeTableEntry {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "stake_key",
            "stakeKey",
            "stake_amount",
            "stakeAmount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StakeKey,
            StakeAmount,
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
                            "stakeKey" | "stake_key" => Ok(GeneratedField::StakeKey),
                            "stakeAmount" | "stake_amount" => Ok(GeneratedField::StakeAmount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StakeTableEntry;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.StakeTableEntry")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StakeTableEntry, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut stake_key__ = None;
                let mut stake_amount__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::StakeKey => {
                            if stake_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stakeKey"));
                            }
                            stake_key__ = map_.next_value()?;
                        }
                        GeneratedField::StakeAmount => {
                            if stake_amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stakeAmount"));
                            }
                            stake_amount__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StakeTableEntry {
                    stake_key: stake_key__,
                    stake_amount: stake_amount__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.StakeTableEntry", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StakeTableResponse {
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
        if !self.stake_table.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.StakeTableResponse", len)?;
        if let Some(v) = self.epoch.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("epoch", ToString::to_string(&v).as_str())?;
        }
        if !self.stake_table.is_empty() {
            struct_ser.serialize_field("stakeTable", &self.stake_table)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StakeTableResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "epoch",
            "stake_table",
            "stakeTable",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Epoch,
            StakeTable,
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
                            "stakeTable" | "stake_table" => Ok(GeneratedField::StakeTable),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StakeTableResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.StakeTableResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StakeTableResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut epoch__ = None;
                let mut stake_table__ = None;
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
                        GeneratedField::StakeTable => {
                            if stake_table__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stakeTable"));
                            }
                            stake_table__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StakeTableResponse {
                    epoch: epoch__,
                    stake_table: stake_table__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.StakeTableResponse", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for Validator {
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
        if self.stake_table_key.is_some() {
            len += 1;
        }
        if self.state_ver_key.is_some() {
            len += 1;
        }
        if !self.stake.is_empty() {
            len += 1;
        }
        if self.commission != 0 {
            len += 1;
        }
        if !self.delegators.is_empty() {
            len += 1;
        }
        if self.authenticated {
            len += 1;
        }
        if self.x25519_key.is_some() {
            len += 1;
        }
        if self.p2p_addr.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.Validator", len)?;
        if !self.account.is_empty() {
            struct_ser.serialize_field("account", &self.account)?;
        }
        if let Some(v) = self.stake_table_key.as_ref() {
            struct_ser.serialize_field("stakeTableKey", v)?;
        }
        if let Some(v) = self.state_ver_key.as_ref() {
            struct_ser.serialize_field("stateVerKey", v)?;
        }
        if !self.stake.is_empty() {
            struct_ser.serialize_field("stake", &self.stake)?;
        }
        if self.commission != 0 {
            struct_ser.serialize_field("commission", &self.commission)?;
        }
        if !self.delegators.is_empty() {
            struct_ser.serialize_field("delegators", &self.delegators)?;
        }
        if self.authenticated {
            struct_ser.serialize_field("authenticated", &self.authenticated)?;
        }
        if let Some(v) = self.x25519_key.as_ref() {
            struct_ser.serialize_field("x25519Key", v)?;
        }
        if let Some(v) = self.p2p_addr.as_ref() {
            struct_ser.serialize_field("p2pAddr", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Validator {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "account",
            "stake_table_key",
            "stakeTableKey",
            "state_ver_key",
            "stateVerKey",
            "stake",
            "commission",
            "delegators",
            "authenticated",
            "x25519_key",
            "x25519Key",
            "p2p_addr",
            "p2pAddr",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Account,
            StakeTableKey,
            StateVerKey,
            Stake,
            Commission,
            Delegators,
            Authenticated,
            X25519Key,
            P2pAddr,
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
                            "stakeTableKey" | "stake_table_key" => Ok(GeneratedField::StakeTableKey),
                            "stateVerKey" | "state_ver_key" => Ok(GeneratedField::StateVerKey),
                            "stake" => Ok(GeneratedField::Stake),
                            "commission" => Ok(GeneratedField::Commission),
                            "delegators" => Ok(GeneratedField::Delegators),
                            "authenticated" => Ok(GeneratedField::Authenticated),
                            "x25519Key" | "x25519_key" => Ok(GeneratedField::X25519Key),
                            "p2pAddr" | "p2p_addr" => Ok(GeneratedField::P2pAddr),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Validator;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.Validator")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Validator, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut account__ = None;
                let mut stake_table_key__ = None;
                let mut state_ver_key__ = None;
                let mut stake__ = None;
                let mut commission__ = None;
                let mut delegators__ = None;
                let mut authenticated__ = None;
                let mut x25519_key__ = None;
                let mut p2p_addr__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Account => {
                            if account__.is_some() {
                                return Err(serde::de::Error::duplicate_field("account"));
                            }
                            account__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StakeTableKey => {
                            if stake_table_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stakeTableKey"));
                            }
                            stake_table_key__ = map_.next_value()?;
                        }
                        GeneratedField::StateVerKey => {
                            if state_ver_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateVerKey"));
                            }
                            state_ver_key__ = map_.next_value()?;
                        }
                        GeneratedField::Stake => {
                            if stake__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stake"));
                            }
                            stake__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Commission => {
                            if commission__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commission"));
                            }
                            commission__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Delegators => {
                            if delegators__.is_some() {
                                return Err(serde::de::Error::duplicate_field("delegators"));
                            }
                            delegators__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Authenticated => {
                            if authenticated__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authenticated"));
                            }
                            authenticated__ = Some(map_.next_value()?);
                        }
                        GeneratedField::X25519Key => {
                            if x25519_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("x25519Key"));
                            }
                            x25519_key__ = map_.next_value()?;
                        }
                        GeneratedField::P2pAddr => {
                            if p2p_addr__.is_some() {
                                return Err(serde::de::Error::duplicate_field("p2pAddr"));
                            }
                            p2p_addr__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Validator {
                    account: account__.unwrap_or_default(),
                    stake_table_key: stake_table_key__,
                    state_ver_key: state_ver_key__,
                    stake: stake__.unwrap_or_default(),
                    commission: commission__.unwrap_or_default(),
                    delegators: delegators__.unwrap_or_default(),
                    authenticated: authenticated__.unwrap_or_default(),
                    x25519_key: x25519_key__,
                    p2p_addr: p2p_addr__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.Validator", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ValidatorsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.validators.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.ValidatorsResponse", len)?;
        if !self.validators.is_empty() {
            struct_ser.serialize_field("validators", &self.validators)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ValidatorsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "validators",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Validators,
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
                            "validators" => Ok(GeneratedField::Validators),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ValidatorsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.ValidatorsResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ValidatorsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut validators__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Validators => {
                            if validators__.is_some() {
                                return Err(serde::de::Error::duplicate_field("validators"));
                            }
                            validators__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ValidatorsResponse {
                    validators: validators__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.ValidatorsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for VidShareResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.share.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("espresso.api.v2.VidShareResponse", len)?;
        if let Some(v) = self.share.as_ref() {
            match v {
                vid_share_response::Share::V0(v) => {
                    struct_ser.serialize_field("v0", v)?;
                }
                vid_share_response::Share::V1(v) => {
                    struct_ser.serialize_field("v1", v)?;
                }
                vid_share_response::Share::V2(v) => {
                    struct_ser.serialize_field("v2", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for VidShareResponse {
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
            type Value = VidShareResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct espresso.api.v2.VidShareResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<VidShareResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut share__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::V0 => {
                            if share__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v0"));
                            }
                            share__ = map_.next_value::<::std::option::Option<_>>()?.map(vid_share_response::Share::V0)
;
                        }
                        GeneratedField::V1 => {
                            if share__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v1"));
                            }
                            share__ = map_.next_value::<::std::option::Option<_>>()?.map(vid_share_response::Share::V1)
;
                        }
                        GeneratedField::V2 => {
                            if share__.is_some() {
                                return Err(serde::de::Error::duplicate_field("v2"));
                            }
                            share__ = map_.next_value::<::std::option::Option<_>>()?.map(vid_share_response::Share::V2)
;
                        }
                    }
                }
                Ok(VidShareResponse {
                    share: share__,
                })
            }
        }
        deserializer.deserialize_struct("espresso.api.v2.VidShareResponse", FIELDS, GeneratedVisitor)
    }
}
