#![no_std]
extern crate alloc;
use crate::{
    generic::Peripherals,
    serial::{read_reg, write_reg},
};
use alloc::{boxed::Box, string::String};
use bitflags::bitflags;
use generic::SHMC;
use core::fmt;
use embedded_hal::prelude::_embedded_hal_serial_Write;
use peripheral::Timer;
use serial::Serial;
use spin::Mutex;
const GPIO_BASE: usize = 0x02000000;
const PF_CFG0: usize = 0xF0;
const PF_DRV0: usize = 0x104;
const PF_PULL0: usize = 0x114;
const HCLKBASE: usize = 0x2001000 + 0x84c;
const MCLKBASE: usize = 0x2001000 + 0x830;

bitflags! {
    pub struct SunxiMmcRing:u32{
        const SUNXI_MMC_RINT_RESP_ERROR		= (0x1 << 1);
        const SUNXI_MMC_RINT_COMMAND_DONE	=	(0x1 << 2);
        const SUNXI_MMC_RINT_DATA_OVER	=	(0x1 << 3);
        const SUNXI_MMC_RINT_TX_DATA_REQUEST	=	(0x1 << 4);
        const SUNXI_MMC_RINT_RX_DATA_REQUEST	=	(0x1 << 5);
        const SUNXI_MMC_RINT_RESP_CRC_ERROR	=	(0x1 << 6);
        const SUNXI_MMC_RINT_DATA_CRC_ERROR		=(0x1 << 7);
        const SUNXI_MMC_RINT_RESP_TIMEOUT	=	(0x1 << 8);
        const SUNXI_MMC_RINT_DATA_TIMEOUT	=	(0x1 << 9);
        const SUNXI_MMC_RINT_VOLTAGE_CHANGE_DONE=	(0x1 << 10);
        const SUNXI_MMC_RINT_FIFO_RUN_ERROR	=	(0x1 << 11);
        const SUNXI_MMC_RINT_HARD_WARE_LOCKED	=	(0x1 << 12);
        const SUNXI_MMC_RINT_START_BIT_ERROR	=	(0x1 << 13);
        const SUNXI_MMC_RINT_AUTO_COMMAND_DONE=	(0x1 << 14);
        const SUNXI_MMC_RINT_END_BIT_ERROR	=	(0x1 << 15);
        const SUNXI_MMC_RINT_SDIO_INTERRUPT	=	(0x1 << 16);
        const SUNXI_MMC_RINT_CARD_INSERT	=	(0x1 << 30);
        const SUNXI_MMC_RINT_CARD_REMOVE	=	(0x1 << 31);
        const SUNXI_MMC_RINT_INTERRUPT_ERROR_BIT =
            SunxiMmcRing::SUNXI_MMC_RINT_RESP_ERROR.bits |
            SunxiMmcRing::SUNXI_MMC_RINT_RESP_CRC_ERROR.bits |
            SunxiMmcRing::SUNXI_MMC_RINT_DATA_CRC_ERROR.bits |
            SunxiMmcRing::SUNXI_MMC_RINT_RESP_TIMEOUT.bits |
            SunxiMmcRing::SUNXI_MMC_RINT_DATA_TIMEOUT.bits |
            SunxiMmcRing::SUNXI_MMC_RINT_VOLTAGE_CHANGE_DONE.bits |
            SunxiMmcRing::SUNXI_MMC_RINT_FIFO_RUN_ERROR.bits |
            SunxiMmcRing::SUNXI_MMC_RINT_HARD_WARE_LOCKED.bits |
            SunxiMmcRing::SUNXI_MMC_RINT_START_BIT_ERROR.bits |
            SunxiMmcRing::SUNXI_MMC_RINT_END_BIT_ERROR.bits;
    }
    struct MmcResp:u32{
        const MMC_RSP_PRESENT = 1 << 0;
        const MMC_RSP_136     = 1 << 1;
        const MMC_RSP_CRC     = 1 << 2;
        const MMC_RSP_BUSY    = 1 << 3;
        const MMC_RSP_OPCODE  = 1 << 4;
        const MMC_RSP_NONE    = 0;
        const MMC_RSP_R1      =
            MmcResp::MMC_RSP_PRESENT.bits |
            MmcResp::MMC_RSP_CRC.bits |
            MmcResp::MMC_RSP_OPCODE.bits;
        const MMC_RSP_R1B     =
            MmcResp::MMC_RSP_PRESENT.bits |
            MmcResp::MMC_RSP_CRC.bits |
            MmcResp::MMC_RSP_OPCODE.bits |
            MmcResp::MMC_RSP_BUSY.bits;
        const MMC_RSP_R2      =
            MmcResp::MMC_RSP_PRESENT.bits |
            MmcResp::MMC_RSP_136.bits |
            MmcResp::MMC_RSP_CRC.bits;
        const MMC_RSP_R3      = MmcResp::MMC_RSP_PRESENT.bits;
        const MMC_RSP_R6      =
            MmcResp::MMC_RSP_PRESENT.bits |
            MmcResp::MMC_RSP_CRC.bits |
            MmcResp::MMC_RSP_OPCODE.bits;
        const MMC_RSP_R7      =
            MmcResp::MMC_RSP_PRESENT.bits |
            MmcResp::MMC_RSP_CRC.bits |
            MmcResp::MMC_RSP_OPCODE.bits;
    }
    struct MmcFlags:u32{
        const MMC_DATA_NONE  = 0;
        const MMC_DATA_READ  = 1 << 0;
        const MMC_DATA_WRITE = 1 << 1;
        const MMC_CMD_MANUAL = 1;
    }
    struct SdVersion:u32{
        const SD_VERSION_SD      = 0x20000;
        const SD_VERSION_SD_2    = 0x20000 | 0x20;
        const SD_VERSION_1_0     = 0x20000 | 0x10;
        const SD_VERSION_SD_1_10 = 0x20000 | 0x1a;
    }
}
lazy_static::lazy_static! {
    static ref LEGACY_STDIO: Mutex<Option<Box<Serial>>> =
        Mutex::new(Some(Box::new(Serial::new(0x0250_0000))));
        static ref RCA:Mutex<u32> = Mutex::new(0);
}
struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Some(stdio) = LEGACY_STDIO.lock().as_mut() {
            for byte in s.as_bytes() {
                stdio.write(*byte).unwrap();
            }
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.write_str(c.encode_utf8(&mut [0; 4]))
    }

    fn write_fmt(mut self: &mut Self, args: fmt::Arguments<'_>) -> fmt::Result {
        fmt::write(&mut self, args)
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    Stdout.write_fmt(args).unwrap();
}
#[macro_export(local_inner_macros)]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        //_print(core::format_args!(core::concat!($fmt, "\r\n") $(, $($arg)+)?));
    }
}
#[macro_export(local_inner_macros)]
macro_rules! print {
    ($($arg:tt)*) => ({
        _print(core::format_args!($($arg)*));
    });
}
#[macro_export(local_inner_macros)]
macro_rules! warn {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        _print(core::format_args!(core::concat!($fmt, "\r\n") $(, $($arg)+)?));
    }
}

