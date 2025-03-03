#!/bin/bash

# Need to add this into a default toml or something

RUST_LOG=info cargo run -- \
    -u 'http://localhost:6969' \
    -t 'https://swapi.dev/api/' \
    -s '/tmp/swapi-requests' \
    -e 'Accept: application/json' \
    -e 'Content-Type: application/json'
