set -e

pushd kernel
echo "Clean before build? [y/N]"
read -r answer
if [ "$answer" = "y" ]; then
    make clean
fi
make kernel
popd

IMAGE_NAME=kernel/target/x86_64/release/nerdos
GRUB_CFG=isofiles/boot/grub/grub.cfg
DIST_DIR=dist

# elf must have .boot section
if ! objdump -h $IMAGE_NAME | grep -q .boot; then
    echo "warning: $IMAGE_NAME does not have .boot section"
fi

cp $IMAGE_NAME isofiles/boot/nerdos.img

mkdir -p $DIST_DIR
grub-mkrescue -o $DIST_DIR/nerdos.iso isofiles

ABS_OUT_PATH=$(readlink -f $DIST_DIR/nerdos.iso)
echo "ISO image created at $ABS_OUT_PATH"

# qemu-system-x86_64  -cdrom $ABS_OUT_PATH -nographic
echo "Do you want to run the image in qemu? [y/N]"
read -r answer
if [ "$answer" = "y" ]; then
    qemu-system-x86_64  -cdrom $ABS_OUT_PATH -nographic
fi

echo "Compressing image..."
zip $DIST_DIR/nerdos.zip $DIST_DIR/nerdos.iso

echo -n "Size before:"
du -h $DIST_DIR/nerdos.iso
echo -n "Size after:"
du -h $DIST_DIR/nerdos.zip