pub struct MMC {
    pub ocr: u32,
    pub cid: [u32; 4],
    pub csd: [u32; 4],
    pub rca: u32,
    pub scr: [u32; 2],
    pub read_bl_len: u32,
    pub write_bl_len: u32,
    pub tran_speed: u32,
}
impl MMC {
    pub fn new() -> Self {
        MMC {
            ocr: 0,
            rca: 0,
            cid: [0; 4],
            csd: [0; 4],
            scr: [0; 2],
            read_bl_len: 0,
            write_bl_len: 0,
            tran_speed: 0,
        }
    }
}
#[derive(Debug)]
pub struct SdError(String);
impl fmt::Display for SdError{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}
pub struct MmcHost {
    cmdidx: u32,
    cmdarg: u32,
    resptype: MmcResp,
    resp0: [u32; 4],
    resp1: usize,
    flags: MmcFlags,
    blocks: u32,
    blocksize: u32,
    rca:u32,
    shmc:SHMC,
}
impl MmcHost {
    pub fn new() -> Self {
        Self {
            cmdidx: 0,
            cmdarg: 0,
            resptype: MmcResp::MMC_RSP_NONE,
            resp0: [0; 4],
            resp1: 0,
            flags: MmcFlags::MMC_DATA_NONE,
            blocks: 0,
            blocksize: 0,
            shmc:unsafe{Peripherals::steal().shmc},
            rca: 0,
        }
    }

    pub fn set_data(&mut self, data: usize) {
        self.resp1 = data;
    }

    pub unsafe fn mmc_rint_wait(&mut self,done_flag:SunxiMmcRing,timeout_msecs:u64) -> Result<(),SdError> {
        let timeout = Timer.get_us() + timeout_msecs;
        loop{
            let status_bits =self.shmc.rint.read().bits();
            if Timer.get_us() > timeout {
                return Err(SdError(String::from("rint time out")));
            }
            if status_bits & SunxiMmcRing::SUNXI_MMC_RINT_INTERRUPT_ERROR_BIT.bits() != 0 {
                self.error_recover();
                return Err(SdError(String::from("rint error")));
            }
            if status_bits & done_flag.bits() != 0 {
                break;
            }
        }
        
        Ok(())
    }

    pub unsafe fn error_recover(&mut self) {
        self.shmc.gctrl.write(|w|w.bits(0x7));
        loop{
            if self.shmc.gctrl.read().bits() & 0x7 == 0 {
                break;
            }
        }
        self.shmc.gctrl.write(|w|w.bits(0x7));
        mmc_updata_clk().unwrap();
        self.shmc.rint.write(|w|w.bits(0xFFFFFFFF));
        self.shmc.gctrl.write(|w|w.bits(self.shmc.gctrl.read().bits() | (1 << 1)));
    }

