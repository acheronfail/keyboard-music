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

run:
  cargo run
runv:
  cargo run --features visualiser -- --vis
rrun:
  cargo run --release
rrunv:
  cargo run --release --features visualiser -- --vis

test:
  cargo test
  cargo test --features visualiser
