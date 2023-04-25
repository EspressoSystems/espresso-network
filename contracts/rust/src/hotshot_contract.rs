#[cfg(test)]

mod test {
    use crate::helpers::hotshot_contract::get_provider_and_deployer;
    use crate::helpers::{MyG1Point, MyG2Point};
    use ark_ec::CurveGroup;
    use contract_bindings::bls_test::G2Point;
    use contract_bindings::hot_shot::NewBlocksCall;
    use contract_bindings::HotShot;
    use ethers::middleware::SignerMiddleware;
    use ethers::signers::Wallet;
    use ethers::types::Bytes;
    use ethers::{abi::AbiDecode, providers::Middleware, types::U256};
    use ethers_providers::Http;
    use jf_primitives::signatures::bls_over_bn254::{
        BLSOverBN254CurveSignatureScheme, SignKey, Signature, VerKey,
    };
    use jf_primitives::signatures::{AggregateableSignatureSchemes, SignatureScheme};
    use jf_utils::test_rng;

    #[async_std::test]
    async fn test_hotshot_block_commitment() {
        let (provider, deployer) = get_provider_and_deployer().await;
        let hotshot = HotShot::deploy(deployer.clone(), ())
            .unwrap()
            .send()
            .await
            .unwrap();

        let block_num = U256::from(0);
        let commitment = U256::from(1234);

        hotshot
            .new_blocks(vec![commitment], vec![vec![1, 2, 3].into()])
            .send()
            .await
            .unwrap()
            .await
            .unwrap();

        assert_eq!(
            hotshot.commitments(block_num).call().await.unwrap(),
            commitment,
        );

        let (event, meta) = &hotshot
            .new_blocks_filter()
            .from_block(0)
            .query_with_meta()
            .await
            .unwrap()[0];

        assert_eq!(event.first_block_number, block_num);
        assert_eq!(event.num_blocks, 1.into());

        // Parse the commitments from calldata.
        let tx = provider
            .get_transaction(meta.transaction_hash)
            .await
            .unwrap()
            .unwrap();
        let call = NewBlocksCall::decode(&tx.input).unwrap();
        assert_eq!(call.new_commitments, vec![commitment]);
    }

    #[async_std::test]
    async fn test_hotshot_stake_table() {
        let (_, deployer) = get_provider_and_deployer().await;
        let hotshot = HotShot::deploy(deployer.clone(), ())
            .unwrap()
            .send()
            .await
            .unwrap();

        let rng = &mut test_rng();
        for i in 0..5 {
            let (_, pk) = BLSOverBN254CurveSignatureScheme::key_gen(&(), rng).unwrap();
            let pk = pk.to_affine();
            let pk_value: MyG2Point = pk.into();
            let amount = U256::from(10 * i);
            hotshot
                .add_new_staking_key(pk_value.clone().into(), amount)
                .send()
                .await
                .unwrap()
                .await
                .unwrap();

            let expected_stake_and_amount = (G2Point::from(pk_value), amount);
            assert_eq!(
                hotshot.get_staking_key(U256::from(i)).call().await.unwrap(),
                expected_stake_and_amount,
            );
        }
    }

    fn get_slice_from_bitmap<T: Clone>(input: Vec<T>, bitmap: &Vec<U256>) -> Vec<T> {
        assert_eq!(input.len(), bitmap.len());
        let mut res = vec![];
        for i in 0..input.len() {
            if bitmap[i] == U256::from(1) {
                res.push(input[i].clone());
            }
        }
        res
    }

    async fn check_qc_with_bitmap(
        bitmap: &Vec<U256>,
        staking_keys: &[(SignKey, VerKey)],
        hotshot: &HotShot<
            SignerMiddleware<
                ethers::providers::Provider<Http>,
                Wallet<ethers::core::k256::ecdsa::SigningKey>,
            >,
        >,
    ) {
        // Compute a signature with 3 keys holding enough staking in total
        let n_sigs = bitmap.len();
        let rng = &mut test_rng();
        let mut signatures: Vec<Signature> = vec![];
        let message = Bytes::from(b"unique message");
        for key_pair in staking_keys.iter().take(n_sigs) {
            let sk = key_pair.0.clone();
            let sig = BLSOverBN254CurveSignatureScheme::sign(&(), &sk, &message, rng).unwrap();
            signatures.push(sig.clone());
        }

        let vks = get_slice_from_bitmap::<VerKey>(
            staking_keys.iter().map(|k| k.1).collect::<Vec<VerKey>>(),
            bitmap,
        );
        let sigs = get_slice_from_bitmap::<Signature>(signatures, bitmap);
        let agg_sig = BLSOverBN254CurveSignatureScheme::aggregate(&(), &vks, &sigs).unwrap();
        let agg_sig_value: MyG1Point = agg_sig.clone().sigma.into_affine().into();

        // Call the contract
        let res = hotshot
            .verify_agg_sig(message.clone(), agg_sig_value.into(), bitmap.to_vec())
            .call()
            .await;

        assert!(res.is_ok());
    }

    #[async_std::test]
    async fn test_validate_qc() {
        let (_, deployer) = get_provider_and_deployer().await;
        let hotshot = HotShot::deploy(deployer.clone(), ())
            .unwrap()
            .send()
            .await
            .unwrap();

        // Initialize the staking table with 5 keys
        let mut staking_keys = vec![];
        let rng = &mut test_rng();
        let n_sigs = 5;
        for i in 0..n_sigs {
            let (sk, pk) = BLSOverBN254CurveSignatureScheme::key_gen(&(), rng).unwrap();
            staking_keys.push((sk.clone(), pk));
            let pk = pk.to_affine();
            let pk_value: MyG2Point = pk.into();
            let amount = U256::from(10 * i);
            hotshot
                .add_new_staking_key(pk_value.clone().into(), amount)
                .send()
                .await
                .unwrap()
                .await
                .unwrap();
        }

        check_qc_with_bitmap(
            &vec![
                U256::from(1),
                U256::from(1),
                U256::from(1),
                U256::from(0),
                U256::from(0),
            ],
            &staking_keys,
            &hotshot,
        )
        .await;

        check_qc_with_bitmap(
            &vec![
                U256::from(0),
                U256::from(1),
                U256::from(1),
                U256::from(0),
                U256::from(1),
            ],
            &staking_keys,
            &hotshot,
        )
        .await;

        check_qc_with_bitmap(
            &vec![
                U256::from(1),
                U256::from(0),
                U256::from(0),
                U256::from(0),
                U256::from(0),
            ],
            &staking_keys,
            &hotshot,
        )
        .await;

        // TODO error cases
        // Threshold is too low
        // Signature is invalid
        // Bitmap is not correct
    }
}
