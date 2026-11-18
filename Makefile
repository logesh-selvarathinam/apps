# The app author's loop. A thin wrapper: every step runs a tool from the wp2-apps checkout in .sdk/wp2-apps,
# pinned to WP2_APPS_REV. `make help` lists the targets, `make doctor` what this machine is missing.
# ponytail: phony targets only, cargo and the tools already skip unchanged work.

# The SDK and its tooling. To move to a newer one: change the commit here, `make sdk`, build and test every app,
# open a PR. Packages built against it change, but a store update still needs each app's version raised.
WP2_APPS_REV := 12c052b3926265bbd9642d881daca43880246237
WP2_APPS_GIT := https://github.com/cmf-tech-community/cmf-watch-sdk
SDK := .sdk/wp2-apps

# Not vendored: cmf-watch-utils (AGPL; the emulator, its runtime/ and fixtures/, the stock firmware dump) and a
# stock NOR image. Point these at your own copies.
CMF_WATCH_UTILS ?= ../cmf-watch-utils
STOCK_NOR ?= $(CMF_WATCH_UTILS)/fixtures/nor.bin
NOR ?= target/nor-wp2.bin
TEMPLATE := vibration-lab

HOST := $(shell rustc -vV 2>/dev/null | sed -n 's/^host: //p')
ifeq ($(shell uname -s),Darwin)
WP2_NETSIMD ?= $(HOME)/Library/Android/sdk/emulator/netsimd
else
WP2_NETSIMD ?= $(HOME)/Android/Sdk/emulator/netsimd
endif
# the wp2-apps tools resolve cmf-watch-utils relative to their own checkout unless told otherwise
export CMF_WATCH_UTILS := $(abspath $(CMF_WATCH_UTILS))
export WP2_NETSIMD
# llvm-objcopy from rustup's llvm-tools component
export PATH := $(shell rustc --print sysroot 2>/dev/null)/lib/rustlib/$(HOST)/bin:$(PATH)

WP2 := uv run --quiet --locked --project $(SDK) wp2apps
# where the watch is: the emulator's netsimd port, or a real watch's address from `$(WP2) scan`
WHERE = $(if $(ADDRESS),--address $(ADDRESS),--hci-port $(or $(HCI_PORT),$(error set HCI_PORT=<port> or ADDRESS=<watch>)))
need = $(if $($(1)),,$(error set $(1)=<$(2)>))

.PHONY: help doctor sdk new build check test sim fw nor emu run scan list remove clean

help:
	@echo 'sdk                  fetch wp2-apps at the pinned commit into $(SDK) (every target below does it)'
	@echo 'new NAME=x           a new app in apps/x, copied from apps/$(TEMPLATE)'
	@echo 'build [APP=x]        target/x.wp2app (all apps without APP)'
	@echo 'check                validate every store.toml/store.md'
	@echo 'test [APP=x]         host unit tests'
	@echo 'sim APP=x            run the app on this computer in the wp2-apps simulator, args in SIM_ARGS'
	@echo 'fw                   stub firmware + App Loader from the pinned wp2-apps (needs CMF_WATCH_UTILS)'
	@echo 'nor                  $(NOR): STOCK_NOR with the stub firmware'
	@echo 'emu [HCI_PORT=P]     boot $(NOR) in the emulator, extra args in EMU_ARGS'
	@echo 'run APP=x HCI_PORT=P build x, install the loader if missing, then x (or ADDRESS=watch)'
	@echo 'scan                 watches nearby, for ADDRESS='
	@echo 'list / remove SLOT=N what is installed / remove a slot (HCI_PORT=P or ADDRESS=watch)'
	@echo 'doctor               what this machine is missing'
	@echo
	@echo 'CMF_WATCH_UTILS=$(CMF_WATCH_UTILS)'
	@echo 'STOCK_NOR=$(STOCK_NOR)'
	@echo 'WP2_NETSIMD=$(WP2_NETSIMD)'

doctor:
	@ok=1; \
	check() { if eval "$$2" >/dev/null 2>&1; then echo "ok       $$1"; else echo "MISSING  $$1: $$3"; ok=0; fi; }; \
	check git 'command -v git' 'install git'; \
	check uv 'command -v uv' 'https://docs.astral.sh/uv/'; \
	check cargo 'command -v cargo' 'https://rustup.rs'; \
	check 'thumbv7em-none-eabi target' 'rustup target list --installed | grep -qx thumbv7em-none-eabi' 'rustup target add thumbv7em-none-eabi'; \
	check llvm-objcopy 'command -v llvm-objcopy' 'rustup component add llvm-tools'; \
	check resvg 'command -v resvg' 'cargo install resvg, or your package manager'; \
	check 'SDK checkout' 'test "$$(git -C $(SDK) rev-parse HEAD)" = $(WP2_APPS_REV)' 'make sdk'; \
	echo '-- emulator only (fw, nor, emu, run):'; \
	check 'cmf-watch-utils runtime' 'test -x "$(CMF_WATCH_UTILS)/runtime/qemu-system-arm"' 'set CMF_WATCH_UTILS to a cmf-watch-utils checkout with runtime/ set up'; \
	check 'cmf-watch-utils fixtures' 'test -d "$(CMF_WATCH_UTILS)/fixtures"' 'its fixtures/ (see the cmf-watch-utils README)'; \
	check 'stock firmware' 'test -f "$(CMF_WATCH_UTILS)/dumps/binaries/firmware/JX402_V1.0.0.73.bin"' 'JX402 1.0.0.73 under dumps/binaries/firmware/'; \
	check 'stock NOR' 'test -f "$(STOCK_NOR)"' 'set STOCK_NOR'; \
	check netsimd 'test -x "$(WP2_NETSIMD)"' 'Android SDK emulator, or set WP2_NETSIMD'; \
	check 'wish (Tk 8.6+)' 'echo "puts \$$tk_version; exit" | $(or $(WISH),wish) | grep -Eqx "8\.[6-9]|9\..*"' 'the emulator window: Tk 8.6+ wish on PATH or WISH=/path (macOS /usr/bin/wish is 8.5)'; \
	test $$ok = 1

