#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![feature(asm_const, naked_functions)]
#![feature(panic_info_message, alloc_error_handler)]
#![feature(const_refs_to_cell)]
#![feature(const_maybe_uninit_zeroed)]
#![feature(get_mut_unchecked)]

extern crate alloc;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate log;

#[cfg(riscv)]
extern crate riscv;

#[macro_use]
mod logging;

mod arch;
mod config;
mod drivers;
mod loader;
mod mm;
mod percpu;
mod platform;
mod sync;
mod syscall;
mod task;
mod timer;
mod utils;
mod fs;
mod net;
#[path = "boards/qemu.rs"]
mod board;
#[cfg(not(test))]
mod lang_items;

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

const LOGO: &str = r"
                    ,--.                                      ,----..               
                    ,--.'|    ,---,.,-.----.       ,---,       /   /   \   .--.--.    
                    ,--,:  : |  ,'  .' |\    /  \    .'  .' `\    /   .     : /  /    '.  
                    ,`--.'`|  ' :,---.'   |;   :    \ ,---.'     \  .   /   ;.  \  :  /`. /  
                    |   :  :  | ||   |   .'|   | .\ : |   |  .`\  |.   ;   /  ` ;  |  |--`   
                    :   |   \ | ::   :  |-,.   : |: | :   : |  '  |;   |  ; \ ; |  :  ;_     
                    |   : '  '; |:   |  ;/||   |  \ : |   ' '  ;  :|   :  | ; | '\  \    `.  
                    '   ' ;.    ;|   :   .'|   : .  / '   | ;  .  |.   |  ' ' ' : `----.   \ 
                    |   | | \   ||   |  |-,;   | |  \ |   | :  |  ''   ;  \; /  | __ \  \  | 
                    '   : |  ; .''   :  ;/||   | ;\  \'   : | /  ;  \   \  ',  / /  /`--'  / 
                    |   | '`--'  |   |    \:   ' | \.'|   | '` ,/    ;   :    / '--'.     /  
                    '   : |      |   :   .':   : :-'  ;   :  .'       \   \ .'    `--'---'   
                    ;   |.'      |   | ,'  |   |.'    |   ,.'          `---`                 
                    '---'        `----'    `---'      '---'                                

                            _____  _____  _____  _____ 
                            / __  \|  _  |/ __  \|____ |
                            `' / /'| |/' |`' / /'    / /
                            / /  |  /| |  / /      \ \
                            ./ /___\ |_/ /./ /___.___/ /
                            \_____/ \___/ \_____/\____/ 
                                                        
                                                    
";

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    drivers::init_early();
    println!("{}", LOGO);
    println!(
        "\
        arch = {}\n\
        platform = {}\n\
        build_mode = {}\n\
        log_level = {}\n\
        ",
        option_env!("ARCH").unwrap_or(""),
        option_env!("PLATFORM").unwrap_or(""),
        option_env!("MODE").unwrap_or(""),
        option_env!("LOG").unwrap_or(""),
    );

    mm::init_heap_early();

    logging::init();

    info!("Logging is enabled.");

    arch::init();
    arch::init_percpu();
    percpu::init_percpu_early();

    mm::init();
    drivers::init();

    percpu::init_percpu();
    timer::init();
    task::init();
    loader::list_apps();
    // 问题在这里
    task::run();

}
