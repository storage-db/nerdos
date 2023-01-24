# NerdOS

A hobbyist operating system written in Rust based on [equation314/nimbos](https://github.com/equation314/nimbos).

## Build & Run (in QEMU)

```sh
cd kernel
make env
make run ARCH=x86_64 LOG=warn
```
