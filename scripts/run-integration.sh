#!/usr/bin/env bash

# get the path for this file
BASEDIR=$(dirname "$0") 

start_node() {
   $BASEDIR/../scripts/run-standalone.sh > /dev/null 2>&1
}

run_tests() {
    # release mode
    # should find client/Cargo.toml no matter where its called from
    # should run test in synchronous way to avoid race conditions
    # output logs -- events from chain
    sleep 2
    cargo test --release -p webb-client -- --test-threads 1 --nocapture 
}

start_node & run_tests