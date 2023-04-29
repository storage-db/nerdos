## nezha_sdc

目前的使用方法：

```rust
pub fn sdc_init(){
    nezha_sdc::sdcard_init();
    BLK_DRIVERS.write().push(Arc::new(SDCARD))
}
struct SDCARD;
impl BlockDriver for SDCARD {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> bool {
        //warn!("SDCARD Read block_id {} {}",block_id,buf.len());
        let mut cmd = MmcHost::new();
        cmd.set_data(buf.as_ptr() as *const _ as usize);
        unsafe{
            cmd.read_block(block_id as u32, buf.len() as u32 / 512);
        }
        true
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) -> bool {
        warn!("SDCARD Write Block_id {} {}",block_id,buf.len() as u32 / 512);
        let mut cmd = MmcHost::new();
        cmd.set_data(buf.as_ptr() as *const _ as usize);
        unsafe{
            cmd.write_block(block_id as u32, buf.len() as u32 / 512);
        }
        
        true
    }
}
```

