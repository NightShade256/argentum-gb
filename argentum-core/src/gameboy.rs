//! Wrapper struct to conviniently abstract the inner workings.

use crate::bus::Bus;
use crate::cpu::Cpu;

/// T-cycles to execute per frame.
const CYCLES_PER_FRAME: u32 = 70224;

pub struct GameBoy {
    bus: Bus,
    cpu: Cpu,
}

impl GameBoy {
    /// Create a new `GameBoy` instance.
    pub fn new(rom: &[u8]) -> Self {
        Self {
            bus: Bus::new(rom),
            cpu: Cpu::new(),
        }
    }

    /// Execute a frame's worth of instructions.
    pub fn execute_frame(&mut self) {
        let mut cycles = 0;

        while cycles <= CYCLES_PER_FRAME {
            cycles += self.cpu.execute_next(&mut self.bus);
        }
    }

    /// Get a reference to the framebuffer.
    pub fn get_framebuffer(&self) -> &[u8] {
        self.bus.ppu.framebuffer.as_ref()
    }

    pub fn skip_bootrom(&mut self) {
        self.cpu.skip_bootrom();
        self.bus.skip_bootrom();
    }
}
