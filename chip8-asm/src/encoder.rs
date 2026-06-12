use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    /// 00E0 — Clear display
    Cls,
    /// 00EE — Return from subroutine
    Ret,
    /// 1NNN — Jump to addr
    Jp(u16),
    /// 2NNN — Call subroutine at addr
    Call(u16),
    /// 3XKK — Skip if Vx == byte
    SeVb(u8, u8),
    /// 4XKK — Skip if Vx != byte
    SneVb(u8, u8),
    /// 5XY0 — Skip if Vx == Vy
    SeVV(u8, u8),
    /// 6XKK — Vx = byte
    LdVb(u8, u8),
    /// 7XKK — Vx += byte
    AddVb(u8, u8),
    /// 8XY0 — Vx = Vy
    LdVV(u8, u8),
    /// 8XY1 — Vx |= Vy
    Or(u8, u8),
    /// 8XY2 — Vx &= Vy
    And(u8, u8),
    /// 8XY3 — Vx ^= Vy
    Xor(u8, u8),
    /// 8XY4 — Vx += Vy, VF = carry
    AddVV(u8, u8),
    /// 8XY5 — Vx -= Vy, VF = not borrow
    Sub(u8, u8),
    /// 8XY6 — Vx >>= 1, VF = LSB
    Shr(u8),
    /// 8XY7 — Vx = Vy - Vx, VF = not borrow
    Subn(u8, u8),
    /// 8XYE — Vx <<= 1, VF = MSB
    Shl(u8),
    /// 9XY0 — Skip if Vx != Vy
    SneVV(u8, u8),
    /// 9XY0 — Skip if Vx != Vy
    LdI(u16),
    /// BNNN — Jump to addr + V0
    JpV0(u16),
    /// CXKK — Vx = random & byte
    Rnd(u8, u8),
    /// DXYN — Draw sprite at (Vx, Vy), height N
    Drw(u8, u8, u8),
    /// EX9E — Skip if key Vx pressed
    Skp(u8),
    /// EXA1 — Skip if key Vx not pressed
    Sknp(u8),
    /// FX07 — Vx = delay timer
    LdVdt(u8),
    /// FX0A — Wait key press, store in Vx
    LdK(u8),
    /// FX15 — delay timer = Vx
    LdDt(u8),
    /// FX18 — sound timer = Vx
    LdSt(u8),
    /// FX1E — I += Vx
    AddI(u8),
    /// FX29 — I = sprite addr for digit Vx
    LdF(u8),
    /// FX33 — BCD of Vx at I, I+1, I+2
    LdB(u8),
    /// FX55 — Store V0..Vx at I
    LdIV(u8),
    /// FX65 — Load V0..Vx from I
    LdVI(u8),
}

