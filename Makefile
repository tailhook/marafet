PREFIX ?= /usr/local
DESTDIR ?=

marafet:
	cargo build

install:
	cargo build --release
	cp ./target/release/marafet $(DESTDIR)$(PREFIX)/bin/marafet

