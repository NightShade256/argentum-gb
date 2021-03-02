//! Implementation of the Sharp SM83 CPU core.

mod decode;
mod instructions;
mod registers;

use core::fmt::{Display, Formatter, Result};

use self::registers::Registers;
use crate::bus::Bus;

/// Enumerates all the states the CPU can be in.
pub enum CpuState {
    Halted,
    Running,
}

pub struct CPU {
    // All the registers associated with the CPU.
    pub reg: Registers,

    // The Interrupt Master Enable flag.
    // Interrupts are serviced iff this flag is enabled.
    ime: bool,

    // The state the CPU is in.
    state: CpuState,
}

impl Display for CPU {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let reg_one = format!(
            "A: {:02X} F: {:02X} B: {:02X} C: {:02X} D: {:02X}",
            self.reg.a,
            self.reg.f.bits(),
            self.reg.b,
            self.reg.c,
            self.reg.d
        );

        let reg_two = format!(
            "E: {:02X} H: {:02X} L: {:02X} SP: {:04X} PC: 00:{:04X}",
            self.reg.e, self.reg.h, self.reg.l, self.reg.sp, self.reg.pc,
        );

        write!(f, "{} {}", reg_one, reg_two)
    }
}

impl CPU {
    /// Create a new `CPU` instance.
    pub fn new() -> Self {
        Self {
            reg: Registers::new(),
            ime: false,
            state: CpuState::Running,
        }
    }

    /// Read a byte from the current PC address.
    pub fn imm_byte(&mut self, bus: &Bus) -> u8 {
        let value = bus.read_byte(self.reg.pc);
        self.reg.pc += 1;

        value
    }

    /// TODO
    /// Tick components by one M cycle.
    pub fn internal_cycle(&self, _bus: &mut Bus) {}

    /// Read a R16 by specifiying the group and its index.
    /// See wheremyfoodat's decoding opcode PDF.
    pub fn read_r16<const GROUP: u8>(&mut self, r16: u8) -> u16 {
        match GROUP {
            1 => match r16 {
                0 => self.reg.get_bc(),
                1 => self.reg.get_de(),
                2 => self.reg.get_hl(),
                3 => self.reg.sp,

                _ => unreachable!(),
            },

            2 => match r16 {
                0 => self.reg.get_bc(),
                1 => self.reg.get_de(),
                2 => {
                    let value = self.reg.get_hl();
                    self.reg.set_hl(value.wrapping_add(1));

                    value
                }
                3 => {
                    let value = self.reg.get_hl();
                    self.reg.set_hl(value.wrapping_sub(1));

                    value
                }

                _ => unreachable!(),
            },

            3 => match r16 {
                0 => self.reg.get_bc(),
                1 => self.reg.get_de(),
                2 => self.reg.get_hl(),
                3 => self.reg.get_af(),

                _ => unreachable!(),
            },

            _ => unreachable!(),
        }
    }

    /// Write a value to a R16 by specifiying the group and its index.
    /// See wheremyfoodat's decoding opcode PDF.
    pub fn write_r16<const GROUP: u8>(&mut self, r16: u8, value: u16) {
        match GROUP {
            1 => match r16 {
                0 => self.reg.set_bc(value),
                1 => self.reg.set_de(value),
                2 => self.reg.set_hl(value),
                3 => self.reg.sp = value,

                _ => unreachable!(),
            },

            2 => match r16 {
                0 => self.reg.set_bc(value),
                1 => self.reg.set_de(value),
                2 | 3 => self.reg.set_hl(value),

                _ => unreachable!(),
            },

            3 => match r16 {
                0 => self.reg.set_bc(value),
                1 => self.reg.set_de(value),
                2 => self.reg.set_hl(value),
                3 => self.reg.set_af(value),

                _ => unreachable!(),
            },

            _ => unreachable!(),
        }
    }

    /// Read a R8 by specifiying its index.
    /// See wheremyfoodat's decoding opcode PDF.
    pub fn read_r8(&mut self, bus: &Bus, r8: u8) -> u8 {
        match r8 {
            0 => self.reg.b,
            1 => self.reg.c,
            2 => self.reg.d,
            3 => self.reg.e,
            4 => self.reg.h,
            5 => self.reg.l,
            6 => bus.read_byte(self.reg.get_hl()),
            7 => self.reg.a,

            _ => unreachable!(),
        }
    }

    /// Write a value to R8 by specifiying its index.
    /// See wheremyfoodat's decoding opcode PDF.
    pub fn write_r8(&mut self, bus: &mut Bus, r8: u8, value: u8) {
        match r8 {
            0 => self.reg.b = value,
            1 => self.reg.c = value,
            2 => self.reg.d = value,
            3 => self.reg.e = value,
            4 => self.reg.h = value,
            5 => self.reg.l = value,
            6 => bus.write_byte(self.reg.get_hl(), value),
            7 => self.reg.a = value,

            _ => unreachable!(),
        }
    }
}