INSTALL_DIR ?= $(HOME)/.local/bin
BINARY      := sai-code
INSTALL_AS  := sai

.PHONY: build install uninstall clean

build:
	cargo build --release -p sai-cli

install: build
	@mkdir -p $(INSTALL_DIR)
	install -m 755 target/release/$(BINARY) $(INSTALL_DIR)/$(INSTALL_AS)
	@echo "Installed: $(INSTALL_DIR)/$(INSTALL_AS)"

uninstall:
	rm -f $(INSTALL_DIR)/$(INSTALL_AS)
	@echo "Removed: $(INSTALL_DIR)/$(INSTALL_AS)"

clean:
	cargo clean
