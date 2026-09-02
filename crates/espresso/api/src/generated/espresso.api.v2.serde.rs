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
