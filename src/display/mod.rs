mod init_cmd;
pub mod draw;

use draw::Spd2010;

use embassy_time::{Duration, Timer};
use init_cmd::LCD_INIT_CMD;

extern crate alloc;
use alloc::{boxed::Box, vec::Vec};

// use heapless::Vec;
use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::Rgb888,
    prelude::{OriginDimensions, Size},
    Pixel,
};
use embedded_hal::spi::SpiBus;
use esp_hal::{
    gpio::Io,
    i2c::master::I2c,
    ledc::{
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
        LSGlobalClkSource, Ledc, LowSpeed,
    },
    peripherals::{GPIO5, LEDC},
    spi::{
        master::{Address, Command, Spi, SpiDma, SpiDmaBus},
        DataMode,
    },
    time::Rate,
    Async, Blocking,
};
use esp_println::{dbg, println};

use crate::exio::{self, PinDirection, PinState};

const DISPLAY_WIDTH: u32 = 412;
const DISPLAY_HEIGHT: u32 = 412;
const DISPLAY_X_MAX: u32 = DISPLAY_WIDTH - 1;
const DISPLAY_Y_MAX: u32 = DISPLAY_HEIGHT - 1;
// const DISPLAY_WIDTH: u32 = 192;
// const DISPLAY_HEIGHT: u32 = 192;
const COLOR_BYTES: usize = 3;
// const COLOR_DEPTH: usize = 24; // 24-bit colour
const BUFFER_SIZE: u32 = DISPLAY_WIDTH * DISPLAY_HEIGHT * COLOR_BYTES as u32; // 3 bytes per pixel
pub const DMA_CHUNK_SIZE: usize = 1236;

// The format is [ OPCODE, 0, CMD, 0, 0, PARAMS ]

mod opcode {
    pub const WRITE_CMD: u8 = 0x02;
    pub const READ_CMD: u8 = 0x0B;
    pub const WRITE_COLOR: u8 = 0x32;
}

// const LCD_OPCODE_WRITE_CMD: u8 = 0x02;
// const LCD_OPCODE_READ_CMD: u8 = 0x0B;
// const LCD_OPCODE_WRITE_COLOR: u8 = 0x32;
// const PARAMS_MAX_LEN: u8 = 4;

const SPD2010_CMD_SET: u8 = 0xFF;
const SPD2010_CMD_SET_BYTE0: u8 = 0x20;
const SPD2010_CMD_SET_BYTE1: u8 = 0x10;
const SPD2010_CMD_SET_USER: u8 = 0x00;

const EXIO_LCD_RESET_PIN: u8 = 1;

// Guessed values
// const LCD_CMD_MADCTL: u8 = 0x36;
// const LCD_CMD_COLMOD: u8 = 0x3A; // 0x2A;

