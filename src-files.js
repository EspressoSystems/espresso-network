var srcIndex = new Map(JSON.parse('[["builder",["",[],["lib.rs","non_permissioned.rs"]]],["cdn_broker",["",[],["cdn-broker.rs"]]],["cdn_marshal",["",[],["cdn-marshal.rs"]]],["cdn_whitelist",["",[],["cdn-whitelist.rs"]]],["client",["",[],["lib.rs"]]],["contract_bindings_alloy",["",[],["erc1967proxy.rs","esptoken.rs","feecontract.rs","iplonkverifier.rs","lib.rs","lightclient.rs","lightclientarbitrum.rs","lightclientmock.rs","permissionedstaketable.rs","plonkverifier.rs","plonkverifier2.rs","staketable.rs"]]],["contract_bindings_ethers",["",[],["erc1967_proxy.rs","fee_contract.rs","i_plonk_verifier.rs","lib.rs","light_client.rs","light_client_arbitrum.rs","light_client_mock.rs","permissioned_stake_table.rs","plonk_verifier.rs","plonk_verifier_2.rs","shared_types.rs"]]],["deploy",["",[],["deploy.rs"]]],["dev_cdn",["",[],["dev-cdn.rs"]]],["dev_rollup",["",[],["dev-rollup.rs"]]],["diff_test",["",[],["main.rs"]]],["espresso_bridge",["",[],["espresso-bridge.rs"]]],["espresso_types",["",[["v0",[["impls",[["block",[["full_payload",[],["ns_proof.rs","ns_table.rs","payload.rs"]],["namespace_payload",[],["iter.rs","ns_payload.rs","ns_payload_range.rs","tx_proof.rs","types.rs"]]],["full_payload.rs","mod.rs","namespace_payload.rs","uint_bytes.rs"]]],["auction.rs","chain_config.rs","fee_info.rs","header.rs","instance_state.rs","l1.rs","mod.rs","reward.rs","solver.rs","stake_table.rs","state.rs","transaction.rs"]],["v0_1",[],["block.rs","chain_config.rs","fee_info.rs","header.rs","instance_state.rs","l1.rs","mod.rs","signature.rs","state.rs","transaction.rs"]],["v0_2",[],["mod.rs"]],["v0_3",[],["chain_config.rs","header.rs","mod.rs","stake_table.rs"]],["v0_99",[],["auction.rs","chain_config.rs","fee_info.rs","header.rs","mod.rs","solver.rs"]]],["config.rs","header.rs","mod.rs","traits.rs","utils.rs"]]],["eth_signature_key.rs","lib.rs"]]],["ethers_conv",["",[],["lib.rs"]]],["eval_domain",["",[],["eval_domain.rs"]]],["gen_demo_genesis",["",[],["gen-demo-genesis.rs"]]],["gen_vk_contract",["",[],["main.rs"]]],["hotshot",["",[["tasks",[],["mod.rs","task_state.rs"]],["traits",[["election",[],["dummy_catchup_membership.rs","helpers.rs","mod.rs","randomized_committee.rs","randomized_committee_members.rs","static_committee.rs","static_committee_leader_two_views.rs","two_static_committees.rs"]],["networking",[],["combined_network.rs","libp2p_network.rs","memory_network.rs","push_cdn_network.rs"]]],["networking.rs","node_implementation.rs"]],["types",[],["event.rs","handle.rs"]]],["documentation.rs","helpers.rs","lib.rs","traits.rs","types.rs"]]],["hotshot_builder_api",["",[["v0_1",[],["block_info.rs","builder.rs","data_source.rs","mod.rs","query_data.rs"]],["v0_99",[],["builder.rs","data_source.rs","mod.rs"]]],["api.rs","lib.rs"]]],["hotshot_builder_core",["",[],["builder_state.rs","lib.rs","service.rs"]]],["hotshot_builder_core_refactored",["",[],["block_size_limits.rs","block_store.rs","lib.rs","service.rs"]]],["hotshot_contract_adapter",["",[],["jellyfish.rs","lib.rs","light_client.rs","stake_table.rs"]]],["hotshot_events_service",["",[],["api.rs","events.rs","events_source.rs","lib.rs","test.rs"]]],["hotshot_example_types",["",[],["auction_results_provider_types.rs","block_types.rs","lib.rs","node_types.rs","state_types.rs","storage_types.rs","testable_delay.rs"]]],["hotshot_fakeapi",["",[],["fake_solver.rs","lib.rs"]]],["hotshot_libp2p_networking",["",[["network",[["behaviours",[["dht",[["store",[],["mod.rs","persistent.rs","validated.rs"]]],["bootstrap.rs","mod.rs","record.rs"]]],["direct_message.rs","exponential_backoff.rs","mod.rs"]],["node",[],["config.rs","handle.rs"]]],["cbor.rs","def.rs","mod.rs","node.rs","transport.rs"]]],["lib.rs"]]],["hotshot_macros",["",[],["lib.rs"]]],["hotshot_orchestrator",["",[],["client.rs","lib.rs"]]],["hotshot_query_service",["",[["availability",[],["data_source.rs","fetch.rs","query_data.rs"]],["data_source",[["fetching",[],["block.rs","header.rs","leaf.rs","transaction.rs","vid.rs"]],["storage",[["sql",[["queries",[],["availability.rs","explorer.rs","node.rs","state.rs"]]],["db.rs","migrate.rs","queries.rs","transaction.rs"]]],["fail_storage.rs","fs.rs","ledger_log.rs","pruning.rs","sql.rs"]]],["extension.rs","fetching.rs","fs.rs","metrics.rs","notifier.rs","sql.rs","storage.rs","update.rs"]],["explorer",[],["currency.rs","data_source.rs","errors.rs","monetary_value.rs","query_data.rs","traits.rs"]],["fetching",[["provider",[],["any.rs","query_service.rs","testing.rs"]]],["provider.rs","request.rs"]],["merklized_state",[],["data_source.rs"]],["node",[],["data_source.rs","query_data.rs"]],["status",[],["data_source.rs"]],["testing",[],["consensus.rs","mocks.rs"]]],["api.rs","availability.rs","data_source.rs","error.rs","explorer.rs","fetching.rs","lib.rs","merklized_state.rs","metrics.rs","node.rs","resolvable.rs","status.rs","task.rs","testing.rs","types.rs"]]],["hotshot_stake_table",["",[["mt_based",[],["config.rs","internal.rs"]],["vec_based",[],["config.rs"]]],["config.rs","lib.rs","mt_based.rs","utils.rs","vec_based.rs"]]],["hotshot_state_prover",["",[],["circuit.rs","lib.rs","mock_ledger.rs","service.rs","snark.rs"]]],["hotshot_task",["",[],["dependency.rs","dependency_task.rs","lib.rs","task.rs"]]],["hotshot_task_impls",["",[["consensus",[],["handlers.rs","mod.rs"]],["quorum_proposal",[],["handlers.rs","mod.rs"]],["quorum_proposal_recv",[],["handlers.rs","mod.rs"]],["quorum_vote",[],["handlers.rs","mod.rs"]]],["builder.rs","da.rs","events.rs","harness.rs","helpers.rs","lib.rs","network.rs","request.rs","response.rs","rewind.rs","transactions.rs","upgrade.rs","vid.rs","view_sync.rs","vote_collection.rs"]]],["hotshot_testing",["",[["block_builder",[],["mod.rs","random.rs","simple.rs"]],["byzantine",[],["byzantine_behaviour.rs","mod.rs"]],["predicates",[],["event.rs","mod.rs","upgrade_with_proposal.rs","upgrade_with_vote.rs"]]],["completion_task.rs","consistency_task.rs","helpers.rs","lib.rs","overall_safety_task.rs","script.rs","spinning_task.rs","test_builder.rs","test_launcher.rs","test_runner.rs","test_task.rs","txn_task.rs","view_generator.rs","view_sync_task.rs"]]],["hotshot_types",["",[["data",[],["ns_table.rs","vid_disperse.rs"]],["traits",[],["auction_results_provider.rs","block_contents.rs","consensus_api.rs","election.rs","metrics.rs","network.rs","node_implementation.rs","qc.rs","signature_key.rs","stake_table.rs","states.rs","storage.rs"]],["vid",[],["advz.rs","avidm.rs"]]],["bundle.rs","consensus.rs","constants.rs","data.rs","drb.rs","epoch_membership.rs","error.rs","event.rs","hotshot_config_file.rs","lib.rs","light_client.rs","message.rs","network.rs","qc.rs","request_response.rs","signature_key.rs","simple_certificate.rs","simple_vote.rs","stake_table.rs","traits.rs","upgrade_config.rs","utils.rs","vid.rs","vote.rs"]]],["hotshot_utils",["",[["anytrace",[],["macros.rs"]]],["anytrace.rs","lib.rs"]]],["keygen",["",[],["keygen.rs"]]],["marketplace_builder",["",[],["builder.rs","hooks.rs","lib.rs"]]],["marketplace_builder_core",["",[],["hooks.rs","lib.rs","service.rs"]]],["marketplace_builder_shared",["",[["coordinator",[],["mod.rs","tiered_view_map.rs"]],["testing",[],["consensus.rs","constants.rs","generation.rs","mock.rs","mod.rs","validation.rs"]],["utils",[],["event_service_wrapper.rs","mod.rs","rotating_set.rs"]]],["block.rs","error.rs","lib.rs","state.rs"]]],["marketplace_solver",["",[],["api.rs","database.rs","events.rs","lib.rs","options.rs","state.rs"]]],["nasty_client",["",[],["nasty-client.rs"]]],["node_metrics",["",[["api",[["node_validator",[["v0",[["cdn",[],["mod.rs"]]],["create_node_validator_api.rs","mod.rs"]]],["mod.rs"]]],["mod.rs"]],["service",[["client_id",[],["mod.rs"]],["client_message",[],["mod.rs"]],["client_state",[],["mod.rs"]],["data_state",[],["location_details.rs","mod.rs","node_identity.rs"]],["node_type",[],["mod.rs"]],["server_message",[],["mod.rs"]]],["mod.rs"]]],["lib.rs"]]],["orchestrator",["",[],["orchestrator.rs"]]],["permissionless_builder",["",[],["permissionless-builder.rs"]]],["pub_key",["",[],["pub-key.rs"]]],["request_response",["",[],["data_source.rs","lib.rs","message.rs","network.rs","recipient_source.rs","request.rs","util.rs"]]],["reset_storage",["",[],["reset-storage.rs"]]],["sequencer",["",[["api",[],["data_source.rs","endpoints.rs","fs.rs","options.rs","sql.rs","update.rs"]],["network",[],["cdn.rs","libp2p.rs","mod.rs"]],["persistence",[],["fs.rs","sql.rs"]],["request_response",[],["data_source.rs","mod.rs","network.rs","recipient_source.rs","request.rs"]],["state_signature",[],["relay_server.rs"]]],["api.rs","catchup.rs","context.rs","external_event_handler.rs","genesis.rs","lib.rs","options.rs","persistence.rs","proposal_fetcher.rs","run.rs","state.rs","state_signature.rs"]]],["sequencer_utils",["",[],["blocknative.rs","deployer.rs","lib.rs","logging.rs","ser.rs","stake_table.rs","test_utils.rs"]]],["staking_cli",["",[],["claim.rs","delegation.rs","demo.rs","l1.rs","lib.rs","parse.rs","registration.rs"]]],["state_prover",["",[],["state-prover.rs"]]],["state_relay_server",["",[],["state-relay-server.rs"]]],["submit_transactions",["",[],["submit-transactions.rs"]]],["update_permissioned_stake_table",["",[],["update-permissioned-stake-table.rs"]]],["utils",["",[],["keygen.rs","main.rs","pubkey.rs","reset_storage.rs"]]],["verify_headers",["",[],["verify-headers.rs"]]],["vid",["",[["avid_m",[],["config.rs","namespaced.rs","proofs.rs"]],["utils",[],["bytes_to_field.rs"]]],["avid_m.rs","lib.rs","utils.rs"]]],["workspace_hack",["",[],["lib.rs"]]]]'));
createSrcSidebar();
//{"start":36,"fragment_lengths":[52,41,43,47,30,262,265,33,35,41,34,51,882,35,43,53,40,541,190,76,108,98,96,170,56,331,38,56,1084,169,98,83,415,472,678,80,33,67,72,300,101,45,409,45,65,35,128,47,516,125,109,45,57,59,83,73,49,140,38]}