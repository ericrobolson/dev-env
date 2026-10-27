.PHONY: deps install test

deps:
	@if ! command -v pdfinfo >/dev/null 2>&1 || ! command -v pdftotext >/dev/null 2>&1; then \
		if command -v brew >/dev/null 2>&1; then brew install poppler; \
		else echo 'Install poppler (pdfinfo and pdftotext) with your package manager.' >&2; exit 1; fi; \
	fi
	@if ! command -v php >/dev/null 2>&1; then \
		if command -v brew >/dev/null 2>&1; then brew install php; \
		else echo 'Install php with your package manager.' >&2; exit 1; fi; \
	fi

install:
	cargo build --release
	mkdir -p "$(HOME)/bin"
	cp target/release/wiki-writer "$(HOME)/bin/wiki-writer"

test:
	cargo test
	cargo clippy --all-targets -- -D warnings
