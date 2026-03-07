default:
    @just --list

prepare:
    cd crates/cli && just prepare

build profile='release':
    cargo build -p pruner --profile={{ profile }}

install profile='release': (build profile)
    cp target/{{ if profile == "dev" { "debug" } else { profile } }}/pruner ~/.local/bin/pruner

test test="":
    cargo test {{ test }} -- --nocapture
