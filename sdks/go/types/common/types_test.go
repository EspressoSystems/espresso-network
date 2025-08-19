package common

import (
	"encoding/json"
	"strings"
	"testing"

	tagged_base64 "github.com/EspressoSystems/espresso-network/sdks/go/tagged-base64"

	"github.com/ethereum/go-ethereum/common"

	"github.com/stretchr/testify/require"
)

// Reference data taken from the reference sequencer implementation
// (https://github.com/EspressoSystems/espresso-network/blob/main/data)

var ReferenceL1BLockInfo L1BlockInfo = L1BlockInfo{
	Number:    123,
	Timestamp: *NewU256().SetUint64(0x456),
	Hash:      common.Hash{0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef},
}

var ReferenceNsTable NsTable = NsTable{
	Bytes: []byte{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0},
}

var ReferencePayloadCommitment, _ = tagged_base64.Parse("HASH~1yS-KEtL3oDZDBJdsW51Pd7zywIiHesBZsTbpOzrxOfu")
var ReferenceBuilderCommitment, _ = tagged_base64.Parse("BUILDER_COMMITMENT~tEvs0rxqOiMCvfe2R0omNNaphSlUiEDrb2q0IZpRcgA_")
var ReferenceBlockMerkleTreeRoot, _ = tagged_base64.Parse("MERKLE_COMM~yB4_Aqa35_PoskgTpcCR1oVLh6BUdLHIs7erHKWi-usUAAAAAAAAAAEAAAAAAAAAJg")
var ReferenceFeeMerkleTreeRoot, _ = tagged_base64.Parse("MERKLE_COMM~VJ9z239aP9GZDrHp3VxwPd_0l28Hc5KEAB1pFeCIxhYgAAAAAAAAAAIAAAAAAAAAdA")

