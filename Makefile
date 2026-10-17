PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
SYSCONFDIR ?= /etc
SYSTEMDDIR ?= $(SYSCONFDIR)/systemd/system

.PHONY: all build test clean install uninstall

all: build

build:
	cargo build --release

test:
	cargo test

install: build
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 target/release/wifi-pm $(DESTDIR)$(BINDIR)/wifi-pm
	install -d $(DESTDIR)$(SYSTEMDDIR)
	$(DESTDIR)$(BINDIR)/wifi-pm install

uninstall:
	systemctl disable --now wifi-pm.service 2>/dev/null || true
	rm -f $(DESTDIR)$(SYSTEMDDIR)/wifi-pm.service
	rm -f $(DESTDIR)$(BINDIR)/wifi-pm
	rm -f $(DESTDIR)$(SYSCONFDIR)/wifi-pm.conf
	rm -f $(DESTDIR)$(SYSCONFDIR)/NetworkManager/conf.d/99-wifi-powersave.conf
	rm -f $(DESTDIR)$(SYSCONFDIR)/NetworkManager/dispatcher.d/99-wifi-pm.sh
	systemctl daemon-reload 2>/dev/null || true

clean:
	cargo clean