    pub unsafe fn send(&mut self, data: bool) -> Result<(),SdError>{

        println!("\n[CMD] SEND CMD {}", self.cmdidx & 0x3F);
        let mut cmdval = 0x8000_0000u32;
        /*
         * CMDREG
         * CMD[5:0]     : Command index
         * CMD[6]       : Has response
         * CMD[7]       : Long response
         * CMD[8]       : Check response CRC
         * CMD[9]       : Has data
         * CMD[10]      : Write
         * CMD[11]      : Steam mode
         * CMD[12]      : Auto stop
         * CMD[13]      : Wait previous over
         * CMD[14]      : About cmd
         * CMD[15]      : Send initialization
         * CMD[21]      : Update clock
         * CMD[31]      : Load cmd
         */
        if self.cmdidx == 0{
            cmdval |= 1 << 15;
        }
        if self.resptype.contains(MmcResp::MMC_RSP_PRESENT) {
            cmdval |= 1 << 6;
        }
        if self.resptype.contains(MmcResp::MMC_RSP_136) {
            cmdval |= 1 << 7;
        }
        if self.resptype.contains(MmcResp::MMC_RSP_CRC) {
            cmdval |= 1 << 8;
        }
        if data {
            cmdval |= (1 << 9) | (1 << 13);
            if self.flags.contains(MmcFlags::MMC_DATA_WRITE) {
                cmdval |= 1 << 10
            }
            if self.blocks > 1 {
                cmdval |= 1 << 12
            }
            self.shmc.blksz.write(|w|w.bits(self.blocksize));
            self.shmc.bytecnt.write(|w|w.bits(self.blocks * self.blocksize));
        } else {
            if (self.cmdidx == 12) && self.flags.contains(MmcFlags::MMC_CMD_MANUAL) {
                cmdval |= 1 << 14;
                cmdval &= !(1 << 13);
            }
        }
        self.shmc.arg.write(|w|w.bits(self.cmdarg));
        if !data {
            println!("CMDIDX: 0x{:x}", self.cmdidx | cmdval);
            
            self.shmc.cmd.write(|w|w.bits(self.cmdidx | cmdval));
        }

        if data {
            self.shmc.gctrl.write(|w|w.bits(self.shmc.gctrl.read().bits() | 0x80000000u32));
            self.shmc.cmd.write(|w|w.bits(self.cmdidx | cmdval));
            self.trans_data_by_cpu().unwrap();
            println!("GCTRL: 0x{:x}", self.shmc.gctrl.read().bits());
            println!("CMD: 0x{:x}", self.shmc.cmd.read().bits());
            println!("BtCnt: 0x{:x}", self.shmc.bytecnt.read().bits());
        }
        
        
        if let Err(err) =self.mmc_rint_wait(SunxiMmcRing::SUNXI_MMC_RINT_COMMAND_DONE, 0xffffff){
            return Err(err);
        }
        if data {
            if let Err(a) = self.mmc_rint_wait(match self.blocks > 1{
                true => SunxiMmcRing::SUNXI_MMC_RINT_AUTO_COMMAND_DONE,
                false => SunxiMmcRing::SUNXI_MMC_RINT_DATA_OVER,
            }, 
            0xffff){
                return Err(a);
            }
        }
        if self.resptype.bits & MmcResp::MMC_RSP_BUSY.bits != 0 {
            let timeout = crate::peripheral::Timer.get_us() + 0x4ffffff;
            loop {
                if crate::peripheral::Timer.get_us() > timeout {
                    return Err(SdError(String::from("time out")));
                }
                
                if self.shmc.status.read().bits() & (1 << 9) == 0 {
                    break;
                }
            }
        }
        if self.resptype.bits & MmcResp::MMC_RSP_136.bits != 0 {
            self.resp0[0] = self.shmc.resp3.read().bits();
            self.resp0[1] = self.shmc.resp2.read().bits();
            self.resp0[2] = self.shmc.resp1.read().bits();
            self.resp0[3] = self.shmc.resp0.read().bits();
        } else {
            self.resp0[0] = self.shmc.resp0.read().bits();
        }
        println!("SHMC_RINT:   0x{:x}", self.shmc.rint.read().bits());
        self.shmc.rint.write(|w|w.bits(0xFFFFFFFF));
        self.shmc.gctrl.write(|w|w.bits(self.shmc.gctrl.read().bits() | (1 << 1)));
        Ok(())
    }
    pub unsafe fn trans_data_by_cpu(&mut self) -> Result<usize,SdError>{
        let shmc = Peripherals::steal().shmc;
        let mut timeout = crate::peripheral::Timer.get_us() + 0xffffff;
        let byte_cnt = (self.blocks * self.blocksize) as usize;
        println!("bytecnt {:x}", byte_cnt >> 2);
        println!("SHMC_status: 0x{:x}", self.shmc.status.read().bits());
        let dst = self.resp1 as *mut u32;
        let status_bit = match self.flags {
            MmcFlags::MMC_DATA_READ => 1 << 2,
            MmcFlags::MMC_DATA_WRITE => 1 << 3,
            _ => 0,
        };
        for i in 0..(byte_cnt >> 2) {
            loop {
                if shmc.status.read().bits() & status_bit == 0 {
                    break;
                }
                if crate::peripheral::Timer.get_us() > timeout {
                    return  Err(SdError(String::from("translate data timeout")));
                }
            }
            match self.flags {
                MmcFlags::MMC_DATA_READ => {
                    dst.offset(i as isize)
                        .write_volatile(shmc.fifo.read().bits());
                }
                MmcFlags::MMC_DATA_WRITE => {
                    shmc.fifo
                        .write(|w| w.bits(dst.offset(i as isize).read_volatile()));
                }
                _ => return  Err(SdError(String::from("wrong data flag"))),
            }
            timeout = Timer.get_us() + 0xffffff;
        }
        Ok(byte_cnt)
    }
    pub unsafe fn ptr2val<T>(&self, offset: isize) -> T {
        (self.resp1 as *const T).offset(offset).read_volatile()
    }
    pub unsafe fn mmc_send_status(&mut self) -> Result<(),SdError>{
        // Timer.mdelay(10);
        self.cmdidx = 13;
        self.resptype = MmcResp::MMC_RSP_R1;
        self.cmdarg = *RCA.lock() << 16;
        let mut retries = 10;
        loop {
            if let Ok(_) = self.send(false) {
                if self.resp0[0] & 1 << 8 != 0 && self.resp0[0] & (0xf << 9) != 7 << 9 {
                    return Ok(());
                }
            } else {
                retries = retries - 1;
                if retries < 0{
                    return Err(SdError(String::from("Sdcard Busy")))
                }
            }
            Timer.udelay(1000)
        }
    }
    pub unsafe fn read_block(&mut self, start: u32, blkcnt: u32) {
        self.cmdidx = match blkcnt {
            1 => 17,
            _ => 18,
        };
        println!("CMDIDX: {} start: {}", self.cmdidx, start);
        self.resptype = MmcResp::MMC_RSP_R1;
        self.cmdarg = start;
        self.blocks = blkcnt;
        self.blocksize = 512;
        self.flags = MmcFlags::MMC_DATA_READ;
        self.send(true).unwrap();
        //self.print();
    }
    pub unsafe fn write_block(&mut self, start: u32, blkcnt: u32) {
        self.cmdidx = match blkcnt {
            1 => 24,
            _ => 25,
        };
        self.cmdarg = start;
        self.resptype = MmcResp::MMC_RSP_R1;
        self.blocks = blkcnt;
        self.blocksize = 512;
        self.flags = MmcFlags::MMC_DATA_WRITE;
        self.send(true).unwrap();
        Timer.mdelay(10);
        self.mmc_send_status().unwrap()
    }

