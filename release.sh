#!/bin/bash

# RELEASE UTILITY
# This script helps with the release process on Github (glibc builds for Linux)

mkdir -p ./builds
rm ./builds/*

CLI_VERSION=$(/usr/bin/cat Cargo.toml | egrep "version = (.*)" | egrep -o --color=never "([0-9]+\.?){3}" | head -n 1)
echo "Releasing v$CLI_VERSION for libc targets"

# Build a 'glibc' (GNU) release for Linux x86_64
cargo build --release --target=x86_64-unknown-linux-gnu --locked
cp -p ./target/x86_64-unknown-linux-gnu/release/watchdog-rs ./builds/watchdog-rs-v$CLI_VERSION-x86_64-unknown-linux-glibc
./builds/watchdog-rs-v$CLI_VERSION-x86_64-unknown-linux-glibc --version

echo "Update the README instructions for v$CLI_VERSION"
echo " ✓ Publish on crates.io"
echo " ✓ Release on Github with Git tag v$CLI_VERSION"
