use core::{cmp, ptr};

use super::Display;

static FONT: &[u8] = include_bytes!("../../../res/unifont.font");

pub struct DebugDisplay {
    pub(super) display: Display,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl DebugDisplay {
    pub(super) fn new(display: Display) -> DebugDisplay {
        let w = display.width / 8;
        let h = display.height / 16;
        DebugDisplay {
            display,
            x: 0,
            y: 0,
            w,
            h,
        }
    }

    pub fn write(&mut self, buf: &[u8]) {
        for &b in buf {
            if self.x >= self.w || b == b'\n' {
                self.x = 0;
                self.y += 1;
            }

            if self.y >= self.h {
                let new_y = self.h - 1;
                let d_y = self.y - new_y;

                self.scroll(d_y * 16);

                unsafe {
                    self.display.sync_screen();
                }

                self.y = new_y;
            }

            if b != b'\n' {
                self.char(self.x * 8, self.y * 16, b as char, 0xFFFFFF);

                unsafe {
                    self.display.sync(self.x * 8, self.y * 16, 8, 16);
                }

                self.x += 1;
            }
        }
    }

    /// Draw a character
    fn char(&mut self, x: usize, y: usize, character: char, color: u32) {
        if x + 8 <= self.display.width && y + 16 <= self.display.height {
            let phys_y = (self.display.offset_y + y) % self.display.height;
            let mut dst = unsafe {
                self.display
                    .data_mut()
                    .add(phys_y * self.display.stride + x)
            };

            let font_i = 16 * (character as usize);
            if font_i + 16 <= FONT.len() {
                for row in 0..16 {
                    let row_data = FONT[font_i + row];
                    for col in 0..8 {
                        if (row_data >> (7 - col)) & 1 == 1 {
                            unsafe {
                                *dst.add(col) = color;
                            }
                        }
                    }

                    let next_phys_y = (phys_y + row + 1) % self.display.height;
                    dst = unsafe {
                        self.display
                            .data_mut()
                            .add(next_phys_y * self.display.stride + x)
                    };
                }
            }
        }
    }

    /// Scroll the screen
    fn scroll(&mut self, lines: usize) {
        let lines = cmp::min(self.h * 16, lines); // clamp
        self.display.offset_y = (self.display.offset_y + lines) % self.display.height;

        // clear the new lines
        let start_y = (self.display.offset_y + self.h * 16 - lines) % self.display.height;
        for row in 0..lines {
            let y = (start_y + row) % self.display.height;
            unsafe {
                let ptr = self.display.data_mut().add(y * self.display.stride);
                ptr::write_bytes(ptr, 0, self.display.stride);
            }
        }
    }
}
