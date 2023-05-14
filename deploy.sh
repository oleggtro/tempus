#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

readonly TARGET_HOST=pi@192.168.178.93
readonly TARGET_PATH=/home/pi/ds18b20-exporter
readonly SOURCE_PATH=./target/arm-unknown-linux-gnueabi/release/ds18b20-exporter
readonly TARGET_ARCH=arm-unknown-linux-gnueabi


cross build --release --target=${TARGET_ARCH}
rsync -P ${SOURCE_PATH} ${TARGET_HOST}:${TARGET_PATH}
ssh -t ${TARGET_HOST} ${TARGET_PATH}
