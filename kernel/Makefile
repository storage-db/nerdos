# Arguments
ARCH ?= x86_64
MODE ?= release
LOG ?= warn
RVM ?= off
GUEST ?= off

# Platform
ifeq ($(ARCH), x86_64)
  ACCEL ?= on
  ifeq ($(RVM), on)
    PLATFORM ?= pc-rvm
  else ifeq ($(GUEST), on)
    PLATFORM ?= rvm-guest-x86_64
  else
    PLATFORM ?= pc
  endif
else ifeq ($(ARCH), aarch64)
  ACCEL ?= off
  PLATFORM ?= qemu-virt-arm
else ifeq ($(ARCH), riscv64)
  ACCEL ?= off
  PLATFORM ?= qemu-virt-riscv
endif

export ARCH
export PLATFORM
export MODE
export LOG

make_args := ARCH=$(ARCH) PLATFORM=$(PLATFORM) MODE=$(MODE) LOG=$(LOG)

# Paths
target := ../targets/$(ARCH).json
kernel_elf := target/$(ARCH)/$(MODE)/nerdos
kernel_bin := $(kernel_elf).bin

# Cargo features and build args
features := platform-$(PLATFORM)

ifeq ($(RVM), on)
  features += rvm
endif

build_args := --no-default-features --features "$(features)" --target $(target) -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
ifeq ($(MODE), release)
  build_args += --release
endif
ifeq ($(MODe), debug)
  build_args += --verbose
endif

# Binutils
OBJDUMP := rust-objdump -d --print-imm-hex --x86-asm-syntax=intel
OBJCOPY := rust-objcopy --binary-architecture=$(ARCH)
GDB := gdb-multiarch

# QEMU
qemu := qemu-system-$(ARCH)
qemu_args := -nographic -m 128M

ifeq ($(ACCEL), on)
  ifeq ($(shell uname), Darwin)
    qemu_args += -cpu host -accel hvf
  else
    qemu_args += -cpu host -accel kvm
  endif
endif

ifeq ($(ARCH), x86_64)
  qemu_args += \
    -machine q35 \
    -serial mon:stdio \
    -kernel $(kernel_elf)
else ifeq ($(ARCH), aarch64)
  qemu_args += \
    -cpu cortex-a72 \
    -machine virt \
    -kernel $(kernel_bin)
else ifeq ($(ARCH), riscv64)
  qemu_args += \
    -machine virt \
    -bios default \
    -kernel $(kernel_bin)
endif

# GDB
GDB := gdb-multiarch

build: $(kernel_bin)

env:
	cargo install cargo-binutils --vers =0.3.3
	rustup component add rust-src
	rustup component add llvm-tools-preview

$(kernel_bin): kernel
	@$(OBJCOPY) $(kernel_elf) --strip-all -O binary $@

user:
	@cd ../user && make build

kernel:
	@echo Arch: $(ARCH), Platform: $(PLATFORM)
ifneq ($(shell head -n1 src/platform/config.rs 2> /dev/null | cut -d' ' -f2), $(ARCH)-$(PLATFORM))
	@echo $(make_args) > .makeargs
endif
ifneq ($(shell cat .makeargs 2> /dev/null), $(make_args))
	@echo $(make_args) > .makeargs
endif
	cargo build $(build_args)

clean:
	@cd ../user && make clean
	@cargo clean

clippy:
	cargo clippy $(build_args)

disasm:
	@$(OBJDUMP) $(kernel_elf) | less

run: user build justrun

justrun:
	$(qemu) $(qemu_args)

debug: build
	$(qemu) $(qemu_args) -s -S &
	sleep 1
	$(GDB) $(kernel_elf) -ex 'target remote localhost:1234'

scp:
	scp -P 2333 $(kernel_bin) ubuntu@localhost:/home/ubuntu

.PHONY: build env kernel user clean disasm run debug scp
