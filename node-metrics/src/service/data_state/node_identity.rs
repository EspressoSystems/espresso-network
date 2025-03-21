use hotshot_types::signature_key::BLSPubKey;
use serde::{Deserialize, Serialize};
use surf_disco::Url;

use super::LocationDetails;

/// [NodeIdentity] represents the identity of the node that is participating
/// in the network.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub(crate) public_key: BLSPubKey,
    pub(crate) name: Option<String>,
    pub(crate) public_url: Option<Url>,
    pub(crate) company: Option<String>,
    pub(crate) company_website: Option<Url>,
    pub(crate) location: Option<LocationDetails>,
    pub(crate) operating_system: Option<String>,

    /// note_type is meant to reflect the type of the node that is being
    /// run.  The simplest representation of this value is the specific
    /// binary program that is running for the node. In the case of the
    /// Espresso sequencer, this is expected to be the value:
    /// "espresso-sequencer <version>".
    ///
    /// Other implementations may use their own values instead.
    pub(crate) node_type: Option<String>,

    /// network_type is meant to represent the type of network that the node is
    /// connected to.  The sample specification has the following values
    /// suggested:
    /// - residential
    /// - hosting
    ///
    /// It is preferred to have some present values we would like for these
    /// to be, but for flexibility it is set to be a generic String.
    /// Proposed values:
    /// - Residential
    /// - AWS
    /// - Azure
    /// - GCP
    ///
    /// These could also potentially include the availability zone for the
    /// hosted networks:
    /// - AWS (us-east-1)
    ///
    /// This could potentially even be:
    /// - AWS (us-east-1a)
    pub(crate) network_type: Option<String>,
}

impl NodeIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        public_key: BLSPubKey,
        name: Option<String>,
        public_url: Option<Url>,
        company: Option<String>,
        company_website: Option<Url>,
        location: Option<LocationDetails>,
        operating_system: Option<String>,
        node_type: Option<String>,
        network_type: Option<String>,
    ) -> Self {
        Self {
            public_key,
            name,
            public_url,
            company,
            company_website,
            location,
            operating_system,
            node_type,
            network_type,
        }
    }

    pub fn public_key(&self) -> &BLSPubKey {
        &self.public_key
    }

    pub fn name(&self) -> &Option<String> {
        &self.name
    }

    pub fn public_url(&self) -> &Option<Url> {
        &self.public_url
    }

    pub fn company(&self) -> &Option<String> {
        &self.company
    }

    pub fn company_website(&self) -> &Option<Url> {
        &self.company_website
    }

    pub fn location(&self) -> Option<&LocationDetails> {
        self.location.as_ref()
    }

    pub fn operating_system(&self) -> &Option<String> {
        &self.operating_system
    }

    pub fn node_type(&self) -> &Option<String> {
        &self.node_type
    }

    pub fn network_type(&self) -> &Option<String> {
        &self.network_type
    }

    pub fn from_public_key(public_key: BLSPubKey) -> Self {
        Self {
            public_key,
            name: None,
            public_url: None,
            company: None,
            company_website: None,
            location: None,
            operating_system: None,
            node_type: None,
            network_type: None,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use hotshot_types::{signature_key::BLSPubKey, traits::signature_key::SignatureKey};

    use super::{LocationDetails, NodeIdentity};

    pub fn create_test_node(index: u64) -> NodeIdentity {
        let (pub_key, _) = BLSPubKey::generated_from_seed_indexed([0; 32], index);

        NodeIdentity::new(
            pub_key,
            Some("a".to_string()),
            Some("https://espressosys.com/".parse().unwrap()),
            Some("company".to_string()),
            Some("https://example.com/".parse().unwrap()),
            Some(LocationDetails::new(
                Some((0.0, 0.0)),
                Some("US".to_string()),
            )),
            Some("Windows 11".to_string()),
            Some("espresso".to_string()),
            Some("residential".to_string()),
        )
    }

    #[test]
    fn test_node_identity_eq() {
        let node_identity_1 = create_test_node(1);
        let node_identity_2 = create_test_node(1);
        let node_identity_3 = create_test_node(2);

        assert_eq!(node_identity_1, node_identity_2);
        assert_ne!(node_identity_1, node_identity_3);
        assert_ne!(node_identity_2, node_identity_3);
    }

    #[test]
    fn test_node_identity_eq_clone() {
        let node_identity_1 = create_test_node(1);
        let node_identity_2 = node_identity_1.clone();

        assert_eq!(node_identity_1, node_identity_2);
    }

    #[test]
    #[cfg(feature = "testing")]
    fn test_node_identity_serialize() {
        use serde_json;

        let node_identity = create_test_node(1);
        let serialized = serde_json::to_string(&node_identity).unwrap();
        let deserialized: NodeIdentity = serde_json::from_str(&serialized).unwrap();

        assert_eq!(node_identity, deserialized);
    }

    #[test]
    fn test_node_identity_public_key() {
        let node_identity = create_test_node(1);
        let public_key = node_identity.public_key();

        assert_eq!(
            public_key,
            &BLSPubKey::generated_from_seed_indexed([0; 32], 1).0
        );
    }

    #[test]
    fn test_node_identity_name() {
        let node_identity = create_test_node(1);
        let name = node_identity.name();

        assert_eq!(name, &Some("a".to_string()));
    }

    #[test]
    fn test_node_identity_public_url() {
        let node_identity = create_test_node(1);
        let public_url = node_identity.public_url();

        assert_eq!(
            public_url,
            &Some("https://espressosys.com/".parse().unwrap()),
        );
    }

    #[test]
    fn test_node_identity_company() {
        let node_identity = create_test_node(1);
        let company = node_identity.company();

        assert_eq!(company, &Some("company".to_string()));
    }

    #[test]
    fn test_node_identity_location() {
        let node_identity = create_test_node(1);
        let location = node_identity.location();

        assert_eq!(
            location,
            Some(&LocationDetails::new(
                Some((0.0, 0.0)),
                Some("US".to_string())
            ))
        );
    }

    #[test]
    fn test_node_identity_operating_system() {
        let node_identity = create_test_node(1);
        let operating_system = node_identity.operating_system();

        assert_eq!(operating_system, &Some("Windows 11".to_string()));
    }

    #[test]
    fn test_node_identity_node_type() {
        let node_identity = create_test_node(1);
        let node_type = node_identity.node_type();

        assert_eq!(node_type, &Some("espresso".to_string()));
    }

    #[test]
    fn test_node_identity_network_type() {
        let node_identity = create_test_node(1);
        let network_type = node_identity.network_type();

        assert_eq!(network_type, &Some("residential".to_string()));
    }
}