    pub unsafe fn print(&self) {
        println!("Count:0x{:x}", self.blocks * self.blocksize * 4);
        let index = self.resp1 as *const u8;
        for i in 0..(self.blocks * self.blocksize) as isize {
            print!("0x{:<02x}, ", index.offset(i).read());
            if (i + 1) % 16 == 0 {
                print!("\r\n");
            }
        }
    }

    pub unsafe fn printinfo(&self) {
        println!("SHMC STATUS: 0x{:x}", self.shmc.status.read().bits());
        println!("SHMC CMD:    0x{:x}", self.shmc.cmd.read().bits());
        println!(
            "RESPONSES:   [0x{:x}, 0x{:x}, 0x{:x}, 0x{:x}]",
            self.resp0[0], self.resp0[1], self.resp0[2], self.resp0[3]
        );
    }
    pub unsafe fn clear(&mut self) {
        self.resp0.iter_mut().for_each(|f| *f = 0);

        println!("Count:0x{:x}", self.blocks * self.blocksize * 4);
        for i in 0..(self.blocks * self.blocksize) as usize {
            *((self.resp1 + i) as *mut u8) = 0;
        }
    }
}
pub unsafe fn _get_pll_periph0() -> u32 {
    let regv = read_reg::<u32>(0x02001000, 0x20);
    let n = ((regv >> 8) & 0xff) + 1;
    let m = ((regv >> 1) & 0x01) + 1;
    let p = ((regv >> 16) & 0x07) + 1;
    24 * n / m / p / 2
}
pub unsafe fn mmc_clk_io_onoff(onoff:bool,clk:u32){
    match onoff {
        true => {
            let rval = read_reg::<u32>(HCLKBASE, 0) | 1;
            write_reg(HCLKBASE, 0, rval);
            let rval = read_reg::<u32>(HCLKBASE, 0) | 1 << 16;
            write_reg(HCLKBASE, 0, rval);
            let rval = read_reg::<u32>(MCLKBASE,0) | 1 << 31;
            write_reg(MCLKBASE, 0, rval);
        },
        false => {
            let rval = read_reg::<u32>(MCLKBASE,0) & !(1 << 31);
            write_reg(MCLKBASE, 0, rval);
            let rval = read_reg::<u32>(HCLKBASE, 0) & !1;
            write_reg(HCLKBASE, 0, rval);
            let rval = read_reg::<u32>(HCLKBASE, 0) & !(1 << 16);
            write_reg(HCLKBASE, 0, rval);
        }
    }
    if clk > 0{
        let rval = read_reg::<u32>(MCLKBASE,0) & !(0x7fffffff);
        write_reg(MCLKBASE, 0, rval);
    }
}
pub unsafe fn mmc_updata_clk() -> Result<(),SdError>{
    let shmc = Peripherals::steal().shmc;
    let timeout = Timer.get_us() + 0xfffff;
    shmc.clkcr.write(|w|w.bits(shmc.clkcr.read().bits() | (1 << 31)));
    shmc.cmd.write(|w|w.bits((1u32 << 31) | (1 << 21) | (1 << 13)));
    loop {
        if shmc.cmd.read().bits() & 0x8000_0000u32 == 0
            && Timer.get_us() < timeout
        {
            break;
        }
    }
    if shmc.cmd.read().bits()  & 0x8000_0000u32 != 0 {
        return Err(SdError(String::from("update clock fail")));
    }
    shmc.clkcr.write(|w|w.bits(shmc.clkcr.read().bits() & !(1 << 31),));
    shmc.rint.write(|w|w.bits(0xFFFFFFFFu32));
    Ok(())
}
pub unsafe fn mmc_get_clock() -> u32 {
    let rval = read_reg::<u32>(MCLKBASE, 0);
    let m = rval & 0xf;
    let n = (rval >> 8) & 0x3;
    let src = (rval >> 24) & 0x3;
    let sclk_hz;
    if src == 0 {
        sclk_hz = 24000000;
    } else if src == 2 {
        sclk_hz = _get_pll_periph0() * 2 * 1000000; /* use 2x pll6 */
    } else {
        return 0;
    }
    sclk_hz / (1 << n) / (m + 1)
}
pub unsafe fn mmc_set_clock(clock: u32) {
    let shmc = Peripherals::steal().shmc;
    /* disable card clock */
    let rval = shmc.clkcr.read().bits() & !(1 << 16);
    shmc.clkcr.write(|w|w.bits(rval));
    /* updata clock */
    mmc_updata_clk().unwrap();

    /* disable mclk */
    write_reg(MCLKBASE, 0, 0u32);
    shmc.ntsr.write(|w|w.bits(shmc.ntsr.read().bits() | (1 << 31)));
    let src: u32;
    let sclk_hz: u32;
    if clock <= 4000000 {
        src = 0;
        sclk_hz = 24000000;
    } else {
        src = 2;
        sclk_hz = _get_pll_periph0() * 2 * 1000000;
    }
    let m;
    let n;
    let mut div = (2 * sclk_hz + clock) / (2 * clock);
    div = if div == 0 { 1 } else { div };
    if div > 128 {
        m = 1;
        n = 0;
    } else if div > 64 {
        n = 3;
        m = div >> 3;
    } else if div > 32 {
        n = 2;
        m = div >> 2;
    } else if div > 16 {
        n = 1;
        m = div >> 1;
    } else {
        n = 0;
        m = div;
    }

    write_reg(MCLKBASE, 0, (src << 24) | (n << 8) | (m - 1));

    /* re-enable mclk */
    write_reg(MCLKBASE, 0, read_reg::<u32>(MCLKBASE, 0) | (1u32 << 31));
    shmc.clkcr.write(|w|w.bits(shmc.clkcr.read().bits()& !(0xff)));
    /* update clock */
    mmc_updata_clk().unwrap();

    /* config delay */
    let odly = 0;
    let sdly = 0;
    let mut rval = shmc.drv_dl.read().bits();
    rval |= ((odly & 0x1) << 16) | ((odly & 0x1) << 17);
    write_reg(MCLKBASE, 0, read_reg::<u32>(MCLKBASE, 0) & !(1 << 31));
    shmc.drv_dl.write(|w|w.bits(rval));
    write_reg(MCLKBASE, 0, read_reg::<u32>(MCLKBASE, 0) | (1 << 31));

    let mut rval = shmc.ntsr.read().bits(); 
    rval &= !(0x3 << 4);
    rval |= (sdly & 0x30) << 4;
    shmc.ntsr.write(|w|w.bits(rval));
    /* Re-enable card clock */
    shmc.clkcr.write(|w|w.bits(shmc.clkcr.read().bits() | (0x1 << 16)));
    /* update clock */
    mmc_updata_clk().unwrap();
}

