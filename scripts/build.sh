#!/usr/bin/env bash

mkdir -p ./dist

cargo build --release --locked

mv target/release/gh-mount "./dist/linux-amd64"