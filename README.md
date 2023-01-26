# NerdOS

![logo](https://raw.githubusercontent.com/pluveto/0images/master/2023/01/upgit_20230125_1674617889.png)

A hobbyist operating system written in Rust based on [equation314/nimbos](https://github.com/equation314/nimbos).

## TODO

- [ ] CFS scheduler
- [ ] FFI <-> alios implementation
- [ ] Multi-core support
- [ ] ...

## Development Environment

1. Clone the repository

    ```sh
    https://github.com/cargo-youth/nerdos
    ```

2. Switch `rust` toolchain to nightly

    ```sh
    rustup install nightly
    rustup default nightly
    ```

3. Install packages

    ```sh
    sudo apt install autoconf automake autotools-dev curl libmpc-dev libmpfr-dev libgmp-dev \
              gawk build-essential bison flex texinfo gperf libtool patchutils bc \
              zlib1g-dev libexpat-dev pkg-config  libglib2.0-dev libpixman-1-dev libsdl2-dev \
              git tmux python3 python3-pip ninja-build
    ```

4. Install `qemu`

    ```sh
    mkdir kit # Here we create a directory in the project root to store the tools
    pushd kit
    wget https://download.qemu.org/qemu-7.0.0.tar.xz
    tar -xf qemu-7.0.0.tar.xz
    cd qemu-7.0.0
    ./configure --target-list=x86_64-softmmu,aarch64-softmmu,riscv64-softmmu --enable-debug
    make -j$(nproc)
    popd
    ```

    And then add the following line to your `~/.bashrc`:

    ```sh
    export PATH=$PATH:/path_to_kit/qemu-7.0.0/build
    ```

    Here `path_to_kit` is the path to the `kit` directory. For example `/home/pluveto/workspace/playground/nerdos/kit/qemu-7.0.0/build`

5. Install `musl`

    Select your target architecture from [musl.cc](https://musl.cc/).

    ```sh
    pushd kit
    wget https://musl.cc/x86_64-linux-musl-cross.tgz
    tar -xf x86_64-linux-musl-cross.tgz
    popd
    ```

    And then add the following line to your `~/.bashrc`:

    ```sh
    export PATH=$PATH:/path_to_kit/x86_64-linux-musl-cross/bin
    ```

    Here `path_to_kit` is the path to the `kit` directory. For example `/home/pluveto/workspace/playground/nerdos/kit/x86_64-linux-musl-cross/bin`

6. Test `qemu` and `musl`

    Restart your terminal and run the following commands:

    ```sh
    qemu-system-x86_64 --version
    x86_64-linux-musl-gcc --version
    ```

## Build & Run (in QEMU)

```sh
cd kernel
make env
make run ARCH=x86_64 LOG=warn
# or 
make run ARCH=riscv64 LOG=warn
# or
make run ARCH=aarch64 LOG=warn
```

If you encounter any problems, try add a `VERBOSE=1` to the `make SOMETHING` command.

```sh
make run VERBOSE=1
```

## Troubleshooting

1. `make run` fails with `error: linker 'x86_64-linux-musl-gcc' not found`

    This is because the `x86_64-linux-musl-gcc` is not in your `PATH`. Or failed to build the musl toolchain.

1. `make run` fails with `Could not access KVM kernel module: No such file or directory`

    This is because you don't have KVM enabled. Try to enable it in your BIOS.

    Or try to run the following command:

    ```sh
    sudo modprobe kvm-intel
    ```

    If it shows `modprobe: ERROR: could not insert 'kvm_intel': Operation not supported`, then you don't have KVM enabled. Try to enable it or switch to another architecture.

1. `make run` fails with `riscv64-linux-musl-gcc: No such file or directory`

    This is because you don't have the riscv64 musl toolchain installed. Try to install it with the following command:

    ```sh
    pushd kit
    wget https://musl.cc/riscv64-linux-musl-cross.tgz
    tar -xf riscv64-linux-musl-cross.tgz
    popd
    ```

    And then add the following line to your `~/.bashrc`:

    ```sh
    export PATH=$PATH:/path_to_kit/riscv64-linux-musl-cross/bin
    ```

    Here `path_to_kit` is the path to the `kit` directory. For example `/home/pluveto/workspace/playground/nerdos/kit/riscv64-linux-musl-cross/bin`

2. ` install qemu ` fails with ` gcc version is too low `(centos) .Try to update it with the following command:
    ```sh
    yum install centos-release-scl
    scl enable devtoolset-8 bash
    ```
    And then you can check its version by  running  the following commands:
    ```sh
    gcc -v
    ```

## Code tree guide (Take riscv as an example)
    ```
        |-- arch(arch related)
    |   |-- aarch64
    |   |   |-- config.rs
    |   |   |-- context.rs
    |   |   |-- instructions.rs
    |   |   |-- mod.rs
    |   |   |-- page_table.rs
    |   |   |-- percpu.rs
    |   |   |-- trap.rs
    |   |   `-- trap.S
    |   |-- mod.rs
    |   |-- riscv(riscv architecture related)
    |   |   |-- config.rs(configuration files, kernel and user address space settings, and the sv39 paging mechanism)
    |   |   |-- context.rs(Context related Registers and Processing Logic)
    |   |   |-- instructions.rs
    |   |   |-- macros.rs
    |   |   |-- mod.rs
    |   |   |-- page_table.rs (page mechanism)
    |   |   |-- percpu.rs
    |   |   |-- trap.rs (trap logic processing)
    |   |   `-- trap.S (trap assembly logic)
    |   `-- x86_64
    |       |-- config.rs
    |       |-- context.rs
    |       |-- gdt.rs
    |       |-- idt.rs
    |       |-- instructions.rs
    |       |-- mod.rs
    |       |-- page_table.rs
    |       |-- percpu.rs
    |       |-- syscall.rs
    |       |-- syscall.S
    |       |-- trap.rs
    |       `-- trap.S
    |-- drivers
    |   |-- interrupt
    |   |   |-- apic.rs
    |   |   |-- gicv2.rs
    |   |   |-- i8259_pic.rs
    |   |   |-- mod.rs
    |   |   `-- riscv_intc.rs (riscv interrupt processing related)
    |   |-- misc
    |   |   |-- mod.rs
    |   |   |-- psci.rs
    |   |   |-- qemu_x86_reset.rs
    |   |   `-- sbi.rs (sbi support)
    |   |-- mod.rs
    |   |-- timer (clock)
    |   |   |-- arm_generic_timer.rs
    |   |   |-- mod.rs
    |   |   |-- riscv.rs (riscv clock setting)
    |   |   |-- x86_common.rs
    |   |   |-- x86_hpet.rs
    |   |   `-- x86_tsc.rs
    |   `-- uart
    |       |-- mod.rs
    |       |-- pl011.rs
    |       |-- riscv.rs (Input and output in riscv)
    |       `-- uart16550.rs
    |-- mm (address space)
    |   |-- address.rs (physical/virtual)
    |   |-- frame_allocator.rs (Physical Page Frame Allocator)
    |   |-- heap_allocator.rs (kernel dynamic memory allocation)
    |   |-- memory_set.rs  (introducing address spaces and logical segments, etc.)
    |   |-- mod.rs (mm initialization method)
    |   |-- paging.rs (Page table abstraction and other content such as establishing and dismantling mapping relationship unmap and map)
    |   `-- uaccess.rs
    |-- platform
    |   |-- config.rs
    |   |-- mod.rs
    |   |-- pc
    |   |   |-- mod.rs
    |   |   |-- multiboot.rs
    |   |   `-- multiboot.S
    |   |-- qemu_virt_arm
    |   |   `-- mod.rs
    |   `-- qemu_virt_riscv
    |       `-- mod.rs
    |-- sync (synchronous mutex module)
    |   |-- lazy_init.rs
    |   |-- mod.rs
    |   |-- mutex.rs (todo)
    |   |-- percpu.rs
    |   `-- spin.rs
    |-- syscall
    |   |-- fs.rs (sys_read&sys_write)
    |   |-- mod.rs (syscall dispatch processing)
    |   |-- task.rs (sys_getpid/fork/exec/waitpid/exit/clone)
    |   `-- time.rs (current_time)
    |-- task
    |   |-- manager.rs  (task manager)
    |   |-- mod.rs
    |   |-- schedule (rr scheduling, and then we need to implement cfs scheduling)
    |   |   |-- mod.rs
    |   |   `-- round_robin.rs
    |   |-- structs.rs (task related status)
    |   `-- wait_queue.rs 
    `-- utils
        |-- allocator.rs
        |-- irq_handler.rs
        |-- mod.rs
        |-- ratio.rs
        `-- timer_list.rs
    |-- config.rs (Configuration related includes memory size, cpu number, scheduling related)
    |-- lang_items.rs (panic processing logic)
    |-- loader.rs (app loads memory and manages it)
    |-- logging.rs (multi-level log and color output)
    |-- main.rs (main function)
    |-- timer.rs
    |-- percpu.rs   (cpu logic)
    ```
