//! Contains implementation of the Game Boy PPU.

use alloc::boxed::Box;
use alloc::vec::Vec;

use bitflags::bitflags;

/// Palette for the framebuffer.
/// 0 - White
/// 1 - Light Gray
/// 2 - Dark Gray
/// 3 - Black
/// Alpha is FF in all cases.
const COLOR_PALETTE: [u32; 4] = [0xFED018FF, 0xD35600FF, 0x5E1210FF, 0x0D0405FF];

bitflags! {
    /// Struct that represents the LCD control register.
    struct Lcdc: u8 {
        /// LCD display enable.
        const LCD_ENABLE = 0b1000_0000;

        /// Window tile map display select.
        const WINDOW_SELECT = 0b0100_0000;

        /// Window display enable.
        const WINDOW_ENABLE = 0b0010_0000;

        /// BG & Window tile data select.
        const TILE_DATA = 0b0001_0000;

        /// BG tile map display select.
        const BG_SELECT = 0b0000_1000;

        /// Sprite Size.
        const SPRITE_SIZE = 0b0000_0100;

        /// Sprite Display Enable.
        const SPRITE_ENABLE = 0b0000_0010;

        /// BG/Window Enable.
        const BG_WIN_ENABLE = 0b0000_0001;
    }
}

bitflags! {
    /// Struct that represents the STAT register.
    struct Stat: u8 {
        /// LYC = LY coincidence interrupt.
        const COINCIDENCE_INT = 0b0100_0000;

        /// OAM search interrupt.
        const OAM_INT = 0b0010_0000;

        /// VBLANK interrupt.
        const VBLANK_INT = 0b0001_0000;

        /// HBLANK interrupt.
        const HBLANK_INT = 0b0000_1000;

        /// LY and LYC coincidence flag.
        const COINCIDENCE_FLAG = 0b0000_0100;
    }
}

/// Represents sprite data as stored in OAM.
#[derive(Clone, Copy)]
struct Sprite {
    /// The Y coordinate of the sprite.
    y: u8,

    /// The X coordinate of the sprite.
    x: u8,

    /// The tile number of the sprite.
    tile_number: u8,

    /// Sprite flags like x flip, etc...
    flags: u8,
}

/// Enumerates all the different modes the PPU can be in.
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum PpuModes {
    HBlank = 0,
    VBlank,
    OamSearch,
    Drawing,
}

/// Implementation of the Game Boy PPU.
pub struct Ppu {
    /// 8 KB Video RAM
    /// Mapped to 0x8000 to 0x9FFF.
    vram: Box<[u8; 0x2000]>,

    /// Object Attribute Map RAM.
    /// Mapped to 0xFE00 to 0xFE9F.
    oam: Box<[u8; 0xA0]>,

    /// The current scanline the PPU is rendering.
    ly: u8,

    /// LY Compare register.
    lyc: u8,

    /// The LCD control register.
    lcdc: Lcdc,

    /// LCD status register.
    stat: Stat,

    /// Scroll Y register.
    scy: u8,

    /// Scroll X register.
    scx: u8,

    /// Background palette data.
    bgp: u8,

    /// Object palette 0
    obp0: u8,

    /// Object palette 1
    obp1: u8,

    /// Window X coordinate - 7.
    wx: u8,

    /// Window Y coordinate.
    wy: u8,

    /// Internal window line counter.
    /// If a window is enabled, disabled and then enabled again
    /// the window rendering will continue off from the line that it
    /// last rendered.
    /// This is reset after every frame.
    window_line: u8,

    /// The current mode the PPU is in.
    current_mode: PpuModes,

    /// Total cycles ticked under the current mode.
    total_cycles: u32,

    /// RGBA32 framebuffer, this is the back buffer.
    back_framebuffer: Box<[u8; 160 * 144 * 4]>,

    /// RGBA32 framebuffer, this is the front buffer.
    pub front_framebuffer: Box<[u8; 160 * 144 * 4]>,
}