const SampleDecideEvent = `{"view_number":13,"event":{"Decide":{"leaf_chain":[{"leaf":{"view_number":13,"justify_qc":{"data":{"leaf_commit":"COMMIT~zaORMZWZIlnqu00P22csaMNcTplixSCL0AmihTBlyyTG","epoch":null,"block_number":null},"vote_commitment":"COMMIT~ksmmmukO_YhmfY6i5Iz9YdRkFEqCizB9ZlupwVT4bsgZ","view_number":12,"signatures":["BLS_SIG~HadXl0J3VyOXA8tiBx_6e6OKrpCjzma2qDR6S-kgUhqkQ9EA23jXZN0g1fBTG5KM-KmN61jwGydMzh7Y6IX1nZc",{"order":"bitvec::order::Lsb0","head":{"width":64,"index":0},"bits":5,"data":[27]}],"_pd":null},"next_epoch_justify_qc":null,"parent_commitment":"COMMIT~zaORMZWZIlnqu00P22csaMNcTplixSCL0AmihTBlyyTG","block_header":{"version":{"Version":{"major":0,"minor":2}},"fields":{"chain_config":{"chain_config":{"Left":{"chain_id":"999999999","max_block_size":"1000000","base_fee":"1","fee_contract":"0x8ce361602b935680e8dec218b820ff5056beb7af","fee_recipient":"0x0000000000000000000000000000000000000000"}}},"height":13,"timestamp":1755545348,"l1_head":44,"l1_finalized":{"number":32,"timestamp":"0x68a37ef8","hash":"0xb17c948b9823ea5521ff7f3b5297451ec85f7b5dc21bec8ed3d51f592f25911c"},"payload_commitment":"HASH~K-IZa5wZsJE7EdcI0lUM3w9bAQbErg7sKqB9KyQ8cmgO","builder_commitment":"BUILDER_COMMITMENT~tEvs0rxqOiMCvfe2R0omNNaphSlUiEDrb2q0IZpRcgA_","ns_table":{"bytes":"AAAAAA=="},"block_merkle_tree_root":"MERKLE_COMM~jxzly80I3ehNWeOmXSmv8aEjHp5DD-69DKXmYONWQa8gAAAAAAAAAA0AAAAAAAAA4Q","fee_merkle_tree_root":"MERKLE_COMM~AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAUAAAAAAAAAAAAAAAAAAAAKA","fee_info":{"account":"0xb0cfa4e5893107e2995974ef032957752bb526e9","amount":"0"},"builder_signature":{"r":"0xa45dbada8e2586f543ee4dcba3ce69be67c4da7acb673b3d3e504945ab7cb4cd","s":"0x7252f44780ed26e40212bee84cb92659cc5c289911533dd6721d7ffa67448a49","v":27}}},"upgrade_certificate":null,"block_payload":{"raw_payload":"","ns_table":{"bytes":"AAAAAA=="}},"view_change_evidence":null,"next_drb_result":null,"with_epoch":false},"state":{"block_merkle_tree":{"root":{"Branch":{"value":"FIELD~jxzly80I3ehNWeOmXSmv8aEjHp5DD-69DKXmYONWQa8N","children":[{"Branch":{"value":"FIELD~GKNh-I_4cmmJA3SmrzTkTcINkiaNatfYMij1r7eg0X10","children":[{"Branch":{"value":"FIELD~f_NWxD5eczjLhl8eG1SY5vf7QIPsGyoopCD90IsPHdU0","children":[{"Branch":{"value":"FIELD~nOn5A4ZqJFH0dYZWxbhNmXt3bPstibcZ4XN7t6ekLZnN","children":[{"Branch":{"value":"FIELD~e3fBo7JKwsfZxpB52B0NW_p1g6KTiPTK-Oljie-gRu6l","children":[{"Branch":{"value":"FIELD~csSdub-sBrjGexOyALWHMYOisr7EF28xp-DEshpnIPdF","children":[{"Branch":{"value":"FIELD~FjtXsFhUmrKSJSbEsHOSAkivlaDN9-HbWpxkpcyjRIm2","children":[{"Branch":{"value":"FIELD~yhKl_b8xbgGQC4i45lQ5Q-KN-H_1kMl8Bq0abcdRSAYY","children":[{"Branch":{"value":"FIELD~x3JI567_Dt7K6hiChGrtsaeWK9Tbu5Kgb7sFvfsduiBN","children":[{"Branch":{"value":"FIELD~O2yZv7r2MBREY4nGYpeVQ10uqWKifixnpO4zocGXIqpU","children":[{"Branch":{"value":"FIELD~I6IMj9huHqN2vqmE1agN9NrNfk9P3H65rWz3rJD3iHVc","children":[{"Branch":{"value":"FIELD~muyXsezNumMI6GRVNA2uSRlZeWvNlv-SbsBZsePZdq8s","children":[{"Branch":{"value":"FIELD~cDau8xQmDvYjIdJhRItB-XdLL3Uvg3WVMNAK16EtstuV","children":[{"Branch":{"value":"FIELD~in_MrPpdxr8ojTs9qDBkTKEofdQaiWsh7mdPspFze2-y","children":[{"Branch":{"value":"FIELD~zNgYvmElNzfNqN-vO6EGZukx19kOENmbAirSyuKF7jZV","children":[{"Branch":{"value":"FIELD~SXkiMd6P_KB1CNShnmzh2Y1QiKuCUcdoKifZAqXQAAQ9","children":[{"Branch":{"value":"FIELD~TUQdMMbQInbGzmmsW0GPfXWzsfKDBNleZAu3SUeDQOWJ","children":[{"Branch":{"value":"FIELD~A3XR1jl68IKsvER42eIff1klFxPdp6sN1bo2zS84FtOO","children":[{"Branch":{"value":"FIELD~_tXwKZWWm90uxNp76uwbNouMPlPX1Ps7g-S6rE9WW0uP","children":[{"Branch":{"value":"FIELD~0Y-pXT8FxpIb8MaPoAKTdZ3FzN3imM2AbwcU0JxsNw4s","children":[{"Branch":{"value":"FIELD~tOAMU8YCKZpVj-aROpZVJm1npAo7jvNgSR5RwAnQKYVt","children":[{"Branch":{"value":"FIELD~jAqiVaCVnswj-q2W2l_ankhWUB6MmzFKYofv5nuHWgNy","children":[{"Branch":{"value":"FIELD~iLqU0HxYps4VKI1DtOLYoCtCCci5ZIX96-6HQ3tJ_Ve4","children":[{"Branch":{"value":"FIELD~fDr8oiL5n5Xf_kBQ0uNhWJ2xWpiaBo1vfhz9nQnAWfLj","children":[{"Branch":{"value":"FIELD~u7q8ICs7SC2gpAIa3wZwHFUBOBrNKJ6ZPDiHBBjDiMCg","children":[{"Branch":{"value":"FIELD~_VkpH8hPbG3UpgXxUzu6EZzNJc--rgWWshJMORM_G5lD","children":[{"Branch":{"value":"FIELD~upMD7RfvaqVGQ7bO9Q3q_-Y55lRczYSg_EvgpyHMqAb8","children":[{"Branch":{"value":"FIELD~CDIQgYDKtvAUPhJFLxgGC2x12J6QIaPHD7Pv_FVICpk0","children":[{"Branch":{"value":"FIELD~oBy8vmZsCs9YRor0W6echiGlcCh6aIwC1PQKbXG2fXRf","children":[{"Branch":{"value":"FIELD~6pfuSyq3bLGubMNPRX4L1scfGJ5vIsnTTIr7hITn0nPV","children":[{"ForgettenSubtree":{"value":"FIELD~f-nbDdKMxHWU7pzR5fFy_w2ZZhlDV3NHCNttf6EIKEqt"}},{"Branch":{"value":"FIELD~VkugNleBhMKySZqT0QBgDsKODesFZP2IBikjTZ0ZCSxf","children":[{"ForgettenSubtree":{"value":"FIELD~_RD_oV26VoK-3lV2IwlR9gCAfDceZ2rb8JCFUcO2y-FD"}},{"Branch":{"value":"FIELD~1AUHLzz5VwTbnx-WVJVGidn6gCzXLeORuzKhgaI-PXkh","children":[{"Leaf":{"value":"FIELD~jjQkMGZBT9C9OWNrw1tmML2qDr9cASPOmWxvLumyyluU","pos":"FIELD~DAAAAAAAAAAv","elem":"FIELD~jJopY0g78dwUut33hkW25W62DPMi7DSRXuBjqaAKhdTb"}},"Empty","Empty"]}},"Empty"]}},"Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"Empty","Empty"]}},"height":32,"num_leaves":13,"_phantom":null},"fee_merkle_tree":{"root":"Empty","height":20,"num_leaves":0,"_phantom":null},"reward_merkle_tree_v1":{"root":"Empty","height":20,"num_leaves":0,"_phantom":null},"reward_merkle_tree_v2":{"root":"Empty","height":160,"num_leaves":0,"_phantom":null},"chain_config":{"chain_config":{"Left":{"chain_id":"999999999","max_block_size":"1000000","base_fee":"1","fee_contract":"0x8ce361602b935680e8dec218b820ff5056beb7af","fee_recipient":"0x0000000000000000000000000000000000000000","stake_table_contract":null}}}},"delta":{"fees_delta":["0xb0cfa4e5893107e2995974ef032957752bb526e9","0x0000000000000000000000000000000000000000"],"rewards_delta":[]},"vid_share":{"V0":{"view_number":13,"payload_commitment":"HASH~K-IZa5wZsJE7EdcI0lUM3w9bAQbErg7sKqB9KyQ8cmgO","share":{"index":4,"evals":"FIELD~AAAAAAAAAAD7","aggregate_proofs":"FIELD~AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQGY","evals_proof":{"pos":"FIELD~BAAAAAAAAAC3","proof":[{"Leaf":{"value":"FIELD~a28U5q8mJxhr-KVeR03PbvP2zoGvVg2ocexR62cBcanN","pos":"FIELD~BAAAAAAAAAC3","elem":"FIELD~AAAAAAAAAAD7"}},{"Branch":{"value":"FIELD~AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA-","children":[{"ForgettenSubtree":{"value":"FIELD~WdWWbJavfsrVydKRjWWC0QKyxn9rdl6iisJDcatPk77C"}},{"ForgettenSubtree":{"value":"FIELD~a28U5q8mJxhr-KVeR03PbvP2zoGvVg2ocexR62cBcanN"}},"Empty"]}},{"Branch":{"value":"FIELD~AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA-","children":[{"ForgettenSubtree":{"value":"FIELD~lf76E5t7luYWTs4hhs3Fhwd6XNhypSL_1MKH4eJU6u_T"}},{"ForgettenSubtree":{"value":"FIELD~9vP_o7cAgIbihRvlRrOrp2E6L5S5TNDYjLmf8DcnNBX5"}},"Empty"]}}]}},"common":{"poly_commits":"FIELD~AAAAAAAAAAD7","all_evals_digest":"FIELD~HE7xxRlER6dA-1s93kzScegDxQTi-H-PpEL2vIBhgaXb","payload_byte_len":0,"num_storage_nodes":5,"multiplicity":1},"recipient_key":"BLS_VER_KEY~bQszS-QKYvUij2g20VqS8asttGSb95NrTu2PUj0uMh1CBUxNy1FqyPDjZqB29M7ZbjWqj79QkEOWkpga84AmDYUeTuWmy-0P1AdKHD3ehc-dKvei78BDj5USwXPJiDUlCxvYs_9rWYhagaq-5_LXENr78xel17spftNd5MA1Mw5U"}},"state_cert":null}],"qc":{"data":{"leaf_commit":"COMMIT~zDQj9T-fNklWU9VpkdYCE3n6vsf697rt8_uHDwgyving","epoch":null,"block_number":null},"vote_commitment":"COMMIT~Wwdd_O5aIONcAtJDlPmi9Gnv4MCrqU_Qaik9A4yz-gU_","view_number":13,"signatures":["BLS_SIG~2YwGf9PcBz0fkcg92TXTQYju867TkrZXqbSuU3HQfShjpqJo54AyY2QOfH-xYpgJaH8s6-lyxjf8O4efHk-yrgE",{"order":"bitvec::order::Lsb0","head":{"width":64,"index":0},"bits":5,"data":[27]}],"_pd":null},"block_size":0}}}`
const SampleQuorumEvent = `{"view_number":13,"event":{"QuorumProposal":{"proposal":{"data":{"proposal":{"block_header":{"version":{"Version":{"major":0,"minor":2}},"fields":{"chain_config":{"chain_config":{"Left":{"chain_id":"999999999","max_block_size":"1000000","base_fee":"1","fee_contract":"0x8ce361602b935680e8dec218b820ff5056beb7af","fee_recipient":"0x0000000000000000000000000000000000000000"}}},"height":13,"timestamp":1755545348,"l1_head":44,"l1_finalized":{"number":32,"timestamp":"0x68a37ef8","hash":"0xb17c948b9823ea5521ff7f3b5297451ec85f7b5dc21bec8ed3d51f592f25911c"},"payload_commitment":"HASH~K-IZa5wZsJE7EdcI0lUM3w9bAQbErg7sKqB9KyQ8cmgO","builder_commitment":"BUILDER_COMMITMENT~tEvs0rxqOiMCvfe2R0omNNaphSlUiEDrb2q0IZpRcgA_","ns_table":{"bytes":"AAAAAA=="},"block_merkle_tree_root":"MERKLE_COMM~jxzly80I3ehNWeOmXSmv8aEjHp5DD-69DKXmYONWQa8gAAAAAAAAAA0AAAAAAAAA4Q","fee_merkle_tree_root":"MERKLE_COMM~AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAUAAAAAAAAAAAAAAAAAAAAKA","fee_info":{"account":"0xb0cfa4e5893107e2995974ef032957752bb526e9","amount":"0"},"builder_signature":{"r":"0xa45dbada8e2586f543ee4dcba3ce69be67c4da7acb673b3d3e504945ab7cb4cd","s":"0x7252f44780ed26e40212bee84cb92659cc5c289911533dd6721d7ffa67448a49","v":27}}},"view_number":13,"epoch":null,"justify_qc":{"data":{"leaf_commit":"COMMIT~zaORMZWZIlnqu00P22csaMNcTplixSCL0AmihTBlyyTG","epoch":null,"block_number":null},"vote_commitment":"COMMIT~ksmmmukO_YhmfY6i5Iz9YdRkFEqCizB9ZlupwVT4bsgZ","view_number":12,"signatures":["BLS_SIG~HadXl0J3VyOXA8tiBx_6e6OKrpCjzma2qDR6S-kgUhqkQ9EA23jXZN0g1fBTG5KM-KmN61jwGydMzh7Y6IX1nZc",{"order":"bitvec::order::Lsb0","head":{"width":64,"index":0},"bits":5,"data":[27]}],"_pd":null},"next_epoch_justify_qc":null,"upgrade_certificate":null,"view_change_evidence":null,"next_drb_result":null,"state_cert":null}},"signature":"BLS_SIG~Cy95RxAs7a9b9oedjd-TFwGZJyeaXvrbBIUnNgFM2A6Trcpk8Uzq6gBx68r6WDZyQoHN4MpOnaGl35y50pgzF0I","_pd":null},"sender":"BLS_VER_KEY~4zQnaCOFJ7m95OjxeNls0QOOwWbz4rfxaL3NwmN2zSdnf8t5Nw_dfmMHq05ee8jCegw6Bn5T8inmrnGGAsQJMMWLv77nd7FJziz2ViAbXg-XGGF7o4HyzELCmypDOIYF3X2UWferFE_n72ZX0iQkUhOvYZZ7cfXToXxRTtb_mwRR"}}}`
const SampleDAEvent = `{"view_number":13,"event":{"DaProposal":{"proposal":{"data":{"encoded_transactions":[],"metadata":{"bytes":"AAAAAA=="},"view_number":13,"epoch":null,"epoch_transition_indicator":"NotInTransition"},"signature":"BLS_SIG~YkC0O16wIMY6udC53JCHQJj9gWfrbA9usk1HbE0uiwATt4Eq-XdEI60LW2UUc3_L8CHcsXxGRQsm2ZCYCsv8FUk","_pd":null},"sender":"BLS_VER_KEY~4zQnaCOFJ7m95OjxeNls0QOOwWbz4rfxaL3NwmN2zSdnf8t5Nw_dfmMHq05ee8jCegw6Bn5T8inmrnGGAsQJMMWLv77nd7FJziz2ViAbXg-XGGF7o4HyzELCmypDOIYF3X2UWferFE_n72ZX0iQkUhOvYZZ7cfXToXxRTtb_mwRR"}}}`
const SampleViewFinishedEvent = `{"view_number":13,"event":{"ViewFinished":{"view_number":13}}}`

