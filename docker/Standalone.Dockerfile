FROM rust:1 as builder
WORKDIR /webb

# Install Required Packages
RUN apt-get update && \
  apt-get install -y git pkg-config clang curl libssl-dev llvm libudev-dev libgmp3-dev protobuf-compiler && \
  rm -rf /var/lib/apt/lists/*
COPY . .

# Build Standalone Node.
RUN git submodule update --init && \
  cargo build --release -p webb-standalone-node

# This is the 2nd stage: a very small image where we copy the Node binary."

FROM ubuntu:20.04

COPY --from=builder /webb/target/release/webb-standalone-node /usr/local/bin

RUN apt-get update && apt-get install -y clang libssl-dev llvm libudev-dev libgmp3-dev && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 -U -s /bin/sh -d /webb webb && \
  mkdir -p /data /webb/.local/share/webb && \
  chown -R webb:webb /data && \
  ln -s /data /webb/.local/share/webb && \
  # Sanity checks
  ldd /usr/local/bin/webb-standalone-node && \
  /usr/local/bin/webb-standalone-node --version

USER webb
EXPOSE 30333 9933 9944 9615 33334
VOLUME ["/data"]
