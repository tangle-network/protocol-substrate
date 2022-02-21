#!/usr/bin/env bash

# get the path for this file
BASEDIR=$(dirname "$0") 

run_tests() {
    # release mode
    # should find client/Cargo.toml no matter where its called from
    # should run test in synchronous way to avoid race conditions
    # output logs -- events from chain
    sleep 1.5
    cargo test --release --manifest-path $BASEDIR/../client/Cargo.toml -- --test-threads 1 --nocapture 
}

start_node() {
   $BASEDIR/../scripts/run-standalone.sh > /dev/null 2>&1

}

start_node & run_tests