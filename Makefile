SHELL := /bin/bash

.PHONY: flow doctor verify verify-fast wiki-init rustdoc-check docs-check docs-audit perf-doctor perf-check perf-bench perf-baseline perf-compare perf-flamegraph perf-heaptrack proto-check proto-build proto-build-dev proto-serve proto-start proto-stop proto-status proto-logs proto-restart tauri-check tauri-dev tauri-build

flow:
	cargo flow

doctor:
	cargo doctor

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

perf-doctor:
	cargo perf doctor

perf-check:
	cargo perf check

perf-bench:
	cargo perf bench

perf-baseline:
	@echo "usage: make perf-baseline BASELINE=<name>"
	@test -n "$(BASELINE)"
	cargo perf baseline "$(BASELINE)"

perf-compare:
	@echo "usage: make perf-compare BASELINE=<name>"
	@test -n "$(BASELINE)"
	cargo perf compare "$(BASELINE)"

perf-flamegraph:
	@echo "usage: make perf-flamegraph ARGS='--bench <bench_name>'"
	@test -n "$(ARGS)"
	cargo perf flamegraph $(ARGS)

perf-heaptrack:
	cargo perf heaptrack $(ARGS)

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

proto-logs:
	cargo dev logs

proto-restart:
	cargo dev restart

tauri-check:
	cargo xtask tauri check

tauri-dev:
	cargo tauri-dev

tauri-build:
	cargo tauri-build
