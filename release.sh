#!/bin/bash

# RELEASE UTILITY
# This script helps with the release process on Github (glibc builds for Linux)

mkdir -p ./builds
rm ./builds/*

CLI_VERSION=$(/usr/bin/cat Cargo.toml | egrep "version = (.*)" | egrep -o --color=never "([0-9]+\.?){3}" | head -n 1)
echo "Releasing v$CLI_VERSION for musl & libc x86_64 targets"

# Build a 'musl' release for Linux x86_64
cargo build --release --target=x86_64-unknown-linux-musl --locked
cp -p ./target/x86_64-unknown-linux-musl/release/watchdog-rs ./builds/watchdog-rs-v$CLI_VERSION-x86_64-unknown-linux-musl
./builds/watchdog-rs-v$CLI_VERSION-x86_64-unknown-linux-musl --version

# Build a 'glibc' (GNU) release for Linux x86_64
cargo build --release --target=x86_64-unknown-linux-gnu --locked
cp -p ./target/x86_64-unknown-linux-gnu/release/watchdog-rs ./builds/watchdog-rs-v$CLI_VERSION-x86_64-unknown-linux-glibc
./builds/watchdog-rs-v$CLI_VERSION-x86_64-unknown-linux-glibc --version

# Build the deb archive
mkdir -p ./builds/watchdog-rs_$CLI_VERSION-1_amd64/DEBIAN
echo "Package: watchdog-rs" > ./builds/watchdog-rs_$CLI_VERSION-1_amd64/DEBIAN/control
echo "Version: $CLI_VERSION" >> ./builds/watchdog-rs_$CLI_VERSION-1_amd64/DEBIAN/control
echo "Architecture: amd64" >> ./builds/watchdog-rs_$CLI_VERSION-1_amd64/DEBIAN/control
echo "Maintainer: Kongbytes" >> ./builds/watchdog-rs_$CLI_VERSION-1_amd64/DEBIAN/control
echo "Description: Minimalist & multi-region network monitoring tool written in Rust" >> ./builds/watchdog-rs_$CLI_VERSION-1_amd64/DEBIAN/control
mkdir -p ./builds/watchdog-rs_$CLI_VERSION-1_amd64/usr/local/bin
cp ./builds/watchdog-rs-v$CLI_VERSION-x86_64-unknown-linux-musl ./builds/watchdog-rs_$CLI_VERSION-1_amd64/usr/local/bin/watchdog
(cd ./builds && dpkg-deb --build --root-owner-group watchdog-rs_$CLI_VERSION-1_amd64)

echo "Update the README instructions for v$CLI_VERSION"
echo " ✓ Publish on crates.io"
echo " ✓ Release on Github with Git tag v$CLI_VERSION"
