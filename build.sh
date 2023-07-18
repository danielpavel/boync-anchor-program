#!/bin/bash

function exists_in_list() {
    LIST=$1
    DELIMITER=$2
    VALUE=$3
    echo $LIST | tr "$DELIMITER" '\n' | grep -F -q -x "$VALUE"
}

input=$1

programs="token-metadata boync-anchor-program"

mkdir -p test-programs

if exists_in_list "$programs" " " $input; then
    echo "building $input"
    cd programs/$input/program
    cargo build-bpf --bpf-out-dir ../../../test-programs/
    cd ../../

elif [[ $input = "all" ]]
then
    echo "building all programs"
    for program in ${programs}; do
        echo "building $program"
        cd programs/$program/program

        echo "Building inside: $PWD"
        cargo build-bpf --bpf-out-dir ../../../test-programs/
        cd ../../../
        echo "Moving out to: $PWD"
    done

    # echo "building testing-utils"
    # echo $PWD
    # cd programs/core/rust/testing-utils
    # cargo build-bpf --bpf-out-dir ../../../../test-programs/
    # cd ../../../../

elif [[ $input = "token-auth-rules" ]]
then
    solana program dump -u https://api.mainnet-beta.solana.com auth9SigNpDKz4sJJ1DfCTuZrZNSAgh9sFD3rboVmgg ./test-programs/mpl_token_auth_rules.so
else
    echo "Invalid program name: $input"
    exit 1
fi