unsafe fn gpio_init() {
    let mut cfg = read_reg::<u32>(GPIO_BASE, PF_CFG0) & 0x11000000 | 0x222222;
    cfg &= !0x1000000;
    cfg |= 0x222222;
    write_reg(GPIO_BASE, PF_CFG0, cfg);
    let mut drv = read_reg::<u32>(GPIO_BASE, PF_DRV0);
    drv &= !0x1000000;
    drv |= 0x222222;
    write_reg(GPIO_BASE, PF_DRV0, drv);
    let pull = read_reg::<u32>(GPIO_BASE, PF_PULL0) & 0x11111000 | 0x555;
    write_reg(GPIO_BASE, PF_PULL0, pull);
}

unsafe fn __mmc_be32_to_cpu(x: u32) -> u32 {
    (0x000000ff & ((x) >> 24))
        | (0x0000ff00 & ((x) >> 8))
        | (0x00ff0000 & ((x) << 8))
        | (0xff000000 & ((x) << 24))
}

pub(crate) unsafe fn mmc_core_init() -> Result<(),SdError>{
    let shmc = Peripherals::steal().shmc;
    // step 1
    write_reg(HCLKBASE, 0, 0x10000);
    crate::peripheral::Timer.mdelay(1);
    write_reg(HCLKBASE, 0, read_reg::<u32>(HCLKBASE, 0) | (1u32));
    write_reg(MCLKBASE, 0, (1u32 << 31) | (2 << 8) | 14);
    shmc.thldc
        .write(|w| w.bits((512 << 16) | (1u32 << 2) | (1 << 0)));

    // step 2
    shmc.gctrl.write(|w| w.bits(0x7));
    loop {
        if shmc.gctrl.read().bits() & 0x7 == 0 {
            break;
        }
    }
    shmc.hwrst.write(|w| w.bits(1));
    shmc.hwrst.write(|w| w.bits(0));
    peripheral::Timer.udelay(1000);
    shmc.hwrst.write(|w| w.bits(1));
    peripheral::Timer.udelay(1000);
    shmc.thldc
        .write(|w| w.bits((512 << 16) | (1u32 << 2) | (1u32 << 0)));
    shmc.csdc.write(|w| w.bits(3));
    shmc.dbgc.write(|w| w.bits(0xdebu32));
    shmc.imask.write(|w| w.bits(0xFFCEu32));
    shmc.rint.write(|w| w.bits(0xFFFFFFFFu32));

    // step3
    let rval = shmc.clkcr.read().bits() | (0x1u32 << 16) | (0x1 << 31);
    shmc.clkcr.write(|w| w.bits(rval));
    shmc.cmd.write(|w| w.bits(0x80202000u32));
    crate::peripheral::Timer.mdelay(1);
    shmc.clkcr
        .write(|w| w.bits(shmc.clkcr.read().bits() & !(0x1 << 31)));
    let rval = shmc.gctrl.read().bits() & !(1u32 << 10);
    write_reg(MCLKBASE, 0, read_reg::<u32>(MCLKBASE, 0) & !(1 << 31));
    shmc.gctrl.write(|w|w.bits(rval));
    write_reg(MCLKBASE, 0, read_reg::<u32>(MCLKBASE, 0) | (1 << 31));
    // step4

    // 0 -> 8 -> 55, 41 -> 2 ? -> 3 -> 9 -> 7 ->55, 6 -> 9 -> 13 -> 7
    let buf = [0u8; 512 * 4];
    let mut cmdtmp = MmcHost::new();
    cmdtmp.set_data(buf.as_ptr() as *const _ as usize);
    let mut mmc = MMC::new();
    cmdtmp.send(false).unwrap();

    cmdtmp.cmdidx = 8;
    cmdtmp.cmdarg = (1u32 << 8) | 0xaa;
    cmdtmp.resptype = MmcResp::MMC_RSP_R7;
    cmdtmp.send(false).unwrap();
    loop {
        cmdtmp.cmdidx = 55;
        cmdtmp.cmdarg = 0;
        cmdtmp.resptype = MmcResp::MMC_RSP_R1;
        cmdtmp.send(false).unwrap();

        cmdtmp.cmdidx = 41;
        cmdtmp.cmdarg = 0xfe0000 & 0xff8000 | 0x40000000;
        cmdtmp.resptype = MmcResp::MMC_RSP_R3;
        cmdtmp.send(false).unwrap();
        if cmdtmp.resp0[0] & 0x8000_0000u32 != 0 {
            break;
        }
    }
    mmc.ocr = cmdtmp.resp0[0];

    // mmc_startup
    /* Put the Card in Identify Mode */
    cmdtmp.cmdidx = 2;
    cmdtmp.cmdarg = 0;
    cmdtmp.resptype = MmcResp::MMC_RSP_R2;
    cmdtmp.send(false).unwrap();
    cmdtmp.printinfo();
    mmc.cid = cmdtmp.resp0;
    cmdtmp.clear();
    /*
     * For MMC cards, set the Relative Address.
     * For SD cards, get the Relatvie Address.
     * This also puts the cards into Standby State
     */
    cmdtmp.cmdidx = 3;
    cmdtmp.cmdarg = 0;
    cmdtmp.resptype = MmcResp::MMC_RSP_R6;
    cmdtmp.send(false).unwrap();
    cmdtmp.printinfo();
    mmc.rca = cmdtmp.resp0[0] >> 16 & 0xFFFF;
    *RCA.lock() = mmc.rca;
    cmdtmp.rca = mmc.rca;
    cmdtmp.clear();

    /* Get the Card-Specific Data */
    cmdtmp.cmdidx = 9;
    cmdtmp.cmdarg = mmc.rca << 16;
    cmdtmp.resptype = MmcResp::MMC_RSP_R2;
    cmdtmp.send(false).unwrap();
    cmdtmp.printinfo();
    mmc.csd = cmdtmp.resp0;
    let frep = [10000, 100000, 1000000, 10000000][(cmdtmp.resp0[0] & 0x7) as usize];
    let mult = [
        0, 10, 12, 13, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60, 70, 80,
    ][((cmdtmp.resp0[0] >> 3) & 0x7) as usize];
    mmc.read_bl_len = 1 << ((mmc.csd[1] >> 16) & 0xf);
    mmc.write_bl_len = mmc.read_bl_len;
    mmc.tran_speed = frep * mult;

    cmdtmp.clear();

    /* Waiting for the ready status */
    cmdtmp.cmdidx = 13;
    cmdtmp.cmdarg = *RCA.lock() << 16;
    cmdtmp.resptype = MmcResp::MMC_RSP_R1;
    loop {
        cmdtmp.send(false).unwrap();
        cmdtmp.printinfo();
        if cmdtmp.resp0[0] & (1u32 << 8) != 0 {
            break;
        }
        crate::peripheral::Timer.mdelay(1);
        if cmdtmp.resp0[0] & !0x0206BF7F != 0 {
            return Err(SdError(String::from("mmc_status_error")));
        }
        cmdtmp.clear();
    }

    /* Select the card, and put it into Transfer Mode */
    cmdtmp.cmdidx = 7;
    cmdtmp.cmdarg = mmc.rca << 16;
    cmdtmp.resptype = MmcResp::MMC_RSP_R1B;
    cmdtmp.send(false).unwrap();
    cmdtmp.printinfo();
    mmc_set_clock(25000000);
    crate::peripheral::Timer.mdelay(1000);

    // /* change freq */
    cmdtmp.cmdidx = 55;
    cmdtmp.cmdarg = mmc.rca << 16;
    cmdtmp.resptype = MmcResp::MMC_RSP_R1;
    cmdtmp.clear();
    cmdtmp.send(false).unwrap();

    cmdtmp.cmdidx = 51;
    cmdtmp.resptype = MmcResp::MMC_RSP_R1;
    cmdtmp.cmdarg = 0;
    cmdtmp.blocks = 1;
    cmdtmp.blocksize = 8;
    cmdtmp.flags = MmcFlags::MMC_DATA_READ;
    cmdtmp.send(true).unwrap();
    mmc.scr[0] = __mmc_be32_to_cpu(cmdtmp.ptr2val::<u32>(0));
    mmc.scr[1] = __mmc_be32_to_cpu(cmdtmp.ptr2val::<u32>(1));

    if mmc.scr[0] & 0x40000 != 0 {
        println!("MMC_MODE_BIT 0x{:x}", mmc.scr[0]);
    }
    for _ in 0..4 {
        cmdtmp.cmdidx = 6;
        cmdtmp.resptype = MmcResp::MMC_RSP_R1;
        cmdtmp.cmdarg = 0 << 31 | 0xffffff;
        cmdtmp.cmdarg &= !(0xf << (0 * 4));
        cmdtmp.cmdarg |= 1 << (0 * 4);
        cmdtmp.blocks = 1;
        cmdtmp.blocksize = 64;
        cmdtmp.flags = MmcFlags::MMC_DATA_READ;
        cmdtmp.send(true).unwrap();
        if cmdtmp.ptr2val::<u32>(16) & 0xF == 1 {
            break;
        }
    }

    cmdtmp.cmdidx = 6;
    cmdtmp.resptype = MmcResp::MMC_RSP_R1;
    cmdtmp.cmdarg = 1 << 31 | 0xffffff;
    cmdtmp.cmdarg &= !(0xf << (0 * 4));
    cmdtmp.cmdarg |= 1 << (0 * 4);
    cmdtmp.blocks = 1;
    cmdtmp.blocksize = 64;
    cmdtmp.flags = MmcFlags::MMC_DATA_READ;
    cmdtmp.send(true).unwrap();
    if cmdtmp.ptr2val::<u8>(16) & 0xf != 1 {
        return Err(SdError(String::from("sdcard busy")));
    }

    mmc_updata_clk().unwrap();

    #[cfg(feature="iotest")]
    {
        let ind = cmdtmp.resp1 as *mut u32;
        for i in 0isize..128 {
            ind.offset(i).write(0x41424344)
        }
        for i in 128isize..256{
            ind.offset(i).write(0xABCDEF)
        }
        {
            cmdtmp.write_block(0, 1);
            cmdtmp.read_block(0, 1);
        }
        {
            cmdtmp.write_block(0,2);
            cmdtmp.write_block(2,2);
            cmdtmp.read_block(0, 4);
        }
        for _ in 0..10{
            cmdtmp.write_block(100, 1);
            cmdtmp.read_block(100, 1);
        }
    }
    Ok(())
}

