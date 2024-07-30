# Espresso Dev Node

Espresso dev node is a node specifically designed for development and testing. It includes various nodes required to run
a complete Espresso network, such as `builder`, `sequencer`, etc. Developers can use it for development and testing.

## Download

We highly recommend you to use our Docker image. You can run it from the command line:

```cmd
docker run ghcr.io/espressosystems/espresso-sequencer/espresso-dev-node:main
```

## Parameters

| Name                            | Type            | Environment Variable                 | Default Value                                                 | Description                                                                                                                                  |
| ------------------------------- | --------------- | ------------------------------------ | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `rpc_url`                       | `Option<Url>`   | `ESPRESSO_SEQUENCER_L1_PROVIDER`     | Automatically launched Avil node if not provided.             | The JSON-RPC endpoint of the L1. If not provided, an Avil node will be launched automatically.                                               |
| `mnemonic`                      | `String`        | `ESPRESSO_SEQUENCER_ETH_MNEMONIC`    | `test test test test test test test test test test test junk` | Mnemonic for an L1 wallet. This wallet is used to deploy the contracts, so the account indicated by `ACCOUNT_INDEX` must be funded with ETH. |
| `account_index`                 | `u32`           | `ESPRESSO_DEPLOYER_ACCOUNT_INDEX`    | `0`                                                           | Account index of the L1 wallet generated from `MNEMONIC`. Used when deploying contracts.                                                     |
| `sequencer_api_port`            | `u16`           | `ESPRESSO_SEQUENCER_API_PORT`        | Required                                                      | Port that the HTTP API will use.                                                                                                             |
| `sequencer_api_max_connections` | `Option<usize>` | `ESPRESSO_SEQUENCER_MAX_CONNECTIONS` | None                                                          | Maximum concurrent connections allowed by the HTTP API server.                                                                               |
| `builder_port`                  | `Option<u16>`   | `ESPRESSO_BUILDER_PORT`              | An unused port                                                | Port for connecting to the builder.                                                                                                          |
| `prover_port`                   | `Option<u16>`   | `ESPRESSO_PROVER_PORT`               | An unused port                                                | Port for connecting to the prover. If this is not provided, an available port will be selected.                                              |
| `dev_node_port`                 | `u16`           | `ESPRESSO_DEV_NODE_PORT`             | `20000`                                                       | Port for the dev node. This is used to provide tools and information to facilitate developers debugging.                                     |

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
  "prover_port": 23156,
  "l1_url": "http://localhost:8545/",
  "light_client_address": "0xb075b82c7a23e0994df4793422a1f03dbcf9136f"
}
```

### POST /api/set-hotshot-down

This endpoint simulates the effect of a liveness failure of the hotshot consensus protocol in the Light Client smart
contract.

By calling this, the L1 height in the light contract will be frozen, and rollups will detect the HotShot failure. This
is intended for testing rollups' functionalities when HotShot is down.

An example of a `curl` command:

```cmd
curl -X POST "http://localhost:20000/api/set-hotshot-down" \
     -H "Content-Type: application/json" \
     -d '{"height": 12345}'
```

Parameters

| Name   | Type    | Description                              |
| ------ | ------- | ---------------------------------------- |
| height | integer | The L1 height from which hotshot is down |

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
