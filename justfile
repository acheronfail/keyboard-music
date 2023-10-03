alias f := fmt
alias t := test
alias r := run
alias rv := runv
alias rr := run
alias rrv := runv

default:
  just -l

fmt:
  rustup run nightly cargo fmt

run *args:
  cargo run -- {{args}}
runv *args:
  cargo run --features visualiser -- --vis {{args}}
rrun *args:
  cargo run --release -- {{args}}
rrunv *args:
  cargo run --release --features visualiser -- --vis {{args}}

test:
  cargo test
  cargo test --features visualiser

install:
  cargo install --all-features --path .
