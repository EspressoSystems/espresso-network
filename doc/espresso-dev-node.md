# Espresso Dev Node

Espresso dev node is a node specifically designed for development and testing. It includes various nodes required to run
a complete Espresso network, such as `builder`, `sequencer`, `state_prover` etc. Additionally, it supports deploying
light client contracts on alternative chains and running a separate prover for each which can be useful for testing
involving L3s. Developers can use dev node for development and testing.

## Running the Dev Node

We highly recommend you to use our Docker image. You can run it from the command line:

```cmd
docker run -p 8770:8770 ghcr.io/espressosystems/espresso-sequencer/espresso-dev-node:main
```

This command will run the minimal setup of the dev node with the default options. It will run an API server on port
8770, which you can use to interact with the dev node in exactly the same way you would connect to a node in the real
Espresso network. For example, hit http://localhost:8770/v0/status/metrics to check the status of the node and use
http://localhost:8770/v0/availability to query blockchain data.

## Parameters

While the command above is sufficient to run the dev node with the default settings, there are many options for
customizing the node, depending on what type of testing you are trying to do.

| Name                            | Type            | Environment Variable                         | Default Value                                                 | Description                                                                                                                                                                                    |
| ------------------------------- | --------------- | -------------------------------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rpc_url`                       | `Option<Url>`   | `ESPRESSO_SEQUENCER_L1_PROVIDER`             | Automatically launched Anvil node if not provided.            | The JSON-RPC endpoint of the L1. If not provided, an Anvil node will be launched automatically.                                                                                                |
| `mnemonic`                      | `String`        | `ESPRESSO_SEQUENCER_ETH_MNEMONIC`            | `test test test test test test test test test test test junk` | Mnemonic for an L1 wallet. This wallet is used to deploy the contracts, so the account indicated by`ACCOUNT_INDEX` must be funded with ETH.                                                    |
| `account_index`                 | `u32`           | `ESPRESSO_DEPLOYER_ACCOUNT_INDEX`            | `0`                                                           | Account index of the L1 wallet generated from`MNEMONIC`. Used when deploying contracts.                                                                                                        |
| `sequencer_api_port`            | `u16`           | `ESPRESSO_SEQUENCER_API_PORT`                | Required                                                      | Port that the HTTP API will use.                                                                                                                                                               |
| `sequencer_api_max_connections` | `Option<usize>` | `ESPRESSO_SEQUENCER_MAX_CONNECTIONS`         | None                                                          | Maximum concurrent connections allowed by the HTTP API server.                                                                                                                                 |
| `builder_port`                  | `Option<u16>`   | `ESPRESSO_BUILDER_PORT`                      | An unused port                                                | Port for connecting to the builder.                                                                                                                                                            |
| `prover_port`                   | `Option<u16>`   | `ESPRESSO_PROVER_PORT`                       | An unused port                                                | Port for connecting to the prover. If this is not provided, an available port will be selected.                                                                                                |
| `dev_node_port`                 | `u16`           | `ESPRESSO_DEV_NODE_PORT`                     | `20000`                                                       | Port for the dev node. This is used to provide tools and information to facilitate developers debugging.                                                                                       |
| `update_interval`               | `Duration`      | `ESPRESSO_STATE_PROVER_UPDATE_INTERVAL`      | `20s`                                                         | The frequency of updating the light client state                                                                                                                                               |
| `retry_interval`                | `Duration`      | `ESPRESSO_STATE_PROVER_RETRY_INTERVAL`       | `2s`                                                          | Interval between retries if a state update fails.                                                                                                                                              |
| `alt_chain_providers`           | `Vec<Url>`      | `ESPRESSO_DEPLOYER_ALT_CHAIN_PROVIDERS`      | `None`                                                        | Optional list of URLs representing alternate chains where the dev node will deploy Light client contracts and submit Light contract state updates. Useful for test environments involving L3s. |
| `alt_mnemonics`                 | `Vec<String>`   | `ESPRESSO_DEPLOYER_ALT_MNEMONICS`            | `None`                                                        | Optional list of mnemonics for the alternate chains. If there are fewer mnemonics provided than chains, the base `MNEMONIC` will be used.                                                      |
| `alt_account_indices`           | `Vec<u32>`      | `ESPRESSO_SEQUENCER_DEPLOYER_ALT_INDICES`    | `None`                                                        | Optional list of account indices to use when deploying the contracts. If there are fewer indices provided than chains, the base ACCOUNT_INDEX will be used.                                    |
| `alt_prover_update_intervals`   | `Vec<Duration>` | `ESPRESSO_STATE_PROVER_ALT_UPDATE_INTERVALS` | `None`                                                        | The frequency of updating the light client state for alternate chains. If there are fewer provided than chains, the base update_interval will be used.                                         |
| `alt_prover_retry_intervals`    | `Vec<Duration>` | `ESPRESSO_STATE_PROVER_ALT_RETRY_INTERVALS`  | `None`                                                        | Interval between retries if a state update fails for alternate chains. If there are fewer intervals provided than chains, the base retry_interval will be used.                                |

## APIs

Once you have successfully run the dev node, you can access the corresponding ports to call the APIs of the
[`builder`](https://docs.espressosys.com/sequencer/api-reference/builder-api),
[`sequencer`](https://docs.espressosys.com/sequencer/api-reference/sequencer-api), and `prover`.

In addition, you can access the `dev_node_port` to retrieve debugging information. Here are the details of the dev node
API.

### GET /api/dev-info

This endpoint returns some debug information for you.

An example response is like this:

```json
{
  "builder_url": "http://localhost:41003/",
  "l1_prover_port": 23156,
  "l1_url": "http://localhost:8545/",
  "l1_light_client_address": "0xb075b82c7a23e0994df4793422a1f03dbcf9136f",
  "alt_chains": [
    {
      "chain_id": 9,
      "provider_url": "http://localhost:8546/",
      "light_client_address": "0xa1b2c3d4e5f678901234567890abcdef12345678",
      "prover_port": 23157
    },
    {
      "chain_id": 10,
      "provider_url": "http://localhost:8547/",
      "light_client_address": "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdef",
      "prover_port": 23158
    }
  ]
}
```

### POST /api/set-hotshot-down

This endpoint simulates the effect of a liveness failure of the hotshot consensus protocol in the Light Client smart
contract.

By calling this, the height in the light contract will be frozen, and rollups will detect the HotShot failure. This is
intended for testing rollups' functionalities when HotShot is down.

An example of a `curl` command:

```cmd
curl -X POST "http://localhost:20000/api/set-hotshot-down" \
     -H "Content-Type: application/json" \
     -d '{"height": 12345}'
```

Parameters

| Name     | Type    | Required | Description                                                                                                   |
| -------- | ------- | -------- | ------------------------------------------------------------------------------------------------------------- |
| chain_id | integer | No       | chain id for which the height needs to be frozen. If not provided, the base L1 light client contract is used. |
| height   | integer | Yes      | The height from which hotshot is down                                                                         |

### POST /api/set-hotshot-up

This endpoint simulates the effect of a liveness success of the hotshot consensus protocol in the Light Client smart
contract.

This is intended to be used when `set-hotshot-down` has been called previously. By calling this, rollups will detect the
reactivity of HotShot.

An example of a `curl` command:

```cmd
curl -X POST "http://localhost:20000/api/set-hotshot-up" \
     -H "Content-Type: application/json"
```

Parameter

| Name     | Type    | Required | Description                                                                                                          |
| -------- | ------- | -------- | -------------------------------------------------------------------------------------------------------------------- |
| chain_id | integer | No       | chain id for which the height needs to be unfrozen. If not provided, the base L1 light client contract will be used. |