pub fn sdcard_init() {
    unsafe {
        gpio_init();
        mmc_core_init().unwrap();
    }
}
mod generic {
    use core::marker;
    use core::marker::PhantomData;
    use core::ops::Deref;
    pub trait Readable {}
    pub trait Writable {}
    pub trait ResetValue {
        type Type;
        fn reset_value() -> Self::Type;
    }
    pub struct Reg<U, REG> {
        register: vcell::VolatileCell<U>,
        _marker: marker::PhantomData<REG>,
    }
    unsafe impl<U: Send, REG> Send for Reg<U, REG> {}
    impl<U, REG> Reg<U, REG>
    where
        Self: Readable,
        U: Copy,
    {
        #[inline(always)]
        pub fn read(&self) -> R<U, Self> {
            R {
                bits: self.register.get(),
                _reg: marker::PhantomData,
            }
        }
    }
    impl<U, REG> Reg<U, REG>
    where
        Self: ResetValue<Type = U> + Writable,
        U: Copy,
    {
        #[inline(always)]
        pub fn write<F>(&self, f: F)
        where
            F: FnOnce(&mut W<U, Self>) -> &mut W<U, Self>,
        {
            self.register.set(
                f(&mut W {
                    bits: Self::reset_value(),
                    _reg: marker::PhantomData,
                })
                .bits,
            );
        }
    }
    pub struct R<U, T> {
        pub(crate) bits: U,
        _reg: marker::PhantomData<T>,
    }
    impl<U, T> R<U, T>
    where
        U: Copy,
    {
        #[inline(always)]
        pub fn bits(&self) -> U {
            self.bits
        }
    }
    impl<U, T, FI> PartialEq<FI> for R<U, T>
    where
        U: PartialEq,
        FI: Copy + Into<U>,
    {
        #[inline(always)]
        fn eq(&self, other: &FI) -> bool {
            self.bits.eq(&(*other).into())
        }
    }
    pub struct W<U, REG> {
        pub(crate) bits: U,
        _reg: marker::PhantomData<REG>,
    }
    impl<U, REG> W<U, REG> {
        #[inline(always)]
        pub unsafe fn bits(&mut self, bits: U) -> &mut Self {
            self.bits = bits;
            self
        }
    }    pub struct SHMC {
        _marker: PhantomData<*const ()>,
    }
    unsafe impl Send for SHMC {}
    impl SHMC {
        pub const fn ptr() -> *const shmc::RegisterBlock {
            0x04020000u32 as *const _
        }
    }
    impl Deref for SHMC {
        type Target = shmc::RegisterBlock;

