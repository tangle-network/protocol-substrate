# protocol-substrate

## Get Started

### Prerequisites

This guide uses <https://rustup.rs> installer and the `rustup` tool to manage the Rust toolchain.
First install and configure `rustup`:

```bash
# Install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Configure
source ~/.cargo/env
```

Configure the Rust toolchain to default to the latest stable version, add nightly and the nightly wasm target:

```bash
rustup default stable
rustup update
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```

### Build

Clone the submodules using `git submodule update --init`. The `cargo run` command will perform an initial build. Use the following command to build the node
without launching it:

```bash
cargo build --release
```

### Troubleshooting for Apple Silicon users

Install Homebrew if you have not already. You can check if you have it installed with the following command:

```bash
brew help
```

If you do not have it installed open the Terminal application and execute the following commands:

```bash
# Install Homebrew if necessary https://brew.sh/
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install.sh)"

# Make sure Homebrew is up-to-date, install openssl
brew update
brew install openssl
```

❗ **Note:** Native ARM Homebrew installations are only going to be supported at `/opt/homebrew`. After Homebrew installs, make sure to add `/opt/homebrew/bin` to your PATH.

```bash
echo 'export PATH=/opt/homebrew/bin:$PATH' >> ~/.bash_profile
```

In order to build **protocol-substrate** in `--release` mode using `aarch64-apple-darwin` Rust toolchain you need to set the following environment variables:

```bash
echo 'export CC="/opt/homebrew/opt/llvm/bin/clang"' >> ~/.bash_profile
echo 'export AR="/opt/homebrew/opt/llvm/bin/llvm-ar"' >> ~/.bash_profile
```

# Standalone local testnets
In order to run the standalone development network, you should prepare your terminal environment for running 2 substrate nodes. Execute in either terminal the following commands. This will set up a development network using the BABE consensus mechanism for a 2 node network.

```jsx
./target/release/webb-standalone-node --dev --alice --node-key 0000000000000000000000000000000000000000000000000000000000000001
```

```jsx
./target/release/webb-standalone-node --dev --bob --port 33334 --tmp   --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
```