var ReferenceTransaction Transaction = Transaction{
	Namespace: 12648430,
	Payload:   []byte{76, 111, 114, 101, 109, 32, 105, 112, 115, 117, 109, 32, 100, 111, 108, 111, 114, 32, 115, 105, 116, 32, 97, 109, 101, 116, 44, 32, 99, 111, 110, 115, 101, 99, 116, 101, 116, 117, 114, 32, 97, 100, 105, 112, 105, 115, 99, 105, 110, 103, 32, 101, 108, 105, 116, 46, 32, 68, 111, 110, 101, 99, 32, 108, 101, 99, 116, 117, 115, 32, 118, 101, 108, 105, 116, 44, 32, 99, 111, 109, 109, 111, 100, 111, 32, 101, 103, 101, 116, 32, 116, 101, 108, 108, 117, 115, 32, 118, 105, 116, 97, 101, 44, 32, 109, 111, 108, 101, 115, 116, 105, 101, 32, 109, 97, 120, 105, 109, 117, 115, 32, 116, 117, 114, 112, 105, 115, 46, 32, 77, 97, 101, 99, 101, 110, 97, 115, 32, 108, 97, 99, 117, 115, 32, 109, 97, 117, 114, 105, 115, 44, 32, 97, 117, 99, 116, 111, 114, 32, 113, 117, 105, 115, 32, 108, 97, 99, 117, 115, 32, 97, 116, 44, 32, 97, 117, 99, 116, 111, 114, 32, 118, 111, 108, 117, 116, 112, 97, 116, 32, 110, 105, 115, 105, 46, 32, 70, 117, 115, 99, 101, 32, 109, 111, 108, 101, 115, 116, 105, 101, 32, 117, 114, 110, 97, 32, 115, 105, 116, 32, 97, 109, 101, 116, 32, 113, 117, 97, 109, 32, 105, 109, 112, 101, 114, 100, 105, 101, 116, 32, 115, 117, 115, 99, 105, 112, 105, 116, 46, 32, 68, 111, 110, 101, 99, 32, 101, 108, 105, 116, 32, 108, 101, 99, 116, 117, 115, 44, 32, 100, 97, 112, 105, 98, 117, 115, 32, 105, 110, 32, 105, 112, 115, 117, 109, 32, 101, 116, 44, 32, 118, 105, 118, 101, 114, 114, 97, 32, 112, 104, 97, 114, 101, 116, 114, 97, 32, 102, 101, 108, 105, 115, 46, 32, 83, 101, 100, 32, 115, 101, 100, 32, 115, 101, 109, 32, 115, 101, 100, 32, 108, 105, 98, 101, 114, 111, 32, 115, 101, 109, 112, 101, 114, 32, 112, 111, 115, 117, 101, 114, 101, 46, 32, 85, 116, 32, 101, 117, 105, 115, 109, 111, 100, 32, 112, 117, 114, 117, 115, 32, 97, 116, 32, 109, 111, 108, 101, 115, 116, 105, 101, 32, 118, 111, 108, 117, 116, 112, 97, 116, 46, 32, 78, 117, 110, 99, 32, 101, 117, 105, 115, 109, 111, 100, 32, 105, 100, 32, 101, 115, 116, 32, 110, 101, 99, 32, 101, 117, 105, 115, 109, 111, 100, 46, 32, 65, 108, 105, 113, 117, 97, 109, 32, 113, 117, 105, 115, 32, 101, 114, 97, 116, 32, 98, 105, 98, 101, 110, 100, 117, 109, 44, 32, 101, 103, 101, 115, 116, 97, 115, 32, 97, 117, 103, 117, 101, 32, 113, 117, 105, 115, 44, 32, 116, 105, 110, 99, 105, 100, 117, 110, 116, 32, 116, 101, 108, 108, 117, 115, 46, 32, 68, 117, 105, 115, 32, 100, 97, 112, 105, 98, 117, 115, 32, 97, 99, 32, 106, 117, 115, 116, 111, 32, 117, 116, 32, 114, 104, 111, 110, 99, 117, 115, 46, 32, 78, 117, 108, 108, 97, 32, 118, 101, 104, 105, 99, 117, 108, 97, 32, 97, 117, 103, 117, 101, 32, 110, 111, 110, 32, 97, 114, 99, 117, 32, 118, 101, 115, 116, 105, 98, 117, 108, 117, 109, 32, 116, 101, 109, 112, 117, 115, 46, 32, 68, 117, 105, 115, 32, 117, 108, 108, 97, 109, 99, 111, 114, 112, 101, 114, 32, 115, 105, 116, 32, 97, 109, 101, 116, 32, 108, 97, 99, 117, 115, 32, 101, 116, 32, 100, 105, 103, 110, 105, 115, 115, 105, 109, 46, 32, 77, 97, 117, 114, 105, 115, 32, 97, 117, 99, 116, 111, 114, 32, 115, 111, 108, 108, 105, 99, 105, 116, 117, 100, 105, 110, 32, 102, 101, 117, 103, 105, 97, 116, 46, 32, 70, 117, 115, 99, 101, 32, 116, 105, 110, 99, 105, 100, 117, 110, 116, 32, 99, 111, 110, 100, 105, 109, 101, 110, 116, 117, 109, 32, 100, 97, 112, 105, 98, 117, 115, 46, 32, 65, 108, 105, 113, 117, 97, 109, 32, 97, 114, 99, 117, 32, 108, 101, 99, 116, 117, 115, 44, 32, 98, 108, 97, 110, 100, 105, 116, 32, 115, 101, 100, 32, 115, 101, 109, 32, 115, 105, 116, 32, 97, 109, 101, 116, 44, 32, 102, 101, 114, 109, 101, 110, 116, 117, 109, 32, 118, 101, 104, 105, 99, 117, 108, 97, 32, 109, 101, 116, 117, 115, 46, 32, 77, 97, 101, 99, 101, 110, 97, 115, 32, 116, 117, 114, 112, 105, 115, 32, 110, 101, 113, 117, 101, 44, 32, 116, 114, 105, 115, 116, 105, 113, 117, 101, 32, 101, 103, 101, 116, 32, 116, 105, 110, 99, 105, 100, 117, 110, 116, 32, 117, 116, 44, 32, 115, 99, 101, 108, 101, 114, 105, 115, 113, 117, 101, 32, 101, 117, 32, 108, 97, 99, 117, 115, 46, 32, 85, 116, 32, 98, 108, 97, 110, 100, 105, 116, 32, 101, 117, 32, 108, 101, 111, 32, 118, 105, 116, 97, 101, 32, 118, 111, 108, 117, 116, 112, 97, 116, 46},
}

