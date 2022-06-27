<h1 align="center">Anchor Protocol - Substrate 🕸️ </h1>
<div align="center">
<a href="https://www.webb.tools/">
    <img alt="Webb Logo" src="./assets/webb-icon.svg" width="15%" height="30%" />
  </a>
  </div>
<p align="center">
    <strong>🚀 Webb's Substrate Pallet Implementation 🚀</strong>
    <br />
    <sub> ⚠️ Beta Software ⚠️ </sub>
</p>

<div align="center" >

[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/webb-tools/protocol-substrate/Build%20&%20Test?style=flat-square)](https://github.com/webb-tools/protocol-substrate/actions)
[![Codecov](https://img.shields.io/codecov/c/gh/webb-tools/protocol-substrate?style=flat-square&token=A4WGU76TWU)](https://codecov.io/gh/webb-tools/protocol-substrate)
[![License Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=flat-square)](https://opensource.org/licenses/Apache-2.0)
[![Twitter](https://img.shields.io/twitter/follow/webbprotocol.svg?style=flat-square&label=Twitter&color=1DA1F2)](https://twitter.com/webbprotocol)
[![Telegram](https://img.shields.io/badge/Telegram-gray?logo=telegram)](https://t.me/webbprotocol)
[![Discord](https://img.shields.io/discord/833784453251596298.svg?style=flat-square&label=Discord&logo=discord)](https://discord.gg/cv8EfJu3Tn)

</div>

<!-- TABLE OF CONTENTS -->
<h2 id="table-of-contents"> 📖 Table of Contents</h2>

<details open="open">
  <summary>Table of Contents</summary>
  <ul>
    <li><a href="#start"> Getting Started</a></li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#test">Testing</a></li>
</details>

<h2 id="start"> Getting Started  🎉 </h2>

For additional information, please refer to the [Webb Protocol-Substrate Rust Docs](https://webb-tools.github.io/protocol-substrate/) 📝. Have feedback on how to improve protocol-substrate? Or have a specific question to ask? Checkout the [Anchor Protocol Feedback Discussion](https://github.com/webb-tools/feedback/discussions/categories/anchor-protocol) 💬.

### Pallet layout

```
pallets/
  |____anchor/              # A simple module for building Anchors.
  |____anchor-handler/      # A module for executing the creation and modification of anchors.
  |____linkable-tree/       # A module for constructing, modifying and inspecting linkable trees.
  |____hasher/              # A module for abstracting over arbitrary hash functions primarily for zero-knowledge friendly hash functions that have potentially large parameters to deal with.
  |____mixer/               # A simple module for building Mixers. 
  |____signature-bridge/    # A module for managing voting, resource, and maintainer composition through signature verification.
  |____token-wrapper/       # A module for wrapping pooled assets and minting pool share tokens
  |____vanchor/             # A simple module for building variable Anchors.  
  |____verifier/            # A module for abstracting over arbitrary zero-knowledge verifiers for arbitrary zero-knowledge gadgets
  |____xanchor/             # A module for managing the linking process between anchors.
```
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
rustup default nightly
rustup update
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```

Great! Now your Rust environment is ready! 🚀🚀

### Installation 💻

Clone the submodules:

```bash
# clone the repo
git clone git@github.com:webb-tools/protocol-substrate.git

# Fetch submodules
git submodule update --init
```

Build the node in `release mode`:

```bash
cargo build --release
```

#### Troubleshooting for Apple Silicon users

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

<h2 id="usage"> Usage </h2>

### Quick Start ⚡

#### Standalone Local Testnet

In order to run the standalone development network, you will need to prepare 2 terminal windows. Once the below commands are executed it will set up a development network using the BABE consensus mechanism for a 2 node network.

**Terminal 1:**

```jsx
./target/release/webb-standalone-node --dev --alice --node-key 0000000000000000000000000000000000000000000000000000000000000001
```

**Terminal 2:**

```jsx
./target/release/webb-standalone-node --dev --bob --port 33334 --tmp   --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
```

You now have successfully set up a standalone local testnet! 🎉 

### Docker 🐳

To build the [standalone](./docker/Standalone.Dockerfile) docker image specified in `docker/` :

```sh
docker build -f ./docker/Standalone.Dockerfile -t protocol-substrate/standalone .
```

To run docker image:

```sh
docker run protocol-substrate/standalone
```

<h2 id="test"> Testing 🧪 </h2>

The following instructions outlines how to run the protocol-substrate base test suite and integration test suite.

### To run base tests

```
cargo test --release --workspace --exclude webb-client
```
### To run integration tests

1. Run `cd scripts`
2. Run `sh run-integrations.sh`
### Code Coverage

You need to have docker installed to generate code coverage. 

> Build docker image:

```sh
docker build -t cov -f docker/Coverage.Dockerfile .
```
> Run docker image and generate code coverage reports:

```sh
docker run --security-opt seccomp=unconfined cov
```

### Benchmarks

To generate benchmarks for a pallet run

```
cargo b --release --features runtime-benchmarks -p webb-standalone-node

./target/release/webb-standalone-node benchmark pallet \
--chain=dev \
--steps=20 \
--repeat=10 \
--log=warn \
--pallet=<pallet_name> \
--extrinsic="*" \
--execution=wasm \
--wasm-execution=compiled \
--output=./pallets/signature-bridge/src/weights.rs \
--template=./.maintain/webb-weight-template.hbs
```

## Contributing

Interested in contributing to protocol-substrate? Thank you so much for your interest! We are always appreciative for contributions from the open-source community!  

If you have a contribution in mind, please check out our [Contribution Guide](./.github/CONTRIBUTING.md) for information on how to do so. We are excited for your first contribution!

## License

Licensed under <a href="LICENSE">Apache 2.0 license</a>.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache 2.0 license, shall be licensed as above, without any additional terms or conditions.

## Supported by

<br />
<p align="center">
 <img src="./assets/w3f.jpeg" width="30%" height="60%" >
</p>