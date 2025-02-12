(function() {
    var implementors = Object.fromEntries([["espresso_types",[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"espresso_types/v0/v0_1/l1/struct.L1UpdateTask.html\" title=\"struct espresso_types::v0::v0_1::l1::L1UpdateTask\">L1UpdateTask</a>"]]],["hotshot_query_service",[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hotshot_query_service/data_source/notifier/struct.ReceiveHandle.html\" title=\"struct hotshot_query_service::data_source::notifier::ReceiveHandle\">ReceiveHandle</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hotshot_query_service/data_source/storage/sql/testing/struct.TmpDb.html\" title=\"struct hotshot_query_service::data_source::storage::sql::testing::TmpDb\">TmpDb</a>"],["impl&lt;D: <a class=\"trait\" href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html\" title=\"trait hotshot_query_service::testing::consensus::DataSourceLifeCycle\">DataSourceLifeCycle</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hotshot_query_service/testing/consensus/struct.MockNetwork.html\" title=\"struct hotshot_query_service::testing::consensus::MockNetwork\">MockNetwork</a>&lt;D&gt;"],["impl&lt;Mode&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hotshot_query_service/data_source/storage/sql/transaction/struct.TransactionMetricsGuard.html\" title=\"struct hotshot_query_service::data_source::storage::sql::transaction::TransactionMetricsGuard\">TransactionMetricsGuard</a>&lt;Mode&gt;"],["impl&lt;T: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/fs/trait.Revert.html\" title=\"trait hotshot_query_service::data_source::storage::fs::Revert\">Revert</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hotshot_query_service/data_source/storage/fs/struct.Transaction.html\" title=\"struct hotshot_query_service::data_source::storage::fs::Transaction\">Transaction</a>&lt;T&gt;"],["impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'static&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hotshot_query_service/task/struct.Task.html\" title=\"struct hotshot_query_service::task::Task\">Task</a>&lt;T&gt;"]]],["hotshot_task_impls",[["impl&lt;TYPES: NodeType, I: NodeImplementation&lt;TYPES&gt;&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hotshot_task_impls/request/struct.NetworkRequestState.html\" title=\"struct hotshot_task_impls::request::NetworkRequestState\">NetworkRequestState</a>&lt;TYPES, I&gt;"]]],["hotshot_types",[["impl&lt;TYPES: <a class=\"trait\" href=\"hotshot_types/traits/node_implementation/trait.NodeType.html\" title=\"trait hotshot_types::traits::node_implementation::NodeType\">NodeType</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hotshot_types/consensus/struct.ConsensusReadLockGuard.html\" title=\"struct hotshot_types::consensus::ConsensusReadLockGuard\">ConsensusReadLockGuard</a>&lt;'_, TYPES&gt;"],["impl&lt;TYPES: <a class=\"trait\" href=\"hotshot_types/traits/node_implementation/trait.NodeType.html\" title=\"trait hotshot_types::traits::node_implementation::NodeType\">NodeType</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hotshot_types/consensus/struct.ConsensusUpgradableReadLockGuard.html\" title=\"struct hotshot_types::consensus::ConsensusUpgradableReadLockGuard\">ConsensusUpgradableReadLockGuard</a>&lt;'_, TYPES&gt;"],["impl&lt;TYPES: <a class=\"trait\" href=\"hotshot_types/traits/node_implementation/trait.NodeType.html\" title=\"trait hotshot_types::traits::node_implementation::NodeType\">NodeType</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hotshot_types/consensus/struct.ConsensusWriteLockGuard.html\" title=\"struct hotshot_types::consensus::ConsensusWriteLockGuard\">ConsensusWriteLockGuard</a>&lt;'_, TYPES&gt;"]]],["node_metrics",[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/api/node_validator/v0/cdn/struct.BroadcastRollCallTask.html\" title=\"struct node_metrics::api::node_validator::v0::cdn::BroadcastRollCallTask\">BroadcastRollCallTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/api/node_validator/v0/cdn/struct.CdnReceiveMessagesTask.html\" title=\"struct node_metrics::api::node_validator::v0::cdn::CdnReceiveMessagesTask\">CdnReceiveMessagesTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/api/node_validator/v0/create_node_validator_api/struct.HotShotEventProcessingTask.html\" title=\"struct node_metrics::api::node_validator::v0::create_node_validator_api::HotShotEventProcessingTask\">HotShotEventProcessingTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/api/node_validator/v0/create_node_validator_api/struct.ProcessExternalMessageHandlingTask.html\" title=\"struct node_metrics::api::node_validator::v0::create_node_validator_api::ProcessExternalMessageHandlingTask\">ProcessExternalMessageHandlingTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/api/node_validator/v0/struct.ProcessNodeIdentityUrlStreamTask.html\" title=\"struct node_metrics::api::node_validator::v0::ProcessNodeIdentityUrlStreamTask\">ProcessNodeIdentityUrlStreamTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/api/node_validator/v0/struct.ProcessProduceLeafStreamTask.html\" title=\"struct node_metrics::api::node_validator::v0::ProcessProduceLeafStreamTask\">ProcessProduceLeafStreamTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/service/client_state/struct.InternalClientMessageProcessingTask.html\" title=\"struct node_metrics::service::client_state::InternalClientMessageProcessingTask\">InternalClientMessageProcessingTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/service/client_state/struct.ProcessDistributeBlockDetailHandlingTask.html\" title=\"struct node_metrics::service::client_state::ProcessDistributeBlockDetailHandlingTask\">ProcessDistributeBlockDetailHandlingTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/service/client_state/struct.ProcessDistributeNodeIdentityHandlingTask.html\" title=\"struct node_metrics::service::client_state::ProcessDistributeNodeIdentityHandlingTask\">ProcessDistributeNodeIdentityHandlingTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/service/client_state/struct.ProcessDistributeVotersHandlingTask.html\" title=\"struct node_metrics::service::client_state::ProcessDistributeVotersHandlingTask\">ProcessDistributeVotersHandlingTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/service/data_state/struct.ProcessLeafStreamTask.html\" title=\"struct node_metrics::service::data_state::ProcessLeafStreamTask\">ProcessLeafStreamTask</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"node_metrics/service/data_state/struct.ProcessNodeIdentityStreamTask.html\" title=\"struct node_metrics::service::data_state::ProcessNodeIdentityStreamTask\">ProcessNodeIdentityStreamTask</a>"]]],["request_response",[["impl&lt;R: <a class=\"trait\" href=\"request_response/request/trait.Request.html\" title=\"trait request_response::request::Request\">Request</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"request_response/struct.ActiveRequestInner.html\" title=\"struct request_response::ActiveRequestInner\">ActiveRequestInner</a>&lt;R&gt;"]]],["sequencer",[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"sequencer/context/struct.TaskList.html\" title=\"struct sequencer::context::TaskList\">TaskList</a>"],["impl&lt;N: <a class=\"trait\" href=\"hotshot_types/traits/network/trait.ConnectedNetwork.html\" title=\"trait hotshot_types::traits::network::ConnectedNetwork\">ConnectedNetwork</a>&lt;<a class=\"type\" href=\"espresso_types/v0/type.PubKey.html\" title=\"type espresso_types::v0::PubKey\">PubKey</a>&gt;, P: <a class=\"trait\" href=\"espresso_types/v0/traits/trait.SequencerPersistence.html\" title=\"trait espresso_types::v0::traits::SequencerPersistence\">SequencerPersistence</a>, V: <a class=\"trait\" href=\"hotshot_types/traits/node_implementation/trait.Versions.html\" title=\"trait hotshot_types::traits::node_implementation::Versions\">Versions</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"sequencer/context/struct.SequencerContext.html\" title=\"struct sequencer::context::SequencerContext\">SequencerContext</a>&lt;N, P, V&gt;"]]],["sequencer_utils",[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"sequencer_utils/struct.Anvil.html\" title=\"struct sequencer_utils::Anvil\">Anvil</a>"]]]]);
    if (window.register_implementors) {
        window.register_implementors(implementors);
    } else {
        window.pending_implementors = implementors;
    }
})()
//{"start":57,"fragment_lengths":[328,2681,429,1655,4689,482,1268,286]}