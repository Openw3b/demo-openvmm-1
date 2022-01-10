#!/bin/bash

export PULSE_SERVER=tcp:$(cat /mnt/host_ip):4713

a=$(cat /proc/cmdline)
pat='OPENVM_LAUNCH=(.*)'
if [[ "$a" =~ $pat ]]; then
    echo "${BASH_REMATCH[0]}"
    ${BASH_REMATCH[1]}
fi

sudo shutdown -h now