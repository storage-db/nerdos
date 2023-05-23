# NerdOS

![logo](https://raw.githubusercontent.com/pluveto/0images/master/2023/01/upgit_20230125_1674617889.png)

A hobbyist operating system written in Rust based on [equation314/nimbos](https://github.com/equation314/nimbos).



## Development Environment

1. Clone the repository

    ```sh
    git clone https://github.com/cargo-youth/nerdos
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
7. Install `Serial port support`
   ```sh
    pip3 install pyserial
    sudo apt install python3-serial
   ```
## Build & Run (in K210)

```sh
cd kernel
make env
make run BOARD=k210

```

If you encounter any problems, try add a `VERBOSE=1` to the `make SOMETHING` command.

```sh
make run VERBOSE=1
```

## Troubleshooting

1. `make run` fails with `error: linker 'x86_64-linux-musl-gcc' not found`


