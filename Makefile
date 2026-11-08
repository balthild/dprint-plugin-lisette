help:
	echo "Usage: make [build|check]"

build:
	cargo build --release --target=wasm32-unknown-unknown

check: check-lib check-plugin

check-lib:
	cargo clippy --all-targets

check-plugin:
	cargo clippy --target=wasm32-unknown-unknown
