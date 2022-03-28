#!/bin/bash

set -e
cd "$(dirname "${BASH_SOURCE[0]}")/.."

VERSION=0.0.1
IMAGE_NAME=openvmm_dev

if [ ! -d third_party ]
then
   mkdir -p third_party
   git clone https://chromium.googlesource.com/chromiumos/platform/crosvm ./third_party/crosvm || \
   git -C third_party/crosvm pull
   git -C third_party/crosvm checkout a475cd47547431ea4b448b5003a17fd0e8c13237
   git -C third_party/crosvm submodule update --init
   git -C third_party/crosvm apply ../../crosvm_xchanges.patch || true
fi

if [ "$1" == "ubuntu" ]; then
    ./scripts/guest_os/build.sh
else
    ./scripts/guest/build.sh
fi

docker build -f scripts/Dockerfile -t "${IMAGE_NAME}:${VERSION}" .