sdk:
	@if [ "$$(git -C $(SDK) rev-parse HEAD 2>/dev/null)" != $(WP2_APPS_REV) ]; then \
		[ -d $(SDK)/.git ] || git clone --quiet --filter=blob:none $(WP2_APPS_GIT) $(SDK); \
		git -C $(SDK) fetch --quiet origin $(WP2_APPS_REV) 2>/dev/null || git -C $(SDK) fetch --quiet origin; \
		git -C $(SDK) -c advice.detachedHead=false checkout --quiet $(WP2_APPS_REV); \
		echo "wp2-apps at $(WP2_APPS_REV)"; \
	fi

new:
	$(call need,NAME,name)
	@echo '$(NAME)' | grep -Eqx '[a-z0-9_-]{1,23}' || { echo 'NAME: 1-23 lowercase letters, digits, - and _'; exit 1; }
	@test ! -e apps/$(NAME) || { echo 'apps/$(NAME) exists'; exit 1; }
	cp -R apps/$(TEMPLATE) apps/$(NAME)
	sed -i.bak 's/"$(TEMPLATE)"/"$(NAME)"/' apps/$(NAME)/Cargo.toml
	printf 'name = "%s"\nsummary = ""\nversion = "0.1.0"\nauthor = ""\npublished = true\n' '$(NAME)' > apps/$(NAME)/store.toml
	printf 'What %s does.\n\n## Requirements\n\nThe [patched firmware](/firmware) and the [App Loader](/firmware).\n' '$(NAME)' > apps/$(NAME)/store.md
	rm -f apps/$(NAME)/Cargo.toml.bak
	@echo 'apps/$(NAME): now edit src/, icon.svg, store.toml and store.md'

build: sdk
	scripts/store build $(or $(APP),$(notdir $(patsubst %/store.toml,%,$(wildcard apps/*/store.toml))))

check:
	scripts/store check

test: sdk
	cd apps && cargo test --target $(HOST) $(if $(APP),-p $(APP))

sim: sdk
	$(call need,APP,name)
	cd $(SDK)/sim && cargo build --release --quiet
	cd apps && RUSTFLAGS="--cfg wp2_sim --check-cfg=cfg(wp2_sim) -L native=$(CURDIR)/$(SDK)/sim/target/release -C link-arg=-Wl,-rpath,$(CURDIR)/$(SDK)/sim/target/release" \
		cargo build --quiet --target $(HOST) --target-dir $(CURDIR)/target/sim -p $(APP) \
		--config profile.dev.overflow-checks=false --config profile.dev.debug-assertions=false
	WP2_SIM_ROOT=$(CURDIR) target/sim/$(HOST)/debug/$(APP) $(SIM_ARGS)

fw: sdk
	cd $(SDK) && scripts/build-fw

nor: fw
	$(WP2) nor $(STOCK_NOR) --app $(SDK)/target/app-wp2.bin --output $(NOR)

emu: nor
	mkdir -p runs
	uv run --locked --project $(SDK) cmf-watch-utils emu run --fixtures $(CMF_WATCH_UTILS)/fixtures \
		--qemu $(CMF_WATCH_UTILS)/runtime/qemu-system-arm --rom $(CMF_WATCH_UTILS)/runtime/compat.bin \
		--output runs/$(shell date +%Y%m%d-%H%M%S) --nor $(NOR) $(if $(HCI_PORT),--hci-port $(HCI_PORT)) \
		$(if $(WISH),--wish $(WISH)) $(EMU_ARGS)

run: build
	$(call need,APP,name)
	@test -f $(SDK)/target/loader.wp2app || $(MAKE) fw
	$(WP2) list $(WHERE) | grep -q '"name": "loader"' || $(WP2) install $(SDK)/target/loader.wp2app $(WHERE)
	$(WP2) install target/$(APP).wp2app $(WHERE) $(if $(SLOT),--slot $(SLOT))

scan: sdk
	$(WP2) scan --show-address

list: sdk
	$(WP2) list $(WHERE)

remove: sdk
	$(call need,SLOT,slot)
	$(WP2) remove --slot $(SLOT) $(WHERE)

clean:
	rm -rf target
