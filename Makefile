SHELL := /bin/bash

.PHONY: verify verify-fast docs-check docs-audit proto-check proto-build proto-build-dev proto-serve proto-start proto-stop proto-status proto-restart

verify:
	cargo verify

verify-fast:
	cargo verify-fast

docs-check:
	python3 scripts/docs/validate_docs.py all

docs-audit:
	python3 scripts/docs/validate_docs.py audit-report --output .artifacts/docs-audit.json

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
