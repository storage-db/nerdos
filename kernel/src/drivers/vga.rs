
use vga::colors::{Color16, TextModeColor};
use vga::writers::{ScreenCharacter, Text80x25, TextWriter};
use crate::sync::Mutex;

pub fn init() {
    vga::vga::VGA.lock().set_memory_start(0xffffff80000a0000);

    let text_mode = Text80x25::new();
    let color = TextModeColor::new(Color16::Yellow, Color16::Black);
    let screen_character_v = ScreenCharacter::new(b'V', color);
    let screen_character_g = ScreenCharacter::new(b'G', color);
    let screen_character_a = ScreenCharacter::new(b'A', color);
    //0xffff_ff80_420b_8000
    text_mode.set_mode();
    text_mode.clear_screen();
    text_mode.write_character(0, 0, screen_character_v);
    text_mode.write_character(1, 0, screen_character_g);
    text_mode.write_character(2, 0, screen_character_a);
}

pub struct VGAStatus {
    text_mode: Text80x25,
    color: TextModeColor,
    chars: [u8; 80 * 25],
    pub max_cols: usize,
    pub max_rows: usize,
    // current output position
    pub pos_x: u8,
    pub pos_y: u8,
}

lazy_static::lazy_static! {
    static ref VGA_STATUS: Mutex<VGAStatus> = Mutex::new(VGAStatus::default());
}

impl VGAStatus {
    pub fn default() -> VGAStatus {
        VGAStatus {
            color: TextModeColor::new(Color16::Yellow, Color16::Black),
            text_mode: Text80x25::new(),
            max_cols: 80,
            max_rows: 25,
            pos_x: 0,
            pos_y: 0,
            chars: [0; 80 * 25],
        }
    }

    pub fn set_pos(&mut self, x: u8, y: u8) {
        self.pos_x = x;
        self.pos_y = y;
    }

    pub fn get_pos(&self) -> (u8, u8) {
        (self.pos_x, self.pos_y)
    }

    // roll up all the lines
    pub fn roll_up(&mut self) {
        // clean
        self.text_mode.clear_screen();
        // roll up
        for i in 0..(self.max_rows - 1) {
            for j in 0..self.max_cols {
                self.chars[(i * self.max_cols + j) as usize] =
                    self.chars[((i + 1) * self.max_cols + j) as usize];
            }
        }

        // reprint all
        for i in 0..self.max_rows {
            for j in 0..self.max_cols {
                let screen_character =
                    ScreenCharacter::new(self.chars[(i * self.max_cols + j) as usize], self.color);
                self.text_mode.write_character(j, i, screen_character);
            }
        }
    }

    pub fn put_char(&mut self, c: u8) {
        let screen_character = ScreenCharacter::new(c, self.color);
        self.text_mode
            .write_character(self.pos_x as usize, self.pos_y as usize, screen_character);
        if c == b'\n' {
            self.pos_x = 0;
            self.pos_y += 1;
        } else {
            self.chars[(self.pos_y * self.max_cols as u8 + self.pos_x) as usize] = c;
            self.pos_x += 1;
        }

        if self.pos_x >= self.max_cols as u8 {
            self.pos_x = 0;
            self.pos_y += 1;
        }

        if self.pos_y >= self.max_rows as u8 {
            self.pos_y = 0;
            self.roll_up();
        }
    }
}

pub fn console_putchar(c: u8) {
    VGA_STATUS.lock().put_char(c);
}