mod lcd_command {
    pub const NOP: u8 = 0x00; // This command is empty command
    pub const SWRESET: u8 = 0x01; // Software reset registers (the built-in frame buffer is not affected)
    pub const RDDID: u8 = 0x04; // Read 24-bit display ID
    pub const RDDST: u8 = 0x09; // Read display status
    pub const RDDPM: u8 = 0x0A; // Read display power mode
    pub const RDD_MADCTL: u8 = 0x0B; // Read display MADCTL
    pub const RDD_COLMOD: u8 = 0x0C; // Read display pixel format
    pub const RDDIM: u8 = 0x0D; // Read display image mode
    pub const RDDSM: u8 = 0x0E; // Read display signal mode
    pub const RDDSR: u8 = 0x0F; // Read display self-diagnostic result
    pub const SLPIN: u8 = 0x10; // Go into sleep mode (DC/DC, oscillator, scanning stopped, but keeps content)
    pub const SLPOUT: u8 = 0x11; // Exit sleep mode
    pub const PTLON: u8 = 0x12; // Turns on partial display mode
    pub const NORON: u8 = 0x13; // Turns on normal display mode
    pub const INVOFF: u8 = 0x20; // Recover from display inversion mode
    pub const INVON: u8 = 0x21; // Go into display inversion mode
    pub const GAMSET: u8 = 0x26; // Select Gamma curve for current display
    pub const DISPOFF: u8 = 0x28; // Display off (disable frame buffer output)
    pub const DISPON: u8 = 0x29; // Display on (enable frame buffer output)
    pub const CASET: u8 = 0x2A; // Set column address
    pub const RASET: u8 = 0x2B; // Set row address
    pub const RAMWR: u8 = 0x2C; // Write frame memory
    pub const RAMRD: u8 = 0x2E; // Read frame memory
    pub const PTLAR: u8 = 0x30; // Define the partial area
    pub const VSCRDEF: u8 = 0x33; // Vertical scrolling definition
    pub const TEOFF: u8 = 0x34; // Turns off tearing effect
    pub const TEON: u8 = 0x35; // Turns on tearing effect
    pub const MADCTL: u8 = 0x36; // Memory data access control
    pub const VSCSAD: u8 = 0x37; // Vertical scroll start address
    pub const IDMOFF: u8 = 0x38; // Recover from IDLE mode
    pub const IDMON: u8 = 0x39; // Fall into IDLE mode (8 color depth is displayed)
    pub const COLMOD: u8 = 0x3A; // Defines the format of RGB picture data
    pub const RAMWRC: u8 = 0x3C; // Memory write continue
    pub const RAMRDC: u8 = 0x3E; // Memory read continue
    pub const STE: u8 = 0x44; // Set tear scan line, tearing effect output signal when display reaches line N
    pub const GDCAN: u8 = 0x45; // Get scan line
    pub const WRDISBV: u8 = 0x51; // Write display brightness
    pub const RDDISBV: u8 = 0x52; // Read display brightness value
}


// pub struct Spd2010<'a> {
//     spi: Spi<'a, Async>,
//     // spi: SPI,
//     buffer: [u8; BUFFER_SIZE as usize],
// }

// impl<'a> Spd2010<'a> {
//     pub async fn new(spi: Spi<'a, Async>) -> Self {
//         // println!("0 !!!!");

//         // let buffer = [0; BUFFER_SIZE as usize];

//         // println!("1 !!!!");

//         // let myspi: Spi<Async> = spi;
//         // return;

//         let mut display = Self {
//             spi: spi,
//             buffer: [0; BUFFER_SIZE as usize],
//         };

//         // println!("Create display");

//         display
//     }

//     // pub fn init(&mut self) {
//     //     for (cmd, delay, data) in LCD_INIT_CMD {
//     //         let mut cmd32 = *cmd as u32;
//     //         cmd32 &= 0xff;
//     //         cmd32 <<= 8;
//     //         cmd32 |= LCD_OPCODE_WRITE_CMD << 24;
//     //         let mut full_data: Vec<u8, 10> = Vec::new();
//     //         full_data.push(*cmd);
//     //         full_data.extend_from_slice(*data);
//     //         self.spi.write(&full_data);
//     //         self.spi.flush();
//     //     }
//     // }

//     pub fn flush(&self) -> Result<(), core::convert::Infallible> {
//         // self.iface.send_bytes(&self.framebuffer)
//         Ok(())
//     }

//     pub fn draw(&mut self) {
//         let data = [128; 1000];
//         self.spi.write(&data);
//         self.spi.flush();
//     }
// }

// impl<SPI> DrawTarget for Spd2010<SPI> {
//     type Color = Rgb888;
//     type Error = core::convert::Infallible;

//     fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
//         where
//             I: IntoIterator<Item = Pixel<Self::Color>>,
//     {
//         // for Pixel(coord, color) in pixels.into_iter() {
//         //     // dbg!(color);
//         // }

//         Ok(())
//     }
// }

// impl<SPI> OriginDimensions for Spd2010<SPI> {
//     fn size(&self) -> Size {
//         Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT)
//     }
// }

