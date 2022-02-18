#!/usr/bin/env bash

# get the path for this
BASEDIR=$(dirname "$0") 

# release mode
# should find client/Cargo.toml no matter where its called from
# should run test in synchronous way to avoid race conditions
# output logs -- events from chain
cargo test --release --manifest-path $BASEDIR/../client/Cargo.toml -- --test-threads 1 --nocapture 