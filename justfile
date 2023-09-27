default:
  just -l

fmt:
  rustup run nightly cargo fmt
