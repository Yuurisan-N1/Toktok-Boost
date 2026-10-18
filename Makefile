BINARY := toktok-boost
RELEASE_DIR := target/release
DEBUG_DIR := target/debug

.PHONY: all build run release start check fmt clean size package

all: build

build:
	cargo build

run:
	cargo run

release:
	cargo build --release

start: release
	./$(RELEASE_DIR)/$(BINARY)

check:
	cargo check

fmt:
	cargo fmt

clean:
	cargo clean

size: release
	@ls -lh $(RELEASE_DIR)/$(BINARY) | awk '{print "Binary size:", $$5}'

package: release
	bash scripts/package.sh
