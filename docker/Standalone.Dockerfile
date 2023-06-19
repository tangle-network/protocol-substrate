# Use a specific version tag for the alpine base image
FROM rust:1 AS base

# Install required packages
RUN apt-get update && \
    apt-get install --yes git python3 python3-pip pkg-config clang curl libssl-dev llvm libudev-dev libgmp3-dev protobuf-compiler libc6 && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user to run
RUN adduser --uid 1000 --ingroup users --disabled-password --gecos "" --home /webb webb \
  && mkdir -p /data /webb/.local/share/webb \
  && chown -R webb:users /data /webb/.local/share/webb \
  && ln -s /data /webb/.local/share/webb


# Set the user and working directory
USER webb
WORKDIR /webb

# Use a multi-stage build to reduce the size of the final image
FROM rust:1 AS builder

# Install required packages
RUN apt-get update && \
    apt-get install -y git python3 python3-pip pkg-config clang curl libssl-dev llvm libudev-dev libgmp3-dev protobuf-compiler libc6 && \
    rm -rf /var/lib/apt/lists/*

RUN pip3 install dvc

# Copy the source code into the container
WORKDIR /webb
COPY . .

# Use "RUN" instructions to combine multiple commands into a single layer
RUN dvc pull -v \
  && RUST_BACKTRACE=1 cargo build --release -p webb-standalone-node --verbose

# Use the final stage to reduce the size of the final image
FROM base

# Create the /data directory and set permissions
USER root
RUN mkdir -p /data \
  && chown webb:users /data
USER webb

# Copy the binary into the final image
COPY --from=builder /webb/target/release/webb-standalone-node /usr/local/bin

# Expose ports and volume
EXPOSE 30333 9933 9944 9615 33334
VOLUME ["/data"]

# Set the user and working directory
USER webb
WORKDIR /webb

# Sanity check
CMD ["/usr/local/bin/webb-standalone-node", "--version"]
