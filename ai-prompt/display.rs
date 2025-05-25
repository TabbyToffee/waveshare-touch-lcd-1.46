mod init_cmd;

use init_cmd::LCD_INIT_CMD;

use heapless::Vec;
use esp_hal::{
    ledc::{
        channel,
        channel::ChannelIFace,
        timer,
        timer::TimerIFace,
        LowSpeed,
        LSGlobalClkSource,
        Ledc,
    },
    // spi,
    // spi::master::Spi,
    // i2c::master::{I2c, Config},
    peripherals::LEDC,
    gpio::{Io, GpioPin},
    time::Rate,
    spi::master::Spi,
    Async,
};
use embedded_hal::spi::SpiBus;
use embedded_graphics::{
    pixelcolor::{Rgb888},
    prelude::{Size, OriginDimensions},
    draw_target::DrawTarget,
    Pixel,
};
use esp_println::{dbg, println};

const COLOR_DEPTH: usize = 24; // 24-bit colour
const DISPLAY_WIDTH: u32 = 412;
const DISPLAY_HEIGHT: u32 = 412;
const BUFFER_SIZE: u32 = DISPLAY_WIDTH * DISPLAY_HEIGHT * 3; // 8-bits per colour

const LCD_OPCODE_WRITE_CMD: u32 = 0x02;

pub struct Spd2010<'a> {
    spi: Spi<'a, Async>,
    // spi: SPI,
    buffer: [u8; BUFFER_SIZE as usize],
}

impl<'a> Spd2010<'a>  {
    pub async fn new(spi: Spi<'a, Async>) -> Self {
        let mut display = Self {
            spi: spi,
            buffer: [0; BUFFER_SIZE as usize],
        };
        
        display
    }
}