func removeWhitespace(s string) string {
	// Split the string on whitespace then concatenate the segments
	return strings.Join(strings.Fields(s), "")
}
func TestEspressoTypesL1BLockInfoJson(t *testing.T) {
	data := []byte(removeWhitespace(`{
		"number": 123,
		"timestamp": "0x456",
		"hash": "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
	}`))

	// Check encoding.
	encoded, err := json.Marshal(ReferenceL1BLockInfo)
	if err != nil {
		t.Fatalf("Failed to marshal JSON: %v", err)
	}
	require.Equal(t, encoded, data)

	// Check decoding
	var decoded L1BlockInfo
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("Failed to unmarshal JSON: %v", err)
	}
	require.Equal(t, decoded, ReferenceL1BLockInfo)

	CheckJsonRequiredFields[L1BlockInfo](t, data, "number", "timestamp", "hash")
}

func TestEspressoTransactionJson(t *testing.T) {
	data := []byte(removeWhitespace(`{
		"namespace": 12648430,
		"payload": "TG9yZW0gaXBzdW0gZG9sb3Igc2l0IGFtZXQsIGNvbnNlY3RldHVyIGFkaXBpc2NpbmcgZWxpdC4gRG9uZWMgbGVjdHVzIHZlbGl0LCBjb21tb2RvIGVnZXQgdGVsbHVzIHZpdGFlLCBtb2xlc3RpZSBtYXhpbXVzIHR1cnBpcy4gTWFlY2VuYXMgbGFjdXMgbWF1cmlzLCBhdWN0b3IgcXVpcyBsYWN1cyBhdCwgYXVjdG9yIHZvbHV0cGF0IG5pc2kuIEZ1c2NlIG1vbGVzdGllIHVybmEgc2l0IGFtZXQgcXVhbSBpbXBlcmRpZXQgc3VzY2lwaXQuIERvbmVjIGVsaXQgbGVjdHVzLCBkYXBpYnVzIGluIGlwc3VtIGV0LCB2aXZlcnJhIHBoYXJldHJhIGZlbGlzLiBTZWQgc2VkIHNlbSBzZWQgbGliZXJvIHNlbXBlciBwb3N1ZXJlLiBVdCBldWlzbW9kIHB1cnVzIGF0IG1vbGVzdGllIHZvbHV0cGF0LiBOdW5jIGV1aXNtb2QgaWQgZXN0IG5lYyBldWlzbW9kLiBBbGlxdWFtIHF1aXMgZXJhdCBiaWJlbmR1bSwgZWdlc3RhcyBhdWd1ZSBxdWlzLCB0aW5jaWR1bnQgdGVsbHVzLiBEdWlzIGRhcGlidXMgYWMganVzdG8gdXQgcmhvbmN1cy4gTnVsbGEgdmVoaWN1bGEgYXVndWUgbm9uIGFyY3UgdmVzdGlidWx1bSB0ZW1wdXMuIER1aXMgdWxsYW1jb3JwZXIgc2l0IGFtZXQgbGFjdXMgZXQgZGlnbmlzc2ltLiBNYXVyaXMgYXVjdG9yIHNvbGxpY2l0dWRpbiBmZXVnaWF0LiBGdXNjZSB0aW5jaWR1bnQgY29uZGltZW50dW0gZGFwaWJ1cy4gQWxpcXVhbSBhcmN1IGxlY3R1cywgYmxhbmRpdCBzZWQgc2VtIHNpdCBhbWV0LCBmZXJtZW50dW0gdmVoaWN1bGEgbWV0dXMuIE1hZWNlbmFzIHR1cnBpcyBuZXF1ZSwgdHJpc3RpcXVlIGVnZXQgdGluY2lkdW50IHV0LCBzY2VsZXJpc3F1ZSBldSBsYWN1cy4gVXQgYmxhbmRpdCBldSBsZW8gdml0YWUgdm9sdXRwYXQu"
	}`))

	tx := ReferenceTransaction
	// Check encoding.
	encoded, err := json.Marshal(tx)
	if err != nil {
		t.Fatalf("Failed to marshal JSON: %v", err)
	}
	require.Equal(t, encoded, data)

	// Check decoding
	var decoded Transaction
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("Failed to unmarshal JSON: %v", err)
	}
	require.Equal(t, decoded, tx)

	CheckJsonRequiredFields[Transaction](t, data, "namespace", "payload")
}

