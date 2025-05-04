#!/bin/bash

version=$(echo "$(uname -s)-$(uname -m)" | tr '[:upper:]' '[:lower:]')

rm -r solarbx-smb-$version

echo "Start building solarbx smb..."

cargo build --release

mkdir solarbx-smb-$version

mv target/release/ssolarbx-smb solarbx-smb-$version/ssolarbx-smb

echo "Finished build solarbx smb..."

echo "Start compress solarbx smb..."

tar czf solarbx-smb-$version.tar.gz solarbx-smb-$version

echo "Finished compress solarbx smb..."
