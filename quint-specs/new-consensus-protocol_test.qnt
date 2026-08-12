// -*- mode: Bluespec; -*-
//
// Replay tests: traces the quint oracle captured from the real test suite,
// asserted step by step so a replay failure can be located in the spec itself.
module new_consensus_protocol_test {
  import new_consensus_protocol.* from "./new-consensus-protocol"

  /// `tests::consensus::test_decide_not_repeated_for_same_view`, as logged.
  run decideNotRepeatedForSameViewTest =
    init
      .then(receive_proposal(2, 0))
      .then(state_validated(2))
      .then(vote_1(1))
      .then(stored(1))
      .then(block_reconstructed(1))
      .then(receive_proposal(4, 2))
      .then(state_validated(4))
      .then(vote_1(2))
      .then(stored(2))
      .then(block_reconstructed(2))
      .then(receive_cert1(4))
      .then(vote_2_and_update_lock(2))
      .then(stored_high_qc(2))
      .then(receive_cert2(4))
      .then(decide(2))
      .expect(node.proto.decidedViews == Set(1, 2))
      .expect(node.history.sentVote2 == Set(4))

  /// `tests::consensus::test_cert2_broadcast_once`: the same prefix, plus a
  /// late Cert2 for the already-decided ancestor view.
  run cert2BroadcastOnceTest =
    init
      .then(receive_proposal(2, 0))
      .then(state_validated(2))
      .then(vote_1(1))
      .then(stored(1))
      .then(block_reconstructed(1))
      .then(receive_proposal(4, 2))
      .then(state_validated(4))
      .then(vote_1(2))
      .then(stored(2))
      .then(block_reconstructed(2))
      .then(receive_cert1(4))
      .then(vote_2_and_update_lock(2))
      .then(stored_high_qc(2))
      .then(receive_cert2(4))
      .then(decide(2))
      .then(receive_cert2(2))
      .expect(node.proto.certs2.keys() == Set(1, 2))

  /// The step at which replay stopped, in isolation: the second proposal must
  /// be accepted from the state the first five events produce.
  run secondProposalAcceptedTest =
    init
      .then(receive_proposal(2, 0))
      .then(state_validated(2))
      .then(vote_1(1))
      .then(stored(1))
      .then(block_reconstructed(1))
      .expect(can_receive_proposal(node, 4, 2))
      .then(receive_proposal(4, 2))
      .expect(node.proto.proposals.get(2) == 4)
}