// Commitment tests ported from the reference sequencer implementation
// (https://github.com/EspressoSystems/espresso-network/blob/main/sequencer/src/block.rs)

func TestEspressoTypesL1BlockInfoCommit(t *testing.T) {
	require.Equal(t, ReferenceL1BLockInfo.Commit(), Commitment{224, 122, 115, 150, 226, 202, 216, 139, 51, 221, 23, 79, 54, 243, 107, 12, 12, 144, 113, 99, 133, 217, 207, 73, 120, 182, 115, 84, 210, 230, 126, 148})
}

func TestEspressoTypesNsTable(t *testing.T) {
	require.Equal(t, ReferenceNsTable.Commit(), Commitment{24, 191, 165, 16, 16, 48, 53, 144, 229, 119, 16, 233, 201, 36, 89, 64, 40, 77, 158, 105, 253, 188, 220, 221, 32, 2, 252, 91, 209, 13, 58, 232})
}

func TestEspressoTypesTransaction(t *testing.T) {
	require.Equal(t, ReferenceTransaction.Commit(), Commitment{239, 188, 78, 127, 214, 247, 253, 27, 70, 194, 164, 59, 255, 51, 143, 122, 226, 81, 75, 72, 153, 192, 94, 196, 38, 37, 127, 55, 51, 175, 226, 226})
}

