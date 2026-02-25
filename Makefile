SHELL := /bin/bash

.PHONY: verify verify-fast docs-check docs-audit proto-check proto-build proto-serve

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

proto-serve:
	cargo dev
