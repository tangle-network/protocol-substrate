# darkwebb-substrate
### Build

The `cargo run` command will perform an initial build. Use the following command to build the node
without launching it:

```sh
cargo build --release
```

# Standalone local testnets
In order to run the standalone development network, you should prepare your terminal environment for running 2 substrate nodes. Execute in either terminal the following commands. This will set up a development network using the BABE consensus mechanism for a 2 node network.

```jsx
./target/release/darkwebb-standalone-node --dev --alice --node-key 0000000000000000000000000000000000000000000000000000000000000001
```

```jsx
./target/release/darkwebb-standalone-node --dev --bob --port 33334 --tmp   --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
```