func TestEspressoCommitmentFromU256TrailingZero(t *testing.T) {
	comm := Commitment{209, 146, 197, 195, 145, 148, 17, 211, 52, 72, 28, 120, 88, 182, 204, 206, 77, 36, 56, 35, 3, 143, 77, 186, 69, 233, 104, 30, 90, 105, 48, 0}
	roundTrip, err := CommitmentFromUint256(comm.Uint256())
	require.Nil(t, err)
	require.Equal(t, comm, roundTrip)
}

func TestEspressoQuorumProposalEvent(t *testing.T) {
	consensusMessageInBytes := []byte(removeWhitespace(SampleQuorumEvent))

	consensusMessage, err := UnmarshalConsensusMessage(consensusMessageInBytes)
	require.Nil(t, err)
	require.Equal(t, consensusMessage.Event.QuorumProposalWrapper.QuorumProposalDataWrapper.Data.Proposal.ViewNumber, 13)
	require.Equal(t, consensusMessage.Event.QuorumProposalWrapper.QuorumProposalDataWrapper.Data.Proposal.BlockHeader.Fields.BuilderCommitment, ReferenceBuilderCommitment.String())
}

func TestEspressoDaProposalEvent(t *testing.T) {
	consensusMessageInBytes := []byte(removeWhitespace(SampleDAEvent))

	consensusMessage, err := UnmarshalConsensusMessage(consensusMessageInBytes)
	require.Nil(t, err)
	require.Equal(t, consensusMessage.Event.DaProposalWrapper.DaProposalDataWrapper.Data.ViewNumber, 13)
	blockPayload, err := NewBlockPayload(consensusMessage.Event.DaProposalWrapper.DaProposalDataWrapper.Data.EncodedTransactions, consensusMessage.Event.DaProposalWrapper.DaProposalDataWrapper.Data.Metadata)
	require.Nil(t, err)
	builderCommitment, err := blockPayload.BuilderCommitment()
	require.Nil(t, err)
	builderCommitmentTagged, err := builderCommitment.ToTaggedSting()
	require.Nil(t, err)
	require.Equal(t, builderCommitmentTagged, ReferenceBuilderCommitment.String())
}

