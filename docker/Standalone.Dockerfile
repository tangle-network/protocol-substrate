FROM rust:1 as builder
WORKDIR /webb

# Install Required Packages
RUN apt-get update && \
  apt-get install -y git pkg-config clang curl libssl-dev llvm libudev-dev libgmp3-dev && \
  rm -rf /var/lib/apt/lists/* && \
  NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" && \
  brew install mold

COPY . .

# Build Standalone Node (with mold linker)
RUN mold -run cargo build --release -p darkwebb-standalone-node

# This is the 2nd stage: a very small image where we copy the Node binary."

FROM ubuntu:20.04

COPY --from=builder /webb/target/release/darkwebb-standalone-node /usr/local/bin

RUN apt-get update && apt-get install -y clang libssl-dev llvm libudev-dev libgmp3-dev && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 -U -s /bin/sh -d /webb webb && \
  mkdir -p /data /webb/.local/share/webb && \
  chown -R webb:webb /data && \
  ln -s /data /webb/.local/share/webb && \
  # Sanity checks
  ldd /usr/local/bin/darkwebb-standalone-node && \
  /usr/local/bin/darkwebb-standalone-node --version

USER webb
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]
