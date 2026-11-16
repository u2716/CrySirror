# SPDX-License-Identifier: MPL-2.0
# CrySirror Switch NRO: libnx/deko3d shell (source/) + Rust console player
# staticlib. Derived from xmoezzz/siglus_rs platform/switch/runtime/Makefile
# at 19066ea7f36b92fdecaea240a223c9bef14efd2c (MPL-2.0).
DEVKITPRO ?= /opt/devkitpro
DEVKITA64 ?= $(DEVKITPRO)/devkitA64
TARGET := crysirror
BUILD := build
ROMFS ?= romfs
CC := $(DEVKITA64)/bin/aarch64-none-elf-gcc
ELF2NRO := $(DEVKITPRO)/tools/bin/elf2nro
NACPTOOL := $(DEVKITPRO)/tools/bin/nacptool
UAM := $(DEVKITPRO)/tools/bin/uam
TOOLCHAIN ?= nightly
WORKSPACE := ../../..

CPPFLAGS := -I$(DEVKITPRO)/libnx/include
CFLAGS := -std=gnu17 -O2 -g -Wall -Wextra -ffunction-sections -fdata-sections \
	-march=armv8-a+crc+crypto -mtune=cortex-a57 -mtp=soft -fPIE $(CPPFLAGS)
LDFLAGS := -specs=$(DEVKITPRO)/libnx/switch.specs -g \
	-march=armv8-a+crc+crypto -mtune=cortex-a57 -mtp=soft -fPIE \
	-Wl,--gc-sections -Wl,-Map,$(TARGET).map \
	-Wl,--wrap=nvMapCreate -Wl,--wrap=nvAddressSpaceMap -Wl,--wrap=pthread_create -L$(DEVKITPRO)/libnx/lib
LDLIBS := -ldeko3dd -lnx -lm -lstdc++
RUST_TARGET_DIR := $(WORKSPACE)/target/switch
RUST_LIB := $(RUST_TARGET_DIR)/aarch64-switch/release/libcrysirror_console_player.a

# std's struct stat/dirent for aarch64 Horizon must match devkitA64 newlib.
LIBC_VERSION := 0.2.191
LIBC_SHA256 := 65cd87c9a6d60536580b316ad985726104730a5151c99845f2490428ef91240a
LIBC_PATCH := ../patches/libc-$(LIBC_VERSION)-horizon-aarch64.patch
LIBC_DIR := $(BUILD)/libc-$(LIBC_VERSION)
# -Zbuild-std resolves std's own pinned libc (rust-src Cargo.lock); it needs
# the same layout patch, or std's stat/dirent read the wrong fields.
STD_LIBC_VERSION := 0.2.189
STD_LIBC_SHA256 := 3eaf3ede3fee6db1a4c2ee091bf8a8b4dccdc6d17f656fb07896ee72867612f2
STD_LIBC_PATCH := ../patches/libc-$(STD_LIBC_VERSION)-horizon-aarch64.patch
# Current nightlies resolve -Zbuild-std's std workspace separately, so the
# std libc patch goes into a copy of the toolchain's library workspace that
# cargo builds instead (__CARGO_TESTS_ONLY_SRC_ROOT).
RUST_SRC := $(BUILD)/rust-src
STD_LIBC_DIR := $(BUILD)/libc-$(STD_LIBC_VERSION)

OBJECTS := $(BUILD)/main.o $(BUILD)/gpu.o $(BUILD)/csdk.o
# The integrated Siglus renderer's own WGSL, translated by siglus-shaderc
# (naga 0.20) and compiled by uam.
SIGLUS_WGSL := $(WORKSPACE)/crates/crysirror-siglus-integrated/src/native_vm/render/shaders
GLSL_DIR := $(BUILD)/glsl
SIGLUS_SHADERS := sprite mesh shadow wipe page emote
SHADERS := $(foreach n,$(SIGLUS_SHADERS),$(ROMFS)/shaders/$(n)_vsh.dksh $(ROMFS)/shaders/$(n)_fsh.dksh)

.PHONY: all clean rust
all: $(TARGET).nro

rust: $(LIBC_DIR)/.patched $(STD_LIBC_DIR)/.patched $(RUST_SRC)/.patched
	cd $(WORKSPACE) && \
	CC_aarch64_switch=$(DEVKITA64)/bin/aarch64-none-elf-gcc AR_aarch64_switch=$(DEVKITA64)/bin/aarch64-none-elf-ar \
	CXX_aarch64_switch=$(DEVKITA64)/bin/aarch64-none-elf-g++ \
	CFLAGS_aarch64_switch="$(CFLAGS) -D__SWITCH__" CXXFLAGS_aarch64_switch="$(CFLAGS) -D__SWITCH__" \
	__CARGO_TESTS_ONLY_SRC_ROOT=$(CURDIR)/$(RUST_SRC)/library \
	CARGO_TARGET_DIR=target/switch RUSTFLAGS='--cfg getrandom_backend="custom"' \
	cargo +$(TOOLCHAIN) build --release -p crysirror-console-player --lib \
	  --target platform/switch/rust/aarch64-switch.json -Zbuild-std=std,panic_abort -Zjson-target-spec \
	  --config 'patch.crates-io.libc.path="platform/switch/runtime/$(LIBC_DIR)"'