func TestEspressoDecideEvent(t *testing.T) {
	consensusMessageInBytes := []byte(removeWhitespace(SampleDecideEvent))

	consensusMessage, err := UnmarshalConsensusMessage(consensusMessageInBytes)
	require.Nil(t, err)
	require.Equal(t, consensusMessage.Event.Decide.LeafChain[0].Leaf.BlockHeader.Fields.BuilderCommitment, ReferenceBuilderCommitment.String())
	require.Equal(t, consensusMessage.Event.Decide.LeafChain[0].Leaf.ViewNumber, 13)
}

func TestViewFinishedEvent(t *testing.T) {
	consensusMessageInBytes := []byte(removeWhitespace(SampleViewFinishedEvent))
	consensusMessage, err := UnmarshalConsensusMessage(consensusMessageInBytes)
	require.Nil(t, err)
	require.Equal(t, consensusMessage.Event.ViewFinished.ViewNumber, 13)
}

func CheckJsonRequiredFields[T any](t *testing.T, data []byte, fields ...string) {
	// Parse the JSON object into a map so we can selectively delete fields.
	var obj map[string]json.RawMessage
	if err := json.Unmarshal(data, &obj); err != nil {
		t.Fatalf("failed to unmarshal JSON: %v", err)
	}

	for _, field := range fields {
		data, err := json.Marshal(withoutKey(obj, field))
		require.Nil(t, err, "failed to marshal JSON")

		var dec T
		err = json.Unmarshal(data, &dec)
		require.NotNil(t, err, "unmarshalling without required field %s should fail", field)
	}
}

func withoutKey[K comparable, V any](m map[K]V, key K) map[K]V {
	copied := make(map[K]V)
	for k, v := range m {
		if k != key {
			copied[k] = v
		}
	}
	return copied
}
