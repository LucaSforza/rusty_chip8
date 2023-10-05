pub const HEIGHT: usize = 32;
pub const WIDTH: usize = 64;
pub const REAL_HEIGHT: usize = 320;
pub const REAL_WIDTH: usize = 640;

pub struct Display {
    buf: Vec<Vec<bool>>,
}
impl Display {
    pub fn clear_display(&mut self) {
        self.buf.iter_mut().flatten().for_each(|val| *val = false)
    }

    pub fn add_sprite(&mut self, sprite: Sprite) -> bool {
        let mut collision = false;
        let x = sprite.get_x() as usize;
        let mut y = sprite.get_y() as usize;
        sprite.as_slice().iter().for_each(|byte| {
            let mut bit_mask: u8 = 0x80;
            let mut row_x = x;
            if y >= HEIGHT {
                return;
            }
            while bit_mask != 0 {
                if row_x >= WIDTH {
                    row_x = 0
                }
                let bit = (bit_mask & *byte) != 0;
                let result = bit ^ self.buf[y][row_x];
                if self.buf[y][row_x] & bit {
                    collision = true
                }
                self.buf[y][row_x] = result;
                row_x += 1;
                bit_mask >>= 1
            }
            y += 1;
        });
        collision
    }
    pub fn draw(&self, buf: &mut [u32]) {
        if buf.len() != REAL_HEIGHT * REAL_WIDTH {
            panic!("the buffer is incorrect")
        }

        for (n_row, row) in self.buf.iter().enumerate() {
            for (n_col, value) in row.iter().enumerate() {
                let start_pixel = (n_row * REAL_WIDTH * 10) + n_col * 10;
                let pixel_color = buf[start_pixel];
                let last_value = pixel_color != 0;
                if last_value ^ *value {
                    for row in 0..10 {
                        for col in 0..10 {
                            let i_pixel = start_pixel + (row * REAL_WIDTH) + col;
                            buf[i_pixel] = !pixel_color;
                        }
                    }
                }
            }
        }
    }
}
impl Default for Display {
    fn default() -> Self {
        Self {
            buf: vec![vec![false; WIDTH]; HEIGHT],
        }
    }
}

pub struct Sprite {
    pub x: u8,
    pub y: u8,
    pub bytes: Vec<u8>,
}
impl Sprite {
    pub fn from_slice(slice: &[u8], x: u8, y: u8) -> Sprite {
        if slice.len() > 15 {
            panic!("the slice must be of lenght between 0 and 15")
        }
        let mut bytes: Vec<u8> = vec![0; slice.len()];
        slice
            .iter()
            .enumerate()
            .for_each(|(i, byte)| bytes[i] = *byte);
        Sprite {
            bytes: bytes,
            x: x,
            y: y,
        }
    }
    pub fn as_slice(&self) -> &[u8] {
        self.bytes.as_slice()
    }
    pub fn get_x(&self) -> u8 {
        self.x
    }
    pub fn get_y(&self) -> u8 {
        self.y
    }
}
