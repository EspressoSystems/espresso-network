use ark_bn254::{Bn254, Fq, Fr, G1Affine, G2Affine};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ed_on_bn254::{EdwardsConfig as EdOnBn254Config, Fq as FqEd254};
use ark_ff::field_hashers::{DefaultFieldHasher, HashToField};
use ark_poly::domain::radix2::Radix2EvaluationDomain;
use ark_poly::EvaluationDomain;
use ark_std::rand::{rngs::StdRng, Rng, SeedableRng};
use clap::{Parser, ValueEnum};
use diff_test_bn254::ParsedG2Point;
use ethers::{
    abi::{AbiDecode, AbiEncode, Address},
    types::{Bytes, U256},
};
use hotshot_contract::{
    jf_helpers::*,
    light_client::{MockLedger, MockSystemParam, ParsedLightClientState, STAKE_TABLE_CAPACITY},
};
use itertools::multiunzip;
use jf_plonk::proof_system::structs::{Proof, VerifyingKey};
use jf_plonk::proof_system::PlonkKzgSnark;
use jf_plonk::{
    testing_apis::Verifier,
    transcript::{PlonkTranscript, SolidityTranscript},
};
use jf_primitives::constants::CS_ID_BLS_BN254;
use jf_primitives::pcs::prelude::Commitment;
use jf_primitives::signatures::bls_over_bn254::Signature;
use jf_primitives::signatures::bls_over_bn254::{hash_to_curve, KeyPair as BLSKeyPair};
use jf_primitives::signatures::schnorr::KeyPair as SchnorrKeyPair;
use sha3::Keccak256;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    /// Identifier for the functions to invoke in Jellyfish
    #[arg(value_enum)]
    action: Action,
    /// Optional arguments for the `action`
    #[arg(value_parser, num_args = 1.., value_delimiter = ' ')]
    args: Vec<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Action {
    /// Get ark_poly::Radix2EvaluationDomain::new()
    NewPolyEvalDomain,
    /// Get ark_poly::Radix2EvaluationDomain::elements()
    EvalDomainElements,
    /// Get some poly evals during jf_plonk::prepare_pcs_info()
    EvalDataGen,
    /// Get jf_plonk::Transcript::append_message()
    TranscriptAppendMsg,
    /// Get jf_plonk::Transcript::append_challenge()
    TranscriptAppendField,
    /// Get jf_plonk::Transcript::append_commitment()
    TranscriptAppendGroup,
    /// Get jf_plonk::Transcript::get_and_append_challenge()
    TranscriptGetChal,
    /// Get jf_plonk::Transcript::append_vk_and_pub_input()
    TranscriptAppendVkAndPi,
    /// Get jf_plonk::Transcript::append_proof_evaluations()
    TranscriptAppendProofEvals,
    /// Return the Plonk Verifier related constants
    PlonkConstants,
    /// Get jf_plonk::Verifier::compute_challenges()
    PlonkComputeChal,
    /// Get jf_plonk::Verifier::aggregate_evaluations()
    PlonkPrepareEval,
    /// Get jf_plonk::Verifier::prepare_pcs_info()
    PlonkPreparePcsInfo,
    /// Get jf_plonk::Verifier::batch_verify()
    PlonkBatchVerify,
    /// Get a random, dummy proof with correct format
    DummyProof,
    /// Test only logic
    TestOnly,
    /// Generate Client Wallet
    GenClientWallet,
    /// Generate internal hash values for the BLS signature scheme
    GenBLSHashes,
    /// Generate BLS keys and a signature
    GenBLSSig,
    /// Get mock genesis light client state
    MockGenesis,
    /// Get a consecutive finalized light client states
    MockConsecutiveFinalizedStates,
    /// Get a light client state that skipped a few blocks
    MockSkipBlocks,
    /// Get light client states when missing ending block of an epoch
    MockMissEndingBlock,
}

