dev:
  cargo test --all --color=always -- --test-threads=1 --quiet

ci: ci-test build fmt doc

ci-test:
  cargo test --all --color=always --features ci -- --test-threads=1 --quiet

build:
  cargo build --features=ci

fmt:
  cargo fmt -- --check

doc:
  cargo doc