        fn deref(&self) -> &Self::Target {
            unsafe { &*SHMC::ptr() }
        }
    }
    pub mod shmc {
        use super::{Readable, Reg, Writable};
        #[repr(C)]
        pub struct RegisterBlock {
            pub gctrl: Register,
            pub clkcr: Register,
            pub timeout: Register,
            pub width: Register,
            pub blksz: Register,
            pub bytecnt: Register,
            pub cmd: Register,
            pub arg: Register,
            pub resp0: Register,
            pub resp1: Register,
            pub resp2: Register,
            pub resp3: Register,
            pub imask: Register,
            pub mint: Register,
            pub rint: Register,
            pub status: Register,
            pub ftrglevel: Register,
            pub funcsel: Register,
            pub cbcr: Register,
            pub bbcr: Register,
            pub dbgc: Register,
            pub csdc: Register,
            pub a12a: Register,
            pub ntsr: Register,
            pub res1: [Register; 6],
            pub hwrst: Register,
            pub res2: Register,
            pub dmac: Register,
            pub dlba: Register,
            pub idst: Register,
            pub idie: Register,
            pub chda: Register,
            pub cbda: Register,
            pub res3: [Register; 26],
            pub thldc: Register,
            pub sfc: Register,
            pub res4: Register,
            pub dsbd: Register,
            pub res5: [Register; 12],
            pub drv_dl: Register,
            pub samp_dl: Register,
            pub ds_dl: Register,
            pub res6: [Register; 45],
            pub fifo: Register,
        }
        pub struct _REGISTER;
        type Register = Reg<u32, _REGISTER>;
        impl Readable for Register {}
        impl Writable for Register {}
        pub mod register {
            use crate::generic::ResetValue;

