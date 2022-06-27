# Dapp Example

This repository demonstrates how to call the Prosopo [Protocol](https://github.com/prosopo-io/protocol) contract from within a second contract. The easiest way to use this contract in a development environment is via the [integration repository](https://github.com/prosopo-io/integration). Manual build and deploy instructions are included below.

## Prerequisites

### Deploy the Prosopo contract

Follow instructions in Prosopo [Protocol](https://github.com/prosopo-io/protocol)

### Build and deploy the dapp-example contract with docker

Configure the docker file so that the [substrate endpoint](https://github.com/prosopo-io/dapp-example/blob/859ed5088bd77273819023823e6a0c5fb241f0b3/docker/contracts.dapp.dockerfile#L9) is valid. Then run the following command to build and deploy the contract.

```bash
CONTRACT_ADDRESS=5CCfRe5TxkUVMDMznbGs4wpxeWnUK8hC6dqQ7bZZtao6RFiH docker compose --file docker-compose.dapp.yml up dapp-build
```

### Build and deploy the contract locally

After installing all [substrate pre-requisites](https://docs.substrate.io/main-docs/install/), in the contracts folder run:

```bash
cargo +nightly contract build
```

Then deploy the contract to a substrate node after populating the various arguments. For examples, of these arguments, please see the [docker file](https://github.com/prosopo-io/dapp-example/blob/develop/docker/contracts.dapp.dockerfile).

#### Command Line Deploy

```bash
$CONTRACT_ARGS = "$DAPP_CONTRACT_ARGS_INITIAL_SUPPLY $DAPP_CONTRACT_ARGS_FAUCET_AMOUNT $CONTRACT_ADDRESS $DAPP_CONTRACT_ARGS_HUMAN_THRESHOLD $DAPP_CONTRACT_ARGS_RECENCY_THRESHOLD"
cargo contract instantiate $WASM --args $CONTRACT_ARGS --constructor $CONSTRUCTOR --suri $SURI --value $ENDOWMENT --url '$ENDPOINT:$PORT' --gas 500000000000

```

#### UI deploy

Use [polkadot apps](https://polkadot.js.org/apps/) contract page.


