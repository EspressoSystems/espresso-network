use serde::{Deserialize, Serialize};

use super::client_id::ClientId;

/// [ClientMessage] represents the messages that the client can send to the
/// server for a request.
///
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    SubscribeLatestBlock,
    SubscribeNodeIdentity,
    SubscribeVoters,

    RequestBlocksSnapshot,
    RequestNodeIdentitySnapshot,
    RequestHistogramSnapshot,
    RequestVotersSnapshot,
}

/// InternalClientMessage represents the message requests that the client can
/// send to the server.  These messages are request that the client can send
/// in order for the server to send back responses that correspond to the
/// request.
#[derive(Debug)]
pub enum InternalClientMessage<K> {
    Connected(K),
    Disconnected(ClientId),

    Request(ClientId, ClientMessage),
}

impl ClientMessage {
    /// [to_internal_with_client_id] converts the [ClientMessage] into an
    /// [InternalClientMessage] with the given [ClientId].
    pub fn to_internal_with_client_id<K>(&self, client_id: ClientId) -> InternalClientMessage<K> {
        InternalClientMessage::Request(client_id, *self)
    }
}

#[cfg(test)]
mod tests {
    use std::iter::zip;

    use futures::channel::mpsc::Sender;

    use super::{InternalClientMessage, *};
    use crate::service::server_message::ServerMessage;

    impl<K> PartialEq for InternalClientMessage<K> {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                // We don't care about the [Sender] here, as it is unable to be
                // compared.
                (Self::Connected(_), Self::Connected(_)) => true,
                (Self::Disconnected(lhs), Self::Disconnected(rhs)) => lhs == rhs,
                (
                    Self::Request(lhs_client_id, lhs_message),
                    Self::Request(rhs_client_id, rhs_message),
                ) => lhs_client_id == rhs_client_id && lhs_message == rhs_message,
                _ => false,
            }
        }
    }

    #[test]
    fn test_client_message_partial_eq() {
        let messages = [
            ClientMessage::SubscribeLatestBlock,
            ClientMessage::SubscribeNodeIdentity,
            ClientMessage::SubscribeVoters,
            ClientMessage::RequestBlocksSnapshot,
            ClientMessage::RequestNodeIdentitySnapshot,
            ClientMessage::RequestHistogramSnapshot,
        ];

        for (l, r) in zip(messages.iter(), messages.iter()) {
            assert_eq!(l, r);
        }

        for i in 1..messages.len() {
            for (l, r) in zip(
                messages.iter(),
                messages.iter().skip(i).chain(messages.iter().take(i)),
            ) {
                assert_ne!(l, r);
            }
        }
    }

    #[test]
    fn test_client_message_debug() {
        let messages = [
            ClientMessage::SubscribeLatestBlock,
            ClientMessage::SubscribeNodeIdentity,
            ClientMessage::SubscribeVoters,
            ClientMessage::RequestBlocksSnapshot,
            ClientMessage::RequestNodeIdentitySnapshot,
            ClientMessage::RequestHistogramSnapshot,
        ];

        for message in messages.iter() {
            assert_eq!(format!("{:?}", message), format!("{:?}", message));
        }
    }

    #[test]
    #[cfg(feature = "testing")]
    fn test_client_message_serialize() {
        use serde_json;

        let messages = [
            ClientMessage::SubscribeLatestBlock,
            ClientMessage::SubscribeNodeIdentity,
            ClientMessage::SubscribeVoters,
            ClientMessage::RequestBlocksSnapshot,
            ClientMessage::RequestNodeIdentitySnapshot,
            ClientMessage::RequestHistogramSnapshot,
        ];

        for message in messages.iter() {
            let serialized = serde_json::to_string(message).unwrap();
            let deserialized: ClientMessage = serde_json::from_str(&serialized).unwrap();
            assert_eq!(*message, deserialized);
        }
    }

    #[test]
    fn test_client_message_to_internal_with_client_id() {
        let messages = [
            ClientMessage::SubscribeLatestBlock,
            ClientMessage::SubscribeNodeIdentity,
            ClientMessage::SubscribeVoters,
            ClientMessage::RequestBlocksSnapshot,
            ClientMessage::RequestNodeIdentitySnapshot,
            ClientMessage::RequestHistogramSnapshot,
        ];

        for message in messages {
            for i in 0..10 {
                let client_id = ClientId::from_count(i);
                let internal_client_message =
                    message.to_internal_with_client_id::<Sender<ServerMessage>>(client_id);
                match internal_client_message {
                    InternalClientMessage::Request(id, _) => {
                        assert_eq!(id, client_id);
                    },
                    _ => panic!("Unexpected InternalClientMessage"),
                }
            }
        }
    }

    #[test]
    fn test_internal_client_message_partial_eq() {
        let (sender, _) = futures::channel::mpsc::channel::<ServerMessage>(1);
        let messages = [
            InternalClientMessage::Connected(sender),
            InternalClientMessage::Disconnected(ClientId::from_count(1)),
            InternalClientMessage::Request(
                ClientId::from_count(1),
                ClientMessage::SubscribeLatestBlock,
            ),
            InternalClientMessage::Request(
                ClientId::from_count(1),
                ClientMessage::SubscribeNodeIdentity,
            ),
            InternalClientMessage::Request(ClientId::from_count(1), ClientMessage::SubscribeVoters),
            InternalClientMessage::Request(
                ClientId::from_count(1),
                ClientMessage::RequestBlocksSnapshot,
            ),
            InternalClientMessage::Request(
                ClientId::from_count(1),
                ClientMessage::RequestNodeIdentitySnapshot,
            ),
            InternalClientMessage::Request(
                ClientId::from_count(1),
                ClientMessage::RequestHistogramSnapshot,
            ),
        ];

        for (l, r) in zip(messages.iter(), messages.iter()) {
            assert_eq!(l, r);
        }

        for i in 1..messages.len() {
            for (l, r) in zip(
                messages.iter(),
                messages.iter().skip(i).chain(messages.iter().take(i)),
            ) {
                assert_ne!(l, r);
            }
        }

        for j in 2..12 {
            let iter_messages = [
                InternalClientMessage::Disconnected(ClientId::from_count(j)),
                InternalClientMessage::Request(
                    ClientId::from_count(j),
                    ClientMessage::SubscribeLatestBlock,
                ),
                InternalClientMessage::Request(
                    ClientId::from_count(j),
                    ClientMessage::SubscribeNodeIdentity,
                ),
                InternalClientMessage::Request(
                    ClientId::from_count(j),
                    ClientMessage::SubscribeVoters,
                ),
                InternalClientMessage::Request(
                    ClientId::from_count(j),
                    ClientMessage::RequestBlocksSnapshot,
                ),
                InternalClientMessage::Request(
                    ClientId::from_count(j),
                    ClientMessage::RequestNodeIdentitySnapshot,
                ),
                InternalClientMessage::Request(
                    ClientId::from_count(j),
                    ClientMessage::RequestHistogramSnapshot,
                ),
            ];

            // We skip the first message, as we don't want to include the
            // Connected message.
            for (l, r) in zip(messages.iter().skip(1), iter_messages.iter()) {
                assert_ne!(l, r);
            }
        }
    }
}
