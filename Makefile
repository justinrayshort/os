SHELL := /bin/bash

.PHONY: verify verify-fast docs-check rustdoc-check wiki-init proto-serve proto-stop proto-status

verify:
	cargo verify

verify-fast:
	cargo verify-fast

docs-check:
	cargo docs-check

rustdoc-check:
	cargo doc --workspace --no-deps
	cargo test --workspace --doc

wiki-init:
	git submodule sync --recursive
	git submodule update --init --recursive

proto-serve:
	cargo dev serve

proto-stop:
	cargo dev stop

proto-status:
	cargo dev status