            use super::Register;
            impl ResetValue for Register {
                type Type = u32;
                fn reset_value() -> Self::Type {
                    0
                }
            }
        }
    }
    pub struct Peripherals {
        pub shmc: SHMC,
    }
    impl Peripherals {
        pub unsafe fn steal() -> Self {
            Peripherals {
                shmc: SHMC {
                    _marker: PhantomData,
                },
            }
        }
    }
}
mod peripheral {
    pub struct Timer;
    impl Timer {
        pub fn get_arch_counter(&self) -> u64 {
            let mtime: u64;
            unsafe { core::arch::asm!("csrr {}, time", out(reg) mtime) }
            mtime
        }
        pub fn get_us(&self) -> u64 {
            self.get_arch_counter() / 24
        }
        pub fn udelay(&self, us: u64) {
            let mut t1: u64;
            let t2: u64;
            t1 = self.get_arch_counter();
            t2 = t1 + us * 24;
            loop {
                t1 = self.get_arch_counter();
                if t2 < t1 {
                    break;
                }
            }
        }
        pub fn mdelay(&self, ms: u64) {
            self.udelay(ms * 1000)
        }
    }
}
mod serial {
    use core::{
        convert::Infallible,
        ptr::{read_volatile, write_volatile},
    };
    use embedded_hal::serial::{Read, Write};
    pub const UART_THR: usize = 0;
    pub const UART_RBR: usize = 0;
    pub const UART_LSR: usize = 0x14;
    pub const UART_USR: usize = 0x7c;
    #[inline]
    pub unsafe fn write_reg<T>(addr: usize, offset: usize, val: T) {
        write_volatile((addr + offset) as *mut T, val);
    }

    #[inline]
    pub unsafe fn read_reg<T>(addr: usize, offset: usize) -> T {
        read_volatile((addr + offset) as *const T)
    }
    const SUNXI_UART_USR_RFNE: u32 = 0x04;
    pub struct Serial {
        uart: usize,
    }
    impl Serial {
        pub fn new(base: usize) -> Self {
            Self { uart: base }
        }
    }
    impl Read<u8> for Serial {
        type Error = Infallible;

        fn read(&mut self) -> nb::Result<u8, Self::Error> {
            if unsafe { (read_reg::<u32>(self.uart, UART_LSR) & (1 << 0)) != 0 } {
                Ok(unsafe { (read_reg::<u32>(self.uart, UART_RBR) & 0xff) as u8 })
            } else {
                Err(nb::Error::WouldBlock)
            }
        }
    }
    impl Write<u8> for Serial {
        type Error = Infallible;

        fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
            while unsafe { read_reg::<u32>(self.uart, UART_USR) & SUNXI_UART_USR_RFNE } == 0 {}
            unsafe { write_reg::<u32>(self.uart, UART_THR, word as u32) }
            Ok(())
        }

        fn flush(&mut self) -> nb::Result<(), Self::Error> {
            while unsafe { read_reg::<u32>(self.uart, UART_USR) & SUNXI_UART_USR_RFNE } == 0 {}
            Ok(())
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::sdcard_init;

    #[test]
    fn it_works() {
        sdcard_init();
        
    }
}
