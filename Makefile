help:
	echo "Usage: make [build|check|test]"

build:
	cargo build --release --target=wasm32-unknown-unknown --features=plugin

check:
	cargo clippy --all-targets
	cargo clippy --target=wasm32-unknown-unknown --features=plugin

test:
	cargo test