#[allow(clippy::type_complexity)]
fn main() {
    let cli = Cli::parse();

    match cli.action {
        Action::NewPolyEvalDomain => {
            if cli.args.len() != 1 {
                panic!("Should provide arg1=logSize");
            }
            let log_size = cli.args[0].parse::<u32>().unwrap();

            let domain = Radix2EvaluationDomain::<Fr>::new(2u32.pow(log_size) as usize).unwrap();
            let res = (
                field_to_u256(domain.size_inv),
                field_to_u256(domain.group_gen),
                field_to_u256(domain.group_gen_inv),
            );
            println!("{}", res.encode_hex());
        }
        Action::EvalDomainElements => {
            if cli.args.len() != 2 {
                panic!("Should provide arg1=logSize, arg2=length");
            }
            let log_size = cli.args[0].parse::<u32>().unwrap();
            let length = cli.args[1].parse::<usize>().unwrap();

            let domain = Radix2EvaluationDomain::<Fr>::new(2u32.pow(log_size) as usize).unwrap();
            let res = domain
                .elements()
                .take(length)
                .map(field_to_u256)
                .collect::<Vec<_>>();
            println!("{}", res.encode_hex());
        }
        Action::EvalDataGen => {
            if cli.args.len() != 3 {
                panic!("Should provide arg1=logSize, arg2=zeta, arg3=publicInput");
            }

            let log_size = cli.args[0].parse::<u32>().unwrap();
            let zeta = u256_to_field::<Fr>(cli.args[1].parse::<U256>().unwrap());
            let pi_u256: Vec<U256> = AbiDecode::decode_hex(&cli.args[2]).unwrap();
            let pi: Vec<Fr> = pi_u256.into_iter().map(u256_to_field).collect();

            let verifier = Verifier::<Bn254>::new(2u32.pow(log_size) as usize).unwrap();
            let (vanish_eval, lagrange_one, _, pi_eval) = verifier
                .compute_poly_evals_for_pcs_info(&zeta, &pi)
                .unwrap();
            let res = (
                field_to_u256(vanish_eval),
                field_to_u256(lagrange_one),
                field_to_u256(pi_eval),
            );
            println!("{}", res.encode_hex());
        }
        Action::TranscriptAppendMsg => {
            if cli.args.len() != 2 {
                panic!("Should provide arg1=transcript, arg2=message");
            }
            let t_parsed = cli.args[0].parse::<ParsedTranscript>().unwrap();
            let msg = {
                let parsed: Bytes = AbiDecode::decode_hex(&cli.args[1]).unwrap();
                parsed.0.to_vec()
            };

            let mut t: SolidityTranscript = t_parsed.into();
            <SolidityTranscript as PlonkTranscript<Fr>>::append_message(&mut t, &[], &msg).unwrap();
            let res: ParsedTranscript = t.into();
            println!("{}", (res,).encode_hex());
        }
        Action::TranscriptAppendField => {
            if cli.args.len() != 2 {
                panic!("Should provide arg1=transcript, arg2=fieldElement");
            }
            let t_parsed = cli.args[0].parse::<ParsedTranscript>().unwrap();
            let field = u256_to_field::<Fr>(cli.args[1].parse::<U256>().unwrap());

            let mut t: SolidityTranscript = t_parsed.into();
            t.append_challenge::<Bn254>(&[], &field).unwrap();
            let res: ParsedTranscript = t.into();
            println!("{}", (res,).encode_hex());
        }
        Action::TranscriptAppendGroup => {
            if cli.args.len() != 2 {
                panic!("Should provide arg1=transcript, arg2=groupElement");
            }

            let t_parsed = cli.args[0].parse::<ParsedTranscript>().unwrap();
            let point: G1Affine = cli.args[1].parse::<ParsedG1Point>().unwrap().into();

            let mut t: SolidityTranscript = t_parsed.into();
            t.append_commitment::<Bn254, ark_bn254::g1::Config>(&[], &Commitment::from(point))
                .unwrap();
            let res: ParsedTranscript = t.into();
            println!("{}", (res,).encode_hex());
        }
        Action::TranscriptGetChal => {
            if cli.args.len() != 1 {
                panic!("Should provide arg1=transcript");
            }

            let t_parsed = cli.args[0].parse::<ParsedTranscript>().unwrap();

            let mut t: SolidityTranscript = t_parsed.into();
            let chal = t.get_and_append_challenge::<Bn254>(&[]).unwrap();

            let updated_t: ParsedTranscript = t.into();
            let res = (updated_t, field_to_u256(chal));
            println!("{}", res.encode_hex());
        }
        Action::TranscriptAppendVkAndPi => {
            if cli.args.len() != 3 {
                panic!("Should provide arg1=transcript, arg2=verifyingKey, arg3=publicInput");
            }

            let t_parsed = cli.args[0].parse::<ParsedTranscript>().unwrap();
            let vk_parsed = cli.args[1].parse::<ParsedVerifyingKey>().unwrap();
            let pi_u256: Vec<U256> = AbiDecode::decode_hex(&cli.args[2]).unwrap();
            let pi: Vec<Fr> = pi_u256.into_iter().map(u256_to_field).collect();

            let mut t: SolidityTranscript = t_parsed.into();
            let vk: VerifyingKey<Bn254> = vk_parsed.into();
            t.append_vk_and_pub_input(&vk, &pi).unwrap();

            let res: ParsedTranscript = t.into();
            println!("{}", (res,).encode_hex());
        }
        Action::TranscriptAppendProofEvals => {
            if cli.args.len() != 1 {
                panic!("Should provide arg1=transcript");
            }

            let mut rng = jf_utils::test_rng();

            let t_parsed = cli.args[0].parse::<ParsedTranscript>().unwrap();
            let proof_parsed = ParsedPlonkProof::dummy_with_rand_proof_evals(&mut rng);
            let proof: Proof<Bn254> = proof_parsed.clone().into();

            let mut t: SolidityTranscript = t_parsed.into();
            <SolidityTranscript as PlonkTranscript<Fr>>::append_proof_evaluations::<Bn254>(
                &mut t,
                &proof.poly_evals,
            )
            .unwrap();

            let t_updated: ParsedTranscript = t.into();
            let res = (t_updated, proof_parsed);
            println!("{}", res.encode_hex());
        }
        Action::PlonkConstants => {
            let coset_k = coset_k();
            let open_key = open_key();

            let res = (
                field_to_u256::<Fr>(coset_k[1]),
                field_to_u256::<Fr>(coset_k[2]),
                field_to_u256::<Fr>(coset_k[3]),
                field_to_u256::<Fr>(coset_k[4]),
                // NOTE: be EXTRA careful here!! Solidity's BN254.G2Point: Fp2 = x0 * u + x1
                // whereas in rust: Fp2 = x0 + x1 * u
                field_to_u256::<Fq>(open_key.beta_h.x().unwrap().c1),
                field_to_u256::<Fq>(open_key.beta_h.x().unwrap().c0),
                field_to_u256::<Fq>(open_key.beta_h.y().unwrap().c1),
                field_to_u256::<Fq>(open_key.beta_h.y().unwrap().c0),
            );
            println!("{}", res.encode_hex());
        }
        Action::PlonkComputeChal => {
            if cli.args.len() != 4 {
                panic!("Should provide arg1=verifyingKey, arg2=publicInput, arg3=proof, arg4=extraTranscriptInitMsg");
            }

            let vk = cli.args[0].parse::<ParsedVerifyingKey>().unwrap().into();
            let pi_u256: Vec<U256> = AbiDecode::decode_hex(&cli.args[1]).unwrap();
            let pi: Vec<Fr> = pi_u256.into_iter().map(u256_to_field).collect();
            let proof: Proof<Bn254> = cli.args[2].parse::<ParsedPlonkProof>().unwrap().into();
            let msg = {
                let parsed: Bytes = AbiDecode::decode_hex(&cli.args[3]).unwrap();
                parsed.0.to_vec()
            };

            let chal: ParsedChallenges =
                Verifier::<Bn254>::compute_challenges::<SolidityTranscript>(
                    &[&vk],
                    &[&pi],
                    &proof.into(),
                    &Some(msg),
                )
                .unwrap()
                .into();
            println!("{}", (chal,).encode_hex());
        }
        Action::PlonkPrepareEval => {
            if cli.args.len() != 3 {
                panic!("Should provide arg1=proof, arg2=linPolyConstant, arg3=commScalars");
            }

            let proof: Proof<Bn254> = cli.args[0].parse::<ParsedPlonkProof>().unwrap().into();
            let lin_poly_constant = u256_to_field::<Fr>(cli.args[1].parse::<U256>().unwrap());
            let comm_scalars_u256: Vec<U256> = AbiDecode::decode_hex(&cli.args[2]).unwrap();
            // NOTE: only take the last 10 scalars, the first 20 are linearization scalars
            let comm_scalars: Vec<Fr> = comm_scalars_u256
                .into_iter()
                .skip(20)
                .map(u256_to_field)
                .collect();

            let eval = Verifier::<Bn254>::aggregate_evaluations(
                &lin_poly_constant,
                &[proof.poly_evals],
                &[None],
                &comm_scalars,
            )
            .unwrap();
            let res = field_to_u256(eval);
            println!("{}", (res,).encode_hex());
        }
        Action::PlonkPreparePcsInfo => {
            if cli.args.len() != 4 {
                panic!("Should provide arg1=verifyingKey, arg2=publicInput, arg3=proof, arg4=extraTranscriptInitMsg");
            }

            let vk: VerifyingKey<Bn254> = cli.args[0].parse::<ParsedVerifyingKey>().unwrap().into();
            let pi_u256: Vec<U256> = AbiDecode::decode_hex(&cli.args[1]).unwrap();
            let pi: Vec<Fr> = pi_u256.into_iter().map(u256_to_field).collect();
            let proof: Proof<Bn254> = cli.args[2].parse::<ParsedPlonkProof>().unwrap().into();
            let msg = {
                let parsed: Bytes = AbiDecode::decode_hex(&cli.args[3]).unwrap();
                parsed.0.to_vec()
            };

            let verifier = Verifier::<Bn254>::new(vk.domain_size).unwrap();
            let pcs_info = verifier
                .prepare_pcs_info::<SolidityTranscript>(&[&vk], &[&pi], &proof.into(), &Some(msg))
                .unwrap();

            let scalars_and_bases_prod: ParsedG1Point = pcs_info
                .comm_scalars_and_bases
                .multi_scalar_mul()
                .into_affine()
                .into();
            let opening_proof: ParsedG1Point = pcs_info.opening_proof.0.into();
            let shifted_opening_proof: ParsedG1Point = pcs_info.shifted_opening_proof.0.into();
            let res = (
                field_to_u256(pcs_info.u),
                field_to_u256(pcs_info.eval_point),
                field_to_u256(pcs_info.next_eval_point),
                field_to_u256(pcs_info.eval),
                scalars_and_bases_prod,
                opening_proof,
                shifted_opening_proof,
            );
            println!("{}", res.encode_hex());
        }
        Action::PlonkBatchVerify => {
            if cli.args.len() != 1 {
                panic!("Should provide arg1=numProof");
            }

            let num_proof = cli.args[0].parse::<u32>().unwrap();
            let (proofs, vks, public_inputs, extra_msgs, _): (
                Vec<Proof<Bn254>>,
                Vec<VerifyingKey<Bn254>>,
                Vec<Vec<Fr>>,
                Vec<Option<Vec<u8>>>,
                Vec<usize>,
            ) = multiunzip(gen_plonk_proof_for_test(num_proof as usize));

            // ensure they are correct params
            let proofs_refs: Vec<&Proof<Bn254>> = proofs.iter().collect();
            let vks_refs: Vec<&VerifyingKey<Bn254>> = vks.iter().collect();
            let pi_refs: Vec<&[Fr]> = public_inputs
                .iter()
                .map(|pub_input| &pub_input[..])
                .collect();
            assert!(PlonkKzgSnark::batch_verify::<SolidityTranscript>(
                &vks_refs,
                &pi_refs,
                &proofs_refs,
                &extra_msgs
            )
            .is_ok());

            let vks_parsed: Vec<ParsedVerifyingKey> = vks.into_iter().map(Into::into).collect();
            let pis_parsed: Vec<Vec<U256>> = public_inputs
                .into_iter()
                .map(|pi| pi.into_iter().map(field_to_u256).collect())
                .collect();
            let proofs_parsed: Vec<ParsedPlonkProof> = proofs.into_iter().map(Into::into).collect();
            let msgs_parsed: Vec<Bytes> = extra_msgs
                .into_iter()
                .map(|msg| msg.unwrap().into())
                .collect();

            let res = (vks_parsed, pis_parsed, proofs_parsed, msgs_parsed);
            println!("{}", res.encode_hex());
        }
        Action::DummyProof => {
            let mut rng = jf_utils::test_rng();
            if !cli.args.is_empty() {
                let seed = cli.args[0].parse::<u64>().unwrap();
                rng = StdRng::seed_from_u64(seed);
            }
            let proof = ParsedPlonkProof::dummy(&mut rng);
            println!("{}", (proof,).encode_hex());
        }
        Action::TestOnly => {
            println!("args: {:?}", cli.args);
        }
        Action::GenClientWallet => {
            if cli.args.len() != 2 {
                panic!("Should provide arg1=senderAddress arg2=seed");
            }

            // Use seed from cli to generate different bls keys
            let seed_value: u8 = cli.args[1].parse::<u8>().unwrap();
            let seed = [seed_value; 32];
            let mut rng = StdRng::from_seed(seed);

            let sender_address = cli.args[0].parse::<Address>().unwrap();
            let sender_address_bytes = AbiEncode::encode(sender_address);

            // Generate the Schnorr key
            let schnorr_key_pair: SchnorrKeyPair<EdOnBn254Config> =
                SchnorrKeyPair::generate(&mut rng);
            let schnorr_ver_key = schnorr_key_pair.ver_key();
            let schnorr_ver_key_affine = schnorr_ver_key.to_affine();
            let schnorr_pk_x = field_to_u256::<FqEd254>(schnorr_ver_key_affine.x);
            let schnorr_pk_y = field_to_u256::<FqEd254>(schnorr_ver_key_affine.y);

            // Generate the BLS ver key
            let key_pair = BLSKeyPair::generate(&mut rng);
            let vk = key_pair.ver_key();
            let vk_g2_affine: G2Affine = vk.to_affine();

            let vk_parsed: ParsedG2Point = vk_g2_affine.into();

            // Sign the ethereum address with the BLS key
            let sig: Signature = key_pair.sign(&sender_address_bytes, CS_ID_BLS_BN254);
            let sig_affine_point = sig.sigma.into_affine();
            let sig_parsed: ParsedG1Point = sig_affine_point.into();

            let res = (
                sig_parsed,
                vk_parsed,
                schnorr_pk_x,
                schnorr_pk_y,
                sender_address,
            );
            println!("{}", res.encode_hex());
        }
        Action::MockGenesis => {
            if cli.args.len() != 2 {
                panic!("Should provide arg1=numBlockPerEpoch,arg2=numInitValidators");
            }

            let block_per_epoch = cli.args[0].parse::<u32>().unwrap();
            let num_init_validators = cli.args[1].parse::<u64>().unwrap();
            let pp = MockSystemParam::init(block_per_epoch);
            let ledger = MockLedger::init(pp, num_init_validators as usize);

            let (voting_st_comm, frozen_st_comm) = ledger.get_stake_table_comms();
            let res = (ledger.get_state(), voting_st_comm, frozen_st_comm);
            println!("{}", res.encode_hex());
        }
        Action::MockConsecutiveFinalizedStates => {
            if cli.args.len() != 4 {
                panic!("Should provide arg1=numBlockPerEpoch,arg2=numInitValidators,arg3=numRegs,arg4=numExit");
            }

            let block_per_epoch = cli.args[0].parse::<u32>().unwrap();
            let num_init_validators = cli.args[1].parse::<u64>().unwrap();
            let num_reg = cli.args[2].parse::<u64>().unwrap();
            let num_exit = cli.args[3].parse::<u64>().unwrap();

            let pp = MockSystemParam::init(block_per_epoch);
            let mut ledger = MockLedger::init(pp, num_init_validators as usize);

            let mut new_states: Vec<ParsedLightClientState> = vec![];
            let mut proofs: Vec<ParsedPlonkProof> = vec![];
            for i in 1..block_per_epoch + 2 {
                // only update stake table at the last block, as it would only take effect in next epoch anyway.
                if i == block_per_epoch {
                    ledger.sync_stake_table(num_reg as usize, num_exit as usize);
                }

                // random number of notorized but not finalized block
                if ledger.rng.gen_bool(0.5) {
                    let num_non_blk = ledger.rng.gen_range(0..5);
                    for _ in 0..num_non_blk {
                        ledger.elapse_without_block();
                    }
                }

                ledger.elapse_with_block();

                let (pi, proof) = ledger.gen_state_proof();
                new_states.push(pi.into());
                proofs.push(proof.into());
            }

            let res = (new_states, proofs);
            println!("{}", res.encode_hex());
        }
        Action::MockSkipBlocks => {
            if cli.args.len() < 2 || cli.args.len() > 3 {
                panic!("Should provide arg1=numBlockPerEpoch,arg2=numBlockSkipped,arg3(opt)=requireValidProof");
            }

            let block_per_epoch = cli.args[0].parse::<u32>().unwrap();
            let num_block_skipped = cli.args[1].parse::<u32>().unwrap();
            let require_valid_proof: bool = if cli.args.len() == 3 {
                cli.args[2].parse::<bool>().unwrap()
            } else {
                true
            };

            let pp = MockSystemParam::init(block_per_epoch);
            let mut ledger = MockLedger::init(pp, STAKE_TABLE_CAPACITY / 2);

            // random stake table update
            ledger.sync_stake_table(4, 3);
            for _ in 0..num_block_skipped {
                ledger.elapse_with_block();
            }

            let res = if require_valid_proof {
                let (pi, proof) = ledger.gen_state_proof();
                let pi_parsed: ParsedLightClientState = pi.into();
                let proof_parsed: ParsedPlonkProof = proof.into();
                (pi_parsed, proof_parsed)
            } else {
                let pi_parsed = ledger.get_state();
                let proof_parsed = ParsedPlonkProof::dummy(&mut ledger.rng);
                (pi_parsed, proof_parsed)
            };
            println!("{}", res.encode_hex());
        }
        Action::MockMissEndingBlock => {
            if cli.args.len() != 1 {
                panic!("Should provide arg1=numBlockPerEpoch");
            }
            let block_per_epoch = cli.args[0].parse::<u32>().unwrap();

            let pp = MockSystemParam::init(block_per_epoch);
            let mut ledger = MockLedger::init(pp, STAKE_TABLE_CAPACITY / 2);

            let mut new_states: Vec<ParsedLightClientState> = vec![];
            let mut proofs: Vec<ParsedPlonkProof> = vec![];

            ledger.elapse_with_block();
            // first block in epoch 1
            let (pi, proof) = ledger.gen_state_proof();
            new_states.push(pi.into());
            proofs.push(proof.into());

            // skipping all remaining blocks in epoch 1, including the last/ending block
            for _ in 2..block_per_epoch + 1 {
                ledger.elapse_with_block();
            }
            // first block in epoch 2
            ledger.elapse_with_block();
            new_states.push(ledger.get_state());
            proofs.push(ParsedPlonkProof::dummy(&mut ledger.rng)); // we don't need correct proof here

            let res = (new_states, proofs);
            println!("{}", res.encode_hex());
        }
        Action::GenBLSHashes => {
            if cli.args.len() != 1 {
                panic!("Should provide arg1=message");
            }

            // Same as in the hash_to_curve function
            // See https://github.com/EspressoSystems/jellyfish/blob/6c2c08f4e966fd1d454d48bcf30bd41a952f9f76/primitives/src/signatures/bls_over_bn254.rs#L310
            let hasher_init = &[1u8];
            let hasher = <DefaultFieldHasher<Keccak256> as HashToField<Fq>>::new(hasher_init);

            let message_bytes = cli.args[0].parse::<Bytes>().unwrap();

            let field_elem: Fq = hasher.hash_to_field(&message_bytes, 1)[0];
            let fq_u256 = field_to_u256::<Fq>(field_elem);
            let hash_to_curve_elem: G1Affine = hash_to_curve::<Keccak256>(&message_bytes).into();
            let hash_to_curve_elem_parsed: ParsedG1Point = hash_to_curve_elem.into();

            let res = (fq_u256, hash_to_curve_elem_parsed);
            println!("{}", res.encode_hex());
        }
        Action::GenBLSSig => {
            let mut rng = jf_utils::test_rng();

            if cli.args.len() != 1 {
                panic!("Should provide arg1=message");
            }
            let message_bytes = cli.args[0].parse::<Bytes>().unwrap();

            // Generate the BLS ver key
            let key_pair = BLSKeyPair::generate(&mut rng);
            let vk = key_pair.ver_key();
            let vk_g2_affine: G2Affine = vk.to_affine();
            let vk_parsed: ParsedG2Point = vk_g2_affine.into();

            // Sign the message
            let sig: Signature = key_pair.sign(&message_bytes, CS_ID_BLS_BN254);
            let sig_affine_point = sig.sigma.into_affine();
            let sig_parsed: ParsedG1Point = sig_affine_point.into();

            let res = (vk_parsed, sig_parsed);
            println!("{}", res.encode_hex());
        }
    };
}
