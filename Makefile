.PHONY: deps-install
deps-install:
	@echo "Installing dependencies..."
	rustup default stable
	cargo fetch
