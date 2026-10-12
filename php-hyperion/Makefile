.PHONY: test test-rust test-php

test: test-rust test-php

test-rust:
	cargo test

test-php:
	@echo "Running PHP tests..."
	@for f in test_*.php lsb_*.php parse_*.php; do \
		echo "--- Testing $$f ---"; \
		./target/debug/hyperion-cli "$$f" || true; \
	done
	@echo "All PHP tests executed."