impl Instr {
    pub fn encode(&self) -> [u8; 2] {
        let op = match *self {
            Instr::Cls => 0x00E0u16,
            Instr::Ret => 0x00EE,
            Instr::Jp(addr) => 0x1000 | (addr & 0x0FFF),
            Instr::Call(addr) => 0x2000 | (addr & 0x0FFF),
            Instr::SeVb(x, kk) => 0x3000 | ((x as u16) << 8) | (kk as u16),
            Instr::SneVb(x, kk) => 0x4000 | ((x as u16) << 8) | (kk as u16),
            Instr::SeVV(x, y) => 0x5000 | ((x as u16) << 8) | ((y as u16) << 4),
            Instr::LdVb(x, kk) => 0x6000 | ((x as u16) << 8) | (kk as u16),
            Instr::AddVb(x, kk) => 0x7000 | ((x as u16) << 8) | (kk as u16),
            Instr::LdVV(x, y) => 0x8000 | ((x as u16) << 8) | ((y as u16) << 4),
            Instr::Or(x, y) => 0x8001 | ((x as u16) << 8) | ((y as u16) << 4),
            Instr::And(x, y) => 0x8002 | ((x as u16) << 8) | ((y as u16) << 4),
            Instr::Xor(x, y) => 0x8003 | ((x as u16) << 8) | ((y as u16) << 4),
            Instr::AddVV(x, y) => 0x8004 | ((x as u16) << 8) | ((y as u16) << 4),
            Instr::Sub(x, y) => 0x8005 | ((x as u16) << 8) | ((y as u16) << 4),
            Instr::Shr(x) => 0x8006 | ((x as u16) << 8) | ((x as u16) << 4),
            Instr::Subn(x, y) => 0x8007 | ((x as u16) << 8) | ((y as u16) << 4),
            Instr::Shl(x) => 0x800E | ((x as u16) << 8) | ((x as u16) << 4),
            Instr::SneVV(x, y) => 0x9000 | ((x as u16) << 8) | ((y as u16) << 4),
            Instr::LdI(addr) => 0xA000 | (addr & 0x0FFF),
            Instr::JpV0(addr) => 0xB000 | (addr & 0x0FFF),
            Instr::Rnd(x, kk) => 0xC000 | ((x as u16) << 8) | (kk as u16),
            Instr::Drw(x, y, n) => 0xD000 | ((x as u16) << 8) | ((y as u16) << 4) | (n as u16),
            Instr::Skp(x) => 0xE09E | ((x as u16) << 8),
            Instr::Sknp(x) => 0xE0A1 | ((x as u16) << 8),
            Instr::LdVdt(x) => 0xF007 | ((x as u16) << 8),
            Instr::LdK(x) => 0xF00A | ((x as u16) << 8),
            Instr::LdDt(x) => 0xF015 | ((x as u16) << 8),
            Instr::LdSt(x) => 0xF018 | ((x as u16) << 8),
            Instr::AddI(x) => 0xF01E | ((x as u16) << 8),
            Instr::LdF(x) => 0xF029 | ((x as u16) << 8),
            Instr::LdB(x) => 0xF033 | ((x as u16) << 8),
            Instr::LdIV(x) => 0xF055 | ((x as u16) << 8),
            Instr::LdVI(x) => 0xF065 | ((x as u16) << 8),
        };
        op.to_be_bytes()
    }

    pub fn mnemonic(&self) -> &'static str {
        match self {
            Instr::Cls => "CLS",
            Instr::Ret => "RET",
            Instr::Jp(_) => "JP",
            Instr::Call(_) => "CALL",
            Instr::SeVb(..) => "SE",
            Instr::SneVb(..) => "SNE",
            Instr::SeVV(..) => "SE",
            Instr::LdVb(..) => "LD",
            Instr::AddVb(..) => "ADD",
            Instr::LdVV(..) => "LD",
            Instr::Or(..) => "OR",
            Instr::And(..) => "AND",
            Instr::Xor(..) => "XOR",
            Instr::AddVV(..) => "ADD",
            Instr::Sub(..) => "SUB",
            Instr::Shr(_) => "SHR",
            Instr::Subn(..) => "SUBN",
            Instr::Shl(_) => "SHL",
            Instr::SneVV(..) => "SNE",
            Instr::LdI(_) => "LD",
            Instr::JpV0(_) => "JP",
            Instr::Rnd(..) => "RND",
            Instr::Drw(..) => "DRW",
            Instr::Skp(_) => "SKP",
            Instr::Sknp(_) => "SKNP",
            Instr::LdVdt(_) => "LD",
            Instr::LdK(_) => "LD",
            Instr::LdDt(_) => "LD",
            Instr::LdSt(_) => "LD",
            Instr::AddI(_) => "ADD",
            Instr::LdF(_) => "LD",
            Instr::LdB(_) => "LD",
            Instr::LdIV(_) => "LD",
            Instr::LdVI(_) => "LD",
        }
    }
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.encode();
        write!(f, "{:02X}{:02X}", bytes[0], bytes[1])
    }
}
