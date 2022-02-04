## Webb Parachain Local Setup.

To set up a local development for testing parachains setup, follow the following steps:

### Step 1: Setting up the Relay Chain(s)

We will use [Polkadot](https://github.com/paritytech/polkadot) as our Relay Chain

1. Clone Polkadot node locally:

```shell
git clone https://github.com/paritytech/polkadot.git
```
2. Build Release version of Polkadot node:

```shell
cargo build --release
```
> You should get something to drink while it builds (should take around 30min).

3. Make sure it works:

```shell
./target/release/polkadot --help
```

If it works, Good!

### Step 2: Setting up The Parachains

In another directory, do the following:

1. After cloning this repo, switch to this branch `parachain-kickstart`.

```shell
git checkout parachain-kickstart
```

2. Start build the webb node.

```shell
cargo build --release
```

3. Make sure it works:

```shell
./target/release/webb-node --help
```

### Step 3: Put together

In another directory, that is besides the `polkadot` and `webb-substrate`.

1. Create a new `polkadot-config`

```shell
mkdir -p polkadot-config && cd polkadot-config
```

2. install `polkadot-launch`

```shell
yarn global add polkadot-launch
```

3. Add the following snippet to `config.json`

```json
{
  "relaychain": {
    "bin": "../polkadot/target/release/polkadot",
    "chain": "rococo-local",
    "nodes": [
      {
        "name": "alice",
        "wsPort": 9944,
        "port": 19944
      },
      {
        "name": "bob",
        "wsPort": 9955,
        "port": 19955
      },
      {
        "name": "charlie",
        "wsPort": 9966,
        "port": 19966
      }
    ],
    "genesis": {
      "runtime": {
        "runtime_genesis_config": {
          "configuration": {
            "config": {
              "validation_upgrade_frequency": 1,
              "validation_upgrade_delay": 1
            }
          }
        }
      }
    }
  },
  "parachains": [
    {
      "bin": "../webb-substrate/target/release/webb-node",
      "id": "2000",
      "balance": "1000000000000000000000",
      "nodes": [
        {
          "wsPort": 9977,
          "port": 12000,
          "name": "alice",
          "flags": ["--force-authoring", "--", "--execution=wasm"]
        }
      ]
    },
    {
      "bin": "../webb-substrate/target/release/webb-node",
      "id": "3000",
      "balance": "1000000000000000000000",
      "nodes": [
        {
          "wsPort": 9988,
          "port": 13000,
          "name": "bob",
          "flags": ["--force-authoring", "--", "--execution=wasm"]
        }
      ]
    }
  ],
  "simpleParachains": [],
  "hrmpChannels": [
    {
      "sender": 2000,
      "recipient": 3000,
      "maxCapacity": 8,
      "maxMessageSize": 512
    },
    {
      "sender": 3000,
      "recipient": 2000,
      "maxCapacity": 8,
      "maxMessageSize": 512
    }
  ],
  "types": {},
  "finalization": false
}

```

Save the file, and move to the last step.

### Step 4: Kickstart Everything

Just run

```shell
polkadot-launch config.json
```

You should get output as the following:

```log

🧹 Resolving parachain id...
2021-10-20 16:28:06 Building chain spec    

🧹 Starting with a fresh authority set...
  👤 Added Genesis Authority alice
  👤 Added Genesis Authority bob
  👤 Added Genesis Authority charlie

⚙ Updating Relay Chain Genesis Configuration
  ✓ Updated Genesis Configuration [ validation_upgrade_frequency: 1 ]
  ✓ Updated Genesis Configuration [ validation_upgrade_delay: 1 ]

⛓ Adding Genesis Parachains
  ✓ Added Genesis Parachain 2000
  ✓ Added Genesis Parachain 3000
⛓ Adding Genesis HRMP Channels
  ✓ Added HRMP channel 2000 -> 3000
  ✓ Added HRMP channel 3000 -> 2000

2021-10-20 16:28:21 Building chain spec    
2021-10-20 16:28:29 Took active validators from set with wrong size    
2021-10-20 16:28:29 Took active validators from set with wrong size    
Starting alice...
Starting bob...
Starting charlie...
2021-10-20 16:28:29          API-WS: disconnected from ws://127.0.0.1:9944: 1006:: connection failed
2021-10-20 16:28:31          API-WS: disconnected from ws://127.0.0.1:9944: 1006:: connection failed
2021-10-20 16:28:34          API-WS: disconnected from ws://127.0.0.1:9944: 1006:: connection failed
2021-10-20 16:28:36          API-WS: disconnected from ws://127.0.0.1:9944: 1006:: connection failed
2021-10-20 16:28:39          API-WS: disconnected from ws://127.0.0.1:9944: 1006:: connection failed
2021-10-20 16:28:41          API-WS: disconnected from ws://127.0.0.1:9944: 1006:: connection failed
Starting a Collator for parachain 2000: 5Ec4AhPUwPeyTFyuhGuBbD224mY85LKLMSqSSo33JYWCazU4, Collator port : 12000 wsPort : 9977
Added --alice
Added --parachain-id=2000
Added --force-authoring to parachain
Added --execution=wasm to collator
--- Submitting extrinsic to set balance of 5Ec4AhPUwPeyTFyuhGuBbD224mY85LKLMSqSSo33JYWCazU4 to 1000000000000000000000. (nonce: 0) ---
Current status is Ready
Current status is {"broadcast":["12D3KooWH1KqWgZnxEopupAPKm7wVH7JpxKtuGU2UTGdkef5aLkG","12D3KooWFNZrEV8ppRN6zx6oTiwakHtkBrGyna71RtC8K2aHepAh","12D3KooWAEQzjbyMnD7mFEuErHJ8vu1vF32gm5kb3c6V2cMAkt47"]}
Current status is {"inBlock":"0x56b378a83881ebe0c3fd5d500e94050b882134565cef441839848b7a45af0e5d"}
Transaction included at blockHash 0x56b378a83881ebe0c3fd5d500e94050b882134565cef441839848b7a45af0e5d
Starting a Collator for parachain 3000: 5Ec4AhPQ8esUzs1gsg6DiUp9q87vJgFNHhPmrXwYiU9FFR7H, Collator port : 13000 wsPort : 9988
Added --alice
Added --parachain-id=3000
Added --force-authoring to parachain
Added --execution=wasm to collator
--- Submitting extrinsic to set balance of 5Ec4AhPQ8esUzs1gsg6DiUp9q87vJgFNHhPmrXwYiU9FFR7H to 1000000000000000000000. (nonce: 1) ---
Current status is Ready
Current status is {"broadcast":["12D3KooWH1KqWgZnxEopupAPKm7wVH7JpxKtuGU2UTGdkef5aLkG","12D3KooWFNZrEV8ppRN6zx6oTiwakHtkBrGyna71RtC8K2aHepAh","12D3KooWAEQzjbyMnD7mFEuErHJ8vu1vF32gm5kb3c6V2cMAkt47","12D3KooWM7fszAJQKNXR6CAtbesRmS29z9X3fjNZf1DhS3BbJrkV"]}
Current status is {"inBlock":"0x07f87b9d0db2dc0ddd6b9c7a75c6a11096ca49ed3efc7bc7494c550ae7e95c3a"}
Transaction included at blockHash 0x07f87b9d0db2dc0ddd6b9c7a75c6a11096ca49ed3efc7bc7494c550ae7e95c3a
🚀 POLKADOT LAUNCH COMPLETE 🚀
```

Next is to open the following Polkadotjs UI to see it working

1. [Parachain 1](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2Flocalhost%3A9977#/explorer)
2. [Parachain 2](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2Flocalhost%3A9988#/explorer)
3. [Relay Chain](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2Flocalhost%3A9944#/explorer)


## Resources

**Parachains & Parathreads**

1. [Parachain Basics](https://wiki.polkadot.network/docs/learn-parachains)
2. [Parathreads](https://wiki.polkadot.network/docs/learn-parathreads)
3. [The Path of a Parachain Block](https://polkadot.network/blog/the-path-of-a-parachain-block/)
4. [Parachain Development Overview](https://wiki.polkadot.network/docs/build-build-with-polkadot)

**Cumulus Tutorials**

1. [Start a Relay Chain](https://docs.substrate.io/tutorials/v3/cumulus/start-relay/)
2. [Connect a Parachain](https://docs.substrate.io/tutorials/v3/cumulus/connect-parachain/)
3. [Launch a Parachain Testnet](https://docs.substrate.io/tutorials/v3/cumulus/polkadot-launch/)