pub fn backlight_init(ledc: &mut Ledc, backlight_pwm_pin: GPIO5) {
    // *ledc = Ledc::new(ledc_pin);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let mut backlight_timer = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    backlight_timer
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty13Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(5),
        })
        .unwrap();

    let mut channel0 = ledc.channel(channel::Number::Channel0, backlight_pwm_pin);
    channel0
        .configure(channel::config::Config {
            timer: &backlight_timer,
            duty_pct: 100,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .unwrap();
}

pub async fn reset(i2c: &mut I2c<'_, Blocking>) {
    println!("Reset display");
    exio::set_pin_direction(i2c, EXIO_LCD_RESET_PIN, PinDirection::Output);
    exio::set_pin(i2c, EXIO_LCD_RESET_PIN, PinState::Low);
    Timer::after(Duration::from_millis(100)).await;
    exio::set_pin(i2c, EXIO_LCD_RESET_PIN, PinState::High);
    Timer::after(Duration::from_millis(100)).await;
}

pub fn tx_command(qspi: &mut SpiDmaBus<'_, Blocking>, command: u8) {
    let address_value = (command as u32) << 8;
    qspi.half_duplex_write(
        DataMode::Single,
        Command::_8Bit(opcode::WRITE_CMD as u16, DataMode::Single),
        Address::_24Bit(address_value, DataMode::Single),
        0,
        &[],
    );
}

pub fn tx_command_data(qspi: &mut SpiDmaBus<'_, Blocking>, command: u8, data: &[u8]) {
    let address_value = (command as u32) << 8;
    qspi.half_duplex_write(
        DataMode::Single,
        Command::_8Bit(opcode::WRITE_CMD as u16, DataMode::Single),
        Address::_24Bit(address_value, DataMode::Single),
        0,
        data,
    );
}

pub fn tx_color(qspi: &mut SpiDmaBus<'_, Blocking>, command: u8, data: &[u8]) {
    let address_value = (command as u32) << 8;
    let result = qspi.half_duplex_write(
        DataMode::Quad,
        Command::_8Bit(opcode::WRITE_COLOR as u16, DataMode::Single),
        Address::_24Bit(address_value, DataMode::Single),
        0,
        data,
    );
    dbg!(result);
}

pub async fn test(qspi: &mut SpiDmaBus<'_, Blocking>) {
    // let result = spi.half_duplex_write(
    //     DataMode::Single,
    //     Command::_8Bit(LCD_OPCODE_WRITE_COLOR as u16, DataMode::Single),
    //     Address::_24Bit(0x5C, DataMode::Single),
    //     0,
    //     &[0x44],
    // );
    // dbg!(result);
    
    // return;
    
    // tx_command_data(
    //     spi,
    //     SPD2010_CMD_SET,
    //     &[
    //         SPD2010_CMD_SET_BYTE0,
    //         SPD2010_CMD_SET_BYTE1,
    //         SPD2010_CMD_SET_USER,
    //     ],
    // );
    // tx_command_data(spi, LCD_CMD_MADCTL, &[0x00]);

    // tx_command_data(spi, 0x0D, &[0x35]);

    // tx_command_data(spi, 0xFF, &[0x20, 0x10, 0x45]);
    // tx_command_data(spi, 0x01, &[0x9C]);
    // tx_command_data(spi, 0x03, &[0x9C]);

    // tx_command_data(spi, 0xFF, &[0x20, 0x10, 0x50]);
    // tx_command_data(spi, 0x05, &[0x08]);
    // tx_command_data(spi, 0xFF, &[0x20, 0x10, 0x00]);

    // tx_command_data(spi, 0xFF, &[0x20, 0x10, 0x50]);
    // tx_command_data(spi, 0x08, &[0x55]);

    // tx_command(spi, LCD_CMD_SWRESET);
    // Timer::after(Duration::from_millis(10)).await;
    // tx_command(spi, LCD_CMD_SLPOUT); // Exit sleep
    // Timer::after(Duration::from_millis(120)).await;
    
    // // 24 bit color
    // tx_command_data(spi, LCD_CMD_COLMOD, &[0x77]);
    // Timer::after(Duration::from_millis(5)).await;
    
    // // Set tear scan and enable
    // // tx_command_data(spi, LCD_CMD_STE, &[0x01, 0xC5]);
    // // tx_command_data(spi, LCD_CMD_STE, &[0x44]);
    // tx_command(spi, LCD_CMD_TEOFF);
    
    // tx_command_data(spi, LCD_CMD_MADCTL, &[0x00]);
    
    // tx_command(spi, LCD_CMD_DISPON);
    // Timer::after(Duration::from_millis(120)).await;
    
    for (cmd, delay, data) in LCD_INIT_CMD {
        tx_command_data(qspi, *cmd, &data);
        Timer::after(Duration::from_millis(*delay as u64)).await;
    }
    tx_command_data(qspi, lcd_command::COLMOD, &[0x77]);
    
    display_on(qspi);

    // Invert
    // tx_command_data_new(spi, LCD_CMD_INVON, &[]);
    
    let line_data =
        Box::<[u8]>::try_new_uninit_slice(DISPLAY_WIDTH as usize * COLOR_BYTES).unwrap();
    // We must write to all of line_data before reading
    let mut line_data = unsafe { line_data.assume_init() };

    let mut rand: u8 = 0;

    // ranges dont include the end value so this runs for 0 - 411
    for line in 0..(DISPLAY_HEIGHT) as u16 {
        // line_data = Vec::with_capacity(412 * 3);
        for x in 0..(DISPLAY_WIDTH) as usize {
            // for color_byte in 0..3 {
            //     line_data[x * 3 + color_byte] = rand;
            //     rand = rand.wrapping_add(1);
            // }
            // line_data[x * 3 + 0] = 0x00;
            // line_data[x * 3 + 1] = 90;
            // line_data[x * 3 + 2] = 60;
            line_data[x * 3 + 0] = 255 - (x / 2) as u8;
            line_data[x * 3 + 1] = 255 - (line / 2) as u8;
            line_data[x * 3 + 2] = 0xff;
        }

        // println!("write line {} with len {}", line, line_data.len());
        draw_bitmap(qspi, 0, line as u16, DISPLAY_WIDTH as u16 - 1, line + 1 as u16, &line_data);
    }
}

pub fn display_on(qspi: &mut SpiDmaBus<'_, Blocking>) {
    tx_command_data(qspi, lcd_command::DISPON, &[]);
}

pub fn draw_bitmap(
    qspi: &mut SpiDmaBus<'_, Blocking>,
    x1: u16,
    y1: u16,
    x2: u16,
    y2: u16,
    color_data: &[u8],
) {
    // [ x1 (byte 2), x1 (byte 1), x2 (byte 2), x2 (byte 1) ]
    // 2 before 1 because Endian and stuff
    let x_set_data: [u8; 4] = [
        (x1 >> 8) as u8,
        (x1 & 0xFF) as u8,
        (x2 >> 8) as u8,
        (x2 & 0xFF) as u8,
    ];
    // println!("x pos data: {:?}", x_set_data);
    tx_command_data(qspi, lcd_command::CASET, &x_set_data);

    let y_set_data: [u8; 4] = [
        (y1 >> 8) as u8,
        (y1 & 0xFF) as u8,
        (y2 >> 8) as u8,
        (y2 & 0xFF) as u8,
    ];
    // println!("y pos data: {:?}", y_set_data);
    tx_command_data(qspi, lcd_command::RASET, &y_set_data);

    // Transfer frame buffer

    // let len = (x2 - x1) * (y2 - y1) * 3; // 3 bytes per pixel
    
    let mut is_first = true;
    for chunk in color_data.chunks(1236) {
        if is_first {
            tx_color(qspi, lcd_command::RAMWR, chunk);
                is_first = false;
        } else {
            tx_color(qspi, lcd_command::RAMWRC, chunk);
        }
    }
}