$(RUST_SRC)/.patched: $(STD_LIBC_DIR)/.patched
	mkdir -p $(RUST_SRC)
	cp -R "$$(rustc +$(TOOLCHAIN) --print sysroot)/lib/rustlib/src/rust/library" $(RUST_SRC)/
	python3 ../tools/patch_std_libc.py $(RUST_SRC)/library ../../../libc-$(STD_LIBC_VERSION)
	touch $@

$(BUILD) $(ROMFS)/shaders:
	mkdir -p $@

$(LIBC_DIR)/.patched: $(LIBC_PATCH) | $(BUILD)
	rm -rf $(LIBC_DIR) $(BUILD)/libc.crate
	curl -sSfL -o $(BUILD)/libc.crate https://static.crates.io/crates/libc/libc-$(LIBC_VERSION).crate
	echo "$(LIBC_SHA256)  $(BUILD)/libc.crate" | shasum -a 256 -c --quiet -
	tar -xzf $(BUILD)/libc.crate -C $(BUILD)
	cd $(LIBC_DIR) && patch -s -p1 < ../../$(LIBC_PATCH)
	rm $(BUILD)/libc.crate
	touch $@

$(STD_LIBC_DIR)/.patched: $(STD_LIBC_PATCH) | $(BUILD)
	rm -rf $(STD_LIBC_DIR) $(BUILD)/libc-std.crate
	curl -sSfL -o $(BUILD)/libc-std.crate https://static.crates.io/crates/libc/libc-$(STD_LIBC_VERSION).crate
	echo "$(STD_LIBC_SHA256)  $(BUILD)/libc-std.crate" | shasum -a 256 -c --quiet -
	tar -xzf $(BUILD)/libc-std.crate -C $(BUILD)
	cd $(STD_LIBC_DIR) && patch -s -p1 < ../../$(STD_LIBC_PATCH)
	rm $(BUILD)/libc-std.crate
	touch $@

$(BUILD)/%.o: source/%.c | $(BUILD)
	$(CC) $(CFLAGS) -MMD -MP -c $< -o $@

$(GLSL_DIR)/.stamp: $(wildcard $(SIGLUS_WGSL)/*.wgsl) ../siglus-shaderc/src/main.rs | $(BUILD)
	mkdir -p $(GLSL_DIR)
	cargo run --release --manifest-path ../siglus-shaderc/Cargo.toml -- $(SIGLUS_WGSL) $(GLSL_DIR)
	touch $@

$(ROMFS)/shaders/%_vsh.dksh: $(GLSL_DIR)/.stamp | $(ROMFS)/shaders
	$(UAM) --stage=vert --out=$@ $(GLSL_DIR)/$*_vsh.glsl
$(ROMFS)/shaders/%_fsh.dksh: $(GLSL_DIR)/.stamp | $(ROMFS)/shaders
	$(UAM) --stage=frag --out=$@ $(GLSL_DIR)/$*_fsh.glsl

# NRO icons must be 256x256 JPEG, which has no alpha channel: the transparent
# canvas is flattened onto hbmenu's dark-theme tile colour, not white.
ICON_BACKGROUND := \#2d2d2d
# Adopted engines: the shared renderer's own WGSL, captured by running it on
# the deko3d backend's null driver over the repository's example projects,
# then compiled per entry by UAM (crysirror-shader-tools cook-switch).
CAPTURE_PROJECTS := examples/desktop/project.json examples/desktop/arena.json examples/desktop/systems.json \
	examples/desktop/glyph-effects.json examples/aaa-systems/project.json examples/aaa-helmet/project.json \
	examples/aaa-vfx/project.json examples/materials/project.json examples/shader-effects/project.json \
	examples/engine/project.json
$(ROMFS)/wgsl/index.json: | $(ROMFS)/shaders
	rm -rf $(BUILD)/capture $(ROMFS)/wgsl
	cd $(WORKSPACE) && for p in $(CAPTURE_PROJECTS); do \
	  cargo run --release -q -p crysirror-wgpu-deko3d --example capture -- platform/switch/runtime/$(BUILD)/capture $$p || exit 1; done
	cd $(WORKSPACE) && UAM=$(UAM) cargo run --release -q -p crysirror-shader-tools -- cook-switch \
	  platform/switch/runtime/$(BUILD)/capture platform/switch/runtime/$(ROMFS)/wgsl

$(BUILD)/icon.jpg: ../../../icon/crysirror.svg Makefile | $(BUILD)
	rsvg-convert -w 256 -h 256 -b '$(ICON_BACKGROUND)' $< -o $(BUILD)/icon.png
	magick $(BUILD)/icon.png -quality 92 $@

$(TARGET).elf: $(OBJECTS) rust
	$(CC) $(LDFLAGS) $(OBJECTS) $(RUST_LIB) $(LDLIBS) -o $@

$(TARGET).nacp:
	$(NACPTOOL) --create "CrySirror" "CrySirror" "0.2.0" $@ --titleid=0100C5A1E0000001

$(TARGET).nro: $(TARGET).elf $(TARGET).nacp $(SHADERS) $(ROMFS)/wgsl/index.json $(BUILD)/icon.jpg
	$(ELF2NRO) $< $@ --nacp=$(TARGET).nacp --icon=$(BUILD)/icon.jpg --romfsdir=$(ROMFS)

clean:
	rm -rf $(BUILD) $(ROMFS) $(TARGET).elf $(TARGET).map $(TARGET).nacp $(TARGET).nro
-include $(OBJECTS:.o=.d)
