help:
	echo "Usage: make [build|check]"

build:
	cargo build --release --target=wasm32-unknown-unknown

check:
	cargo clippy --target=wasm32-unknown-unknown