impl Ppu {
    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],

            0xFF40 => self.lcdc.bits(),
            0xFF41 => (self.current_mode as u8) | self.stat.bits() | (1 << 7),
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF47 => self.bgp,
            0xFF48 => self.obp0,
            0xFF49 => self.obp1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,

            _ => unreachable!(),
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = value,

            0xFF40 => self.lcdc = Lcdc::from_bits_truncate(value),
            0xFF41 => self.stat = Stat::from_bits_truncate(value),
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => {}
            0xFF45 => self.lyc = value,
            0xFF47 => self.bgp = value,
            0xFF48 => self.obp0 = value,
            0xFF49 => self.obp1 = value,
            0xFF4A => self.wy = value,
            0xFF4B => self.wx = value,

            _ => unreachable!(),
        }
    }

    /// Create a new `Ppu` instance.
    pub fn new() -> Self {
        let mut ppu = Self {
            vram: Box::new([0; 0x2000]),
            oam: Box::new([0; 0xA0]),
            ly: 0,
            lyc: 0,
            lcdc: Lcdc::from_bits_truncate(0x91),
            stat: Stat::empty(),
            scy: 0,
            scx: 0,
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wx: 0,
            wy: 0,
            window_line: 0,
            current_mode: PpuModes::OamSearch,
            total_cycles: 0,
            back_framebuffer: Box::new([0; 160 * 144 * 4]),
            front_framebuffer: Box::new([0; 160 * 144 * 4]),
        };

        // Fill in the shade 0b00 into the framebuffer.
        let colour_bytes = COLOR_PALETTE[0].to_be_bytes();

        for pixel in ppu.back_framebuffer.chunks_exact_mut(4) {
            pixel.copy_from_slice(&colour_bytes);
        }

        for pixel in ppu.front_framebuffer.chunks_exact_mut(4) {
            pixel.copy_from_slice(&colour_bytes);
        }

        ppu
    }

    /// Change the PPU's current mode.
    fn change_mode(&mut self, mode: PpuModes, if_reg: &mut u8) {
        match &mode {
            PpuModes::HBlank => {
                self.current_mode = mode;

                // Render the scanline.
                self.render_scanline();

                // Request STAT interrupt if
                // the appropriate bit is set.
                if self.stat.contains(Stat::HBLANK_INT) {
                    *if_reg |= 0b0000_0010;
                }
            }

            PpuModes::VBlank => {
                self.current_mode = mode;

                // Request VBlank interrupt.
                *if_reg |= 0b0000_0001;

                // Request STAT interrupt if
                // the appropriate bit is set.
                if self.stat.contains(Stat::VBLANK_INT) {
                    *if_reg |= 0b0000_0010;
                }
            }

            PpuModes::OamSearch => {
                self.current_mode = mode;

                // Request STAT interrupt if
                // the appropriate bit is set.
                if self.stat.contains(Stat::OAM_INT) {
                    *if_reg |= 0b0000_0010;
                }
            }

            PpuModes::Drawing => {
                self.current_mode = mode;
            }
        }
    }

    /// Compare LY and LYC, set bits and trigger interrupts.
    fn compare_lyc(&mut self, if_reg: &mut u8) {
        if self.ly == self.lyc {
            self.stat.insert(Stat::COINCIDENCE_FLAG);

            if self.stat.contains(Stat::COINCIDENCE_INT) {
                *if_reg |= 0b0000_0010;
            }
        } else {
            self.stat.remove(Stat::COINCIDENCE_FLAG);
        }
    }

    /// Render the current scanline.
    fn render_scanline(&mut self) {
        self.render_background();
        self.render_sprites();
    }

    /// Tick the PPU by 1 M cycle.
    pub fn tick(&mut self, if_reg: &mut u8) {
        if !self.lcdc.contains(Lcdc::LCD_ENABLE) {
            return;
        }

        self.total_cycles += 4;

        // The actual PPU timings are not fixed.
        // They vary depending upon the number of sprites
        // on the screen, if the window is being drawn etc..
        match self.current_mode {
            PpuModes::OamSearch if self.total_cycles >= 80 => {
                self.total_cycles -= 80;
                self.change_mode(PpuModes::Drawing, if_reg);
            }

            PpuModes::Drawing if self.total_cycles >= 172 => {
                self.total_cycles -= 172;
                self.change_mode(PpuModes::HBlank, if_reg);
            }

            PpuModes::HBlank if self.total_cycles >= 204 => {
                self.total_cycles -= 204;
                self.ly += 1;

                // LY 0x90 (144) signals end of one complete frame.
                if self.ly == 0x90 {
                    self.change_mode(PpuModes::VBlank, if_reg);
                } else {
                    self.change_mode(PpuModes::OamSearch, if_reg);
                }

                self.compare_lyc(if_reg);
            }

            PpuModes::VBlank if self.total_cycles >= 456 => {
                self.total_cycles -= 456;
                self.ly += 1;

                // The PPU actually has 154 lines instead of 144.
                // These 10 lines are `psuedo lines` of sorts.
                if self.ly == 154 {
                    // Swap the copy the back buffer to the front buffer.
                    self.front_framebuffer
                        .copy_from_slice(self.back_framebuffer.as_ref());

                    self.ly = 0;
                    self.change_mode(PpuModes::OamSearch, if_reg);
                }

                self.compare_lyc(if_reg);
            }

            _ => {}
        }
    }

    /// Draw a pixel in the framebuffer at the given `x` and `y`
    /// coordinates.
    fn draw_pixel(&mut self, x: u8, y: u8, colour: u32) {
        let offset = (y as usize * 160 * 4) + x as usize * 4;
        let bytes = colour.to_be_bytes();

        self.back_framebuffer[offset..offset + 4].copy_from_slice(&bytes);
    }

    /// Gets the colour of a particular pixel at the given `x` and `y`
    /// coordinates.
    fn get_pixel(&self, x_coord: u8, y_coord: u8) -> u32 {
        let offset = (y_coord as usize * 160 * 4) + x_coord as usize * 4;

        let r = self.back_framebuffer[offset];
        let g = self.back_framebuffer[offset + 1];
        let b = self.back_framebuffer[offset + 2];
        let a = self.back_framebuffer[offset + 3];

        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
    }

    // Render the background map with scroll OR the window map for this scanline.
    fn render_background(&mut self) {
        // The 0th bit of the LCDC when reset disables all forms
        // of background and window rendering.
        // (it also overrides the window enable bit)
        // Note: This does not affect sprite rendering.
        if !self.lcdc.contains(Lcdc::BG_WIN_ENABLE) {
            return;
        }

        // If this is a new frame, reset the window line counter.
        if self.ly == 0 {
            self.window_line = 0;
        }

        // The tile map that is going to be used to render
        // the window.
        let win_map = if self.lcdc.contains(Lcdc::WINDOW_SELECT) {
            0x1C00
        } else {
            0x1800
        };

        // The tile map that is going to be used to render
        // the background.
        let bgd_map = if self.lcdc.contains(Lcdc::BG_SELECT) {
            0x1C00
        } else {
            0x1800
        };

        // The tile data that is going to be used for rendering
        // the above tile maps.
        let tile_data = if self.lcdc.contains(Lcdc::TILE_DATA) {
            0x0000
        } else {
            0x1000
        };

        // If the window is enabled this line, we increment the internal line counter.
        let mut increment_window_counter = false;

        for x in 0u8..160u8 {
            // Extract the absolute X and Y coordinates of the pixel in the respective 256 x 256 tile map.
            let (map_x, map_y, tile_map) = if self.lcdc.contains(Lcdc::WINDOW_ENABLE)
                && self.wy <= self.ly
                && self.wx <= x + 7
            {
                let map_x = x.wrapping_add(7).wrapping_sub(self.wx);
                let map_y = self.window_line;

                increment_window_counter = true;

                (map_x, map_y, win_map)
            } else {
                let map_x = x.wrapping_add(self.scx);
                let map_y = self.ly.wrapping_add(self.scy);

                (map_x, map_y, bgd_map)
            };

            // Extract the X and Y coordinates of the pixel inside the
            // respective tile.
            let tile_x = map_x & 0x07;
            let tile_y = map_y & 0x07;

            // Extract the the tile number.
            // Each tile is 8 x 8 pixels, and
            // the background or window map is 32 by 32 tiles in size.
            // We first extract the index of the tile number.
            // The map has a range of 0x400 bytes and each row in the map has
            // 0x20 bytes.
            let tile_number_index =
                tile_map + (((map_y as u16 >> 3) << 5) & 0x3FF) + ((map_x as u16 >> 3) & 0x1F);

            let tile_number = self.vram[tile_number_index as usize];

            // Extract the address of the row we are rendering in the concerned tile.
            // There are two addressing modes,
            //
            // 1. 0x8000: (TILE_NUMBER as u8 * 16) + 0x8000.
            // 2. 0x8800: (TILE_NUMBER as i8 * 16) + 0x9000.
            let address = if tile_data == 0x0000 {
                tile_data + ((tile_number as u16) << 4) + (tile_y << 1) as u16
            } else {
                tile_data
                    .wrapping_add(((tile_number as i8 as i16) as u16) << 4)
                    .wrapping_add((tile_y << 1) as u16)
            } as usize;

            // Extract the colour data pertaining to the row.
            let lsb = self.vram[address];
            let msb = self.vram[address + 1];

            // Extract the pixel colour as specified by the particular ROM's palette.
            let custom_colour =
                (((msb >> (7 - tile_x)) & 0x01) << 1) | ((lsb >> (7 - tile_x)) & 0x01);

            // Extract the actual pixel colour, that we are going to use.
            let actual_colour = COLOR_PALETTE[((self.bgp >> (custom_colour << 1)) & 0x03) as usize];

            self.draw_pixel(x, self.ly, actual_colour);
        }

        self.window_line += increment_window_counter as u8;
    }

    /// Render the sprites present on this scanline.
    fn render_sprites(&mut self) {
        // The 1st bit of LCDC controls whether OBJs (sprites)
        // are enabled or not.
        if !self.lcdc.contains(Lcdc::SPRITE_ENABLE) {
            return;
        }

        // If the 2nd bit of LCDC is reset the sprite's size is taken to
        // be 8 x 8 else it's 8 x 16.
        let sprite_size = if self.lcdc.contains(Lcdc::SPRITE_SIZE) {
            16
        } else {
            8
        };

        // Go through the OAM ram and search for all the sprites
        // that are visible in this scanline.
        // This is similar to what the PPU does in OAM search mode.
        //
        // The requirements for a sprite to be visible are,
        // 1. Y COORD <= LY
        // 2. Y COORD + SPRITE SIZE > LY
        let mut sprites = self
            .oam
            .chunks_exact(4)
            .filter_map(|entry| {
                if let [y, x, tile_number, flags] = *entry {
                    let y = y.wrapping_sub(16);
                    let x = x.wrapping_sub(8);

                    // In 8 x 16 sprite mode, the 0th bit of the tile number
                    // is ignored.
                    let tile_number = if sprite_size == 16 {
                        tile_number & 0xFE
                    } else {
                        tile_number
                    };

                    if y <= self.ly && self.ly < y.wrapping_add(sprite_size) {
                        Some(Sprite {
                            y,
                            x,
                            tile_number,
                            flags,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .take(10)
            .enumerate()
            .collect::<Vec<(usize, Sprite)>>();

        // Sort the sprites in a way that,
        //
        // 1. The sprite that has the lower X coordinate will draw
        //    over the sprite that has a higher X coordinate.
        // 2. The sprite that appeared earlier in the OAM RAM will draw
        //    over the sprite with same X coordinates.
        sprites.sort_by(|&a, &b| {
            use core::cmp::Ordering;

            let res = a.1.x.cmp(&b.1.x);

            if let Ordering::Equal = res {
                // X coordinates are equal,
                // therefore the one that appeared earlier wins.
                // BUT we reverse the order since we have to draw the sprite
                // over the other.
                a.0.cmp(&b.0).reverse()
            } else {
                // Here the lower X wins.
                // BUT we reverse the order since we have to draw the sprite
                // over the other.
                res.reverse()
            }
        });

        // Render the sprites.
        for (_, sprite) in sprites {
            // Extract sprite attributes.
            let attributes = sprite.flags;

            // Is the sprite flipped over the Y axis.
            let y_flip = (attributes & 0x40) != 0;

            // Is the sprite flipped over the X axis.
            let x_flip = (attributes & 0x20) != 0;

            // The palette used to render the sprite.
            let palette = if (attributes & 0x10) != 0 {
                self.obp1
            } else {
                self.obp0
            };

            // Should the sprite be drawn over the background layer.
            // If this is false, the sprite will only be drawn
            // if the colour of BG is NOT 1-3.
            let sprite_over_bg = (attributes & 0x80) == 0;

            // The row in the tile of the sprite.
            let tile_y = if y_flip {
                sprite_size - (self.ly - sprite.y + 1)
            } else {
                self.ly - sprite.y
            };

            // The address of the sprite tile.
            let address = (((sprite.tile_number as u16) << 4) + ((tile_y as u16) << 1)) as usize;

            // Extract the colour data pertaining to the row.
            let lsb = self.vram[address];
            let msb = self.vram[address + 1];

            for x in 0..8 {
                let actual_x = sprite.x.wrapping_add(x);

                if actual_x <= 160 {
                    // Get the index of the colour.
                    // 0 - Is always transparent for sprites.
                    let colour_index = if x_flip {
                        ((msb >> x & 0x01) << 1) | (lsb >> x & 0x01)
                    } else {
                        ((msb >> (7 - x) & 0x01) << 1) | (lsb >> (7 - x) & 0x01)
                    };

                    // Extract the actual RGBA colour.
                    let colour = COLOR_PALETTE[((palette >> (colour_index << 1)) & 0x03) as usize];

                    // We don't draw pixels that are transparent.
                    if colour_index != 0 {
                        if sprite_over_bg {
                            self.draw_pixel(actual_x, self.ly, colour);
                        } else if self.get_pixel(actual_x, self.ly) == COLOR_PALETTE[0] {
                            self.draw_pixel(actual_x, self.ly, colour)
                        }
                    }
                }
            }
        }
    }
}
