# Building
TARGET := riscv64
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/nerdos
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := target/$(TARGET)/$(MODE)/asm
FS_IMG := ../user/target/$(TARGET)/$(MODE)/fs.img
U_FAT32_DIR := ../fat32-fuse
ifeq ($(BOARD), k210)
	U_FAT32 := /dev/sdb1
else 
	U_FAT32 := ${U_FAT32_DIR}/fat32.img
endif



SDCARD := /dev/sdb
APPS := ../user/src/bin/*
TOP := ../../k210.bin

# BOARD
BOARD ?= qemu
SBI ?= rustsbi
ifeq ($(BOARD), qemu)
	BOOTLOADER := default
#	BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin # If you have no OpenSBI, try RustSBI.
#	BOOTLOADER := ../bootloader/$(SBI)-$(BOARD)-new.bin # If you want to use new RustSBI, try this.
else ifeq ($(BOARD), k210)
	BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin
endif
K210_BOOTLOADER_SIZE := 131072

# KERNEL ENTRY
ifeq ($(BOARD), qemu)
	KERNEL_ENTRY_PA := 0x80200000
else ifeq ($(BOARD), k210)
	KERNEL_ENTRY_PA := 0x80020000
endif

# Run K210
K210-SERIALPORT	= /dev/ttyUSB0
K210-BURNER	= ../tools/kflash.py

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# Disassembly
DISASM ?= -x

# build: env $(KERNEL_BIN) $(FS_IMG) fat32
build: $(KERNEL_BIN)

env:
	rustup override set nightly-2021-05-10
	(rustup target list | grep "riscv64imac-unknown-none-elf (installed)") || rustup target add $(TARGET)
	rustup component add rust-src
	rustup component add llvm-tools-preview

#(rustup target list | grep "riscv64gc-unknown-none-elf (installed)") || rustup target add $(TARGET)
#cargo install cargo-binutils
	

# dev/zero永远输出0
sdcard: 
	@echo "Are you sure write to $(SDCARD) ? [y/N] " && read ans && [ $${ans:-N} = y ]
	@sudo dd if=/dev/zero of=$(SDCARD) bs=1048576 count=16
	@sudo dd if=$(FS_IMG) of=$(SDCARD)

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@
#	@$(OBJDUMP) -S $(KERNEL_ELF) > $(KERNEL_ELF).S

$(APPS):

fat32: 
	./buildfs.sh

fsimg-format:
	sudo mkfs.vfat -F 32 ${U_FAT32}

kernel:
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo build --release --no-default-features --features "board_$(BOARD) platform-qemu-virt-riscv" --target ../targets/riscv64.json -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
	@rm src/linker.ld

clean:
	@cargo clean

run: run-inner

run-inner:  build 
ifeq ($(BOARD),qemu)
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(U_FAT32),if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0\
		-smp threads=2
else
	(which $(K210-BURNER)) || (cd .. && git clone https://github.com/sipeed/kflash.py.git && mv kflash.py tools)
	@cp $(BOOTLOADER) $(BOOTLOADER).copy
	@dd if=$(KERNEL_BIN) of=$(BOOTLOADER).copy bs=$(K210_BOOTLOADER_SIZE) seek=1
	@mv $(BOOTLOADER).copy $(KERNEL_BIN)
	@sudo chmod 777 $(K210-SERIALPORT)
	python3 $(K210-BURNER) -p $(K210-SERIALPORT) -b 1500000 $(KERNEL_BIN)
	python3 -m serial.tools.miniterm --eol LF --dtr 0 --rts 0 --filter direct $(K210-SERIALPORT) 115200
endif


monitor:
	riscv64-unknown-elf-gdb -ex 'file target/riscv64imac-unknown-none-elf/release/os' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'

gdb:
	@qemu-system-riscv64 -machine virt -nographic -bios $(BOOTLOADER) -device loader,\
	file=target/riscv64imac-unknown-none-elf/release/os,addr=0x80200000 -drive \
	file=$(U_FAT32),if=none,format=raw,id=x0 \
	-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 -smp threads=2 -S -s

runsimple:
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(U_FAT32),if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0\
		-smp threads=2

release: build
	@cp $(BOOTLOADER) $(BOOTLOADER).copy
	@dd if=$(KERNEL_BIN) of=$(BOOTLOADER).copy bs=$(K210_BOOTLOADER_SIZE) seek=1
	@mv $(BOOTLOADER).copy $(TOP)
