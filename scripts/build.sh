#!/usr/bin/env bash

mkdir -p ./dist

cargo build --release

mv target/release/gh-mount "./dist/linux-arm64"