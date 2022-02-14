FROM rust:1
WORKDIR /webb

# Install Required Packages
RUN apt-get update && \
    apt-get install -y git pkg-config clang curl libssl-dev llvm libudev-dev libgmp3-dev && \
    rm -rf /var/lib/apt/lists/*

COPY . .

RUN rustup default nightly

RUN cargo install cargo-tarpaulin

# Build Standalone Node.
CMD git submodule update --init && \
    SKIP_WASM_BUILD=1 cargo +nightly tarpaulin --out Xml \
        -p webb-standalone-runtime \
        -p pallet-token-wrapper-handler \
        -p pallet-token-wrapper \
        -p pallet-anchor-handler \
        # -p pallet-vanchor \
        -p pallet-signature-bridge \
        -p pallet-anchor \
        -p pallet-mixer \
        -p pallet-linkable-tree \
        -p pallet-mt \
        -p pallet-verifier \
        -p pallet-hasher \
        -p pallet-asset-registry \
        --timeout 3600
