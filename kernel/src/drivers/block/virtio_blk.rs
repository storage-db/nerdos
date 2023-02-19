use super::BlockDevice;
use crate::sync::{UPSafeCell};
use virtio_drivers::{ VirtIOBlk, VirtIOHeader};
use crate::drivers::bus::virtio::VirtioHal;
#[allow(unused)]
const VIRTIO0: usize = 0x10001000;
pub struct VirtIOBlock(UPSafeCell<VirtIOBlk<'static, VirtioHal>>);


impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        unsafe {
            Self(UPSafeCell::new(
                VirtIOBlk::<VirtioHal>::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
            ))
        }
    }
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .exclusive_access()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .exclusive_access()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}
