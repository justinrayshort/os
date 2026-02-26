SHELL := /bin/bash

.PHONY: verify verify-fast wiki-init rustdoc-check docs-check docs-audit proto-check proto-build proto-build-dev proto-serve proto-start proto-stop proto-status proto-restart

verify:
	cargo verify

verify-fast:
	cargo verify-fast

wiki-init:
	git submodule sync --recursive
	git submodule update --init --recursive

rustdoc-check:
	cargo doc --workspace --no-deps
	cargo test --workspace --doc

docs-check:
	cargo docs-check
	$(MAKE) rustdoc-check

docs-audit:
	cargo docs-audit

proto-check:
	cargo web-check

proto-build:
	cargo web-build

proto-build-dev:
	cargo dev build

proto-serve:
	cargo dev serve

proto-start:
	cargo dev start

proto-stop:
	cargo dev stop

proto-status:
	cargo dev status

proto-restart:
	cargo dev restart
