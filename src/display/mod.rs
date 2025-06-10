mod init_cmd;

use embassy_time::{Duration, Timer};
use init_cmd::LCD_INIT_CMD;

extern crate alloc;
use alloc::vec::Vec;

// use heapless::Vec;
use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::Rgb888,
    prelude::{OriginDimensions, Size},
    Pixel,
};
use embedded_hal::spi::SpiBus;
use esp_hal::{
    gpio::{GpioPin, Io},
    i2c::master::I2c,
    ledc::{
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
        LSGlobalClkSource, Ledc, LowSpeed,
    },
    peripherals::LEDC,
    spi::master::Spi,
    time::Rate,
    Async, Blocking,
};
use esp_println::{dbg, println};

use crate::exio::{self, PinDirection, PinState};

const COLOR_DEPTH: usize = 24; // 24-bit colour
const DISPLAY_WIDTH: u32 = 412;
const DISPLAY_HEIGHT: u32 = 412;
const BUFFER_SIZE: u32 = DISPLAY_WIDTH * DISPLAY_HEIGHT * 3; // 8-bits per colour

// The format is [ OPCODE, 0, CMD, 0, 0, PARAMS ]
const LCD_OPCODE_WRITE_CMD: u8 = 0x02;
const LCD_OPCODE_READ_CMD: u8 = 0x0B;
const LCD_OPCODE_WRITE_COLOR: u8 = 0x32;
const PARAMS_MAX_LEN: u8 = 4;

const SPD2010_CMD_SET: u8 = 0xFF;
const SPD2010_CMD_SET_BYTE0: u8 = 0x20;
const SPD2010_CMD_SET_BYTE1: u8 = 0x10;
const SPD2010_CMD_SET_USER: u8 = 0x00;

const EXIO_LCD_RESET_PIN: u8 = 1;

// Guessed values
// const LCD_CMD_MADCTL: u8 = 0x36;
// const LCD_CMD_COLMOD: u8 = 0x3A; // 0x2A;

const LCD_CMD_NOP: u8 = 0x00; // This command is empty command
const LCD_CMD_SWRESET: u8 = 0x01; // Software reset registers (the built-in frame buffer is not affected)
const LCD_CMD_RDDID: u8 = 0x04; // Read 24-bit display ID
const LCD_CMD_RDDST: u8 = 0x09; // Read display status
const LCD_CMD_RDDPM: u8 = 0x0A; // Read display power mode
const LCD_CMD_RDD_MADCTL: u8 = 0x0B; // Read display MADCTL
const LCD_CMD_RDD_COLMOD: u8 = 0x0C; // Read display pixel format
const LCD_CMD_RDDIM: u8 = 0x0D; // Read display image mode
const LCD_CMD_RDDSM: u8 = 0x0E; // Read display signal mode
const LCD_CMD_RDDSR: u8 = 0x0F; // Read display self-diagnostic result
const LCD_CMD_SLPIN: u8 = 0x10; // Go into sleep mode (DC/DC, oscillator, scanning stopped, but keeps content)
const LCD_CMD_SLPOUT: u8 = 0x11; // Exit sleep mode
const LCD_CMD_PTLON: u8 = 0x12; // Turns on partial display mode
const LCD_CMD_NORON: u8 = 0x13; // Turns on normal display mode
const LCD_CMD_INVOFF: u8 = 0x20; // Recover from display inversion mode
const LCD_CMD_INVON: u8 = 0x21; // Go into display inversion mode
const LCD_CMD_GAMSET: u8 = 0x26; // Select Gamma curve for current display
const LCD_CMD_DISPOFF: u8 = 0x28; // Display off (disable frame buffer output)
const LCD_CMD_DISPON: u8 = 0x29; // Display on (enable frame buffer output)
const LCD_CMD_CASET: u8 = 0x2A; // Set column address
const LCD_CMD_RASET: u8 = 0x2B; // Set row address
const LCD_CMD_RAMWR: u8 = 0x2C; // Write frame memory
const LCD_CMD_RAMRD: u8 = 0x2E; // Read frame memory
const LCD_CMD_PTLAR: u8 = 0x30; // Define the partial area
const LCD_CMD_VSCRDEF: u8 = 0x33; // Vertical scrolling definition
const LCD_CMD_TEOFF: u8 = 0x34; // Turns off tearing effect
const LCD_CMD_TEON: u8 = 0x35; // Turns on tearing effect
const LCD_CMD_MADCTL: u8 = 0x36; // Memory data access control
const LCD_CMD_VSCSAD: u8 = 0x37; // Vertical scroll start address
const LCD_CMD_IDMOFF: u8 = 0x38; // Recover from IDLE mode
const LCD_CMD_IDMON: u8 = 0x39; // Fall into IDLE mode (8 color depth is displayed)
const LCD_CMD_COLMOD: u8 = 0x3A; // Defines the format of RGB picture data
const LCD_CMD_RAMWRC: u8 = 0x3C; // Memory write continue
const LCD_CMD_RAMRDC: u8 = 0x3E; // Memory read continue
const LCD_CMD_STE: u8 = 0x44; // Set tear scan line, tearing effect output signal when display reaches line N
const LCD_CMD_GDCAN: u8 = 0x45; // Get scan line
const LCD_CMD_WRDISBV: u8 = 0x51; // Write display brightness
const LCD_CMD_RDDISBV: u8 = 0x52; // Read display brightness value

pub struct Spd2010<'a> {
    spi: Spi<'a, Async>,
    // spi: SPI,
    buffer: [u8; BUFFER_SIZE as usize],
}

impl<'a> Spd2010<'a> {
    pub async fn new(spi: Spi<'a, Async>) -> Self {
        // println!("0 !!!!");

        // let buffer = [0; BUFFER_SIZE as usize];

        // println!("1 !!!!");

        // let myspi: Spi<Async> = spi;
        // return;

        let mut display = Self {
            spi: spi,
            buffer: [0; BUFFER_SIZE as usize],
        };

        // println!("Create display");

        display
    }

    // pub fn init(&mut self) {
    //     for (cmd, delay, data) in LCD_INIT_CMD {
    //         let mut cmd32 = *cmd as u32;
    //         cmd32 &= 0xff;
    //         cmd32 <<= 8;
    //         cmd32 |= LCD_OPCODE_WRITE_CMD << 24;
    //         let mut full_data: Vec<u8, 10> = Vec::new();
    //         full_data.push(*cmd);
    //         full_data.extend_from_slice(*data);
    //         self.spi.write(&full_data);
    //         self.spi.flush();
    //     }
    // }

    pub fn flush(&self) -> Result<(), core::convert::Infallible> {
        // self.iface.send_bytes(&self.framebuffer)
        Ok(())
    }

    pub fn draw(&mut self) {
        let data = [128; 1000];
        self.spi.write(&data);
        self.spi.flush();
    }
}

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

pub fn backlight_init(ledc: &mut Ledc, backlight_pwm_pin: GpioPin<5>) {
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

pub fn init(spi: &mut Spi<Blocking>) {
    // Will be changed in loop
    let mut is_user_set = true;
    // True if current command has been overwritten by special case
    let mut cmd_overwritten = false;

    for (cmd, delay, data) in LCD_INIT_CMD {
        if is_user_set && data.len() > 0 {
            match *cmd {
                LCD_CMD_MADCTL => {
                    cmd_overwritten = true;
                }
                LCD_CMD_COLMOD => {
                    cmd_overwritten = true;
                }
                _ => {
                    cmd_overwritten = false;
                }
            }

            if cmd_overwritten {
                cmd_overwritten = false;
            }
        }
        // let mut cmd32 = *cmd as u32;
        // cmd32 &= 0xff;
        // cmd32 <<= 8;
        // cmd32 |= LCD_OPCODE_WRITE_CMD << 24;
        // let mut full_data: Vec<u8, 10> = Vec::new();
        // full_data.push(*cmd);
        // full_data.extend_from_slice(*data);
        // let result = spi.write(&full_data);
        // // esp_println::dbg!(result);
        // spi.flush();

        // let opcode = LCD_OPCODE_WRITE_CMD;
        // let command = *cmd; // Read Display Mode
        // let data_p1: [u8; 4 + PARAMS_MAX_LEN as usize] = [opcode, 0x00, command, 0x00];
        // let mut full_data: &[u8] = [0; data_p1.len() + data.len()];

        // let result = spi.transfer(&mut data);

        // dbg!(result);
        // println!("recieved:");
        // for byte in data {
        //     println!("  {:#04x}", byte)
        // }
    }
}

const init_sequence: [u8; 5] = [
    0x3A, 0x55, // Set color mode (16-bit/pixel)
    0x36, 0x00, // Set MADCTL (orientation)
    0x29, // Display on
];

pub async fn test(spi: &mut Spi<'_, Blocking>) {
    let opcode = 0x0B; // Read
    let command_reset = 0x01; // Read Display Mode
    let command = 0x09; // Read Display Mode

    let mut data_reset: [u8; 5] = [opcode, 0x00, command_reset, 0x00, 0x00];

    let result = spi.transfer(&mut data_reset);

    dbg!(result);
    println!("recieved:");
    for byte in data_reset {
        println!("  {:#04x}", byte)
    }

    Timer::after(Duration::from_millis(100)).await;

    let mut data: [u8; 9] = [opcode, 0x00, command, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    let result = spi.transfer(&mut data);

    dbg!(result);
    println!("recieved:");
    for byte in data {
        println!("  {:#04x}", byte)
    }

    // spi.write(&[0x3A]);

    // let r1 = spi.write(&[0x09]);
    // let mut buf: [u8; 4] = [0; 4];
    // let r2 = spi.read(&mut buf);
    // dbg!(r1, r2, buf);
}

pub fn init2(spi: &mut Spi<Blocking>) {
    for (cmd, delay, data) in LCD_INIT_CMD {
        let opcode = 0x0B; // Read
        let command = 0x09; // Read Display Mode
        let mut data: [u8; 9] = [opcode, 0x00, command, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

        let result = spi.transfer(&mut data);

        dbg!(result);
        println!("recieved:");
        for byte in data {
            println!("  {:#04x}", byte)
        }
    }
}

pub async fn reset(i2c: &mut I2c<'_, Blocking>) {
    println!("Reset display");
    exio::set_pin_direction(i2c, EXIO_LCD_RESET_PIN, PinDirection::Output);
    exio::set_pin(i2c, EXIO_LCD_RESET_PIN, PinState::High);
    Timer::after(Duration::from_millis(100)).await;
    exio::set_pin(i2c, EXIO_LCD_RESET_PIN, PinState::Low);
    Timer::after(Duration::from_millis(100)).await;
}

pub fn tx_command(spi: &mut Spi<Blocking>, command: u8) {
    let opcode = LCD_OPCODE_WRITE_CMD;
    let mut data: [u8; 4] = [opcode, 0x00, command, 0x00];

    let result = spi.transfer(&mut data);
    println!("out: {result:?}");
}

pub fn tx_command_data(spi: &mut Spi<Blocking>, command: u8, data: &[u8]) {
    let opcode = LCD_OPCODE_WRITE_CMD;
    let start: [u8; 4] = [opcode, 0x00, command, 0x00];

    // let full_data_iter = start.iter().chain(data.iter());
    // let full_data: Vec<u8> = Vec::from_iter(full_data_iter);

    let mut buffer = Vec::with_capacity(4 + data.len());
    buffer.extend_from_slice(&start);
    buffer.extend_from_slice(data);

    println!("in:  {buffer:?}");
    let result = spi.transfer(&mut buffer);

    println!("out: {result:?}");
}

pub fn tx_color(spi: &mut Spi<Blocking>, command: u8, data: &[u8]) {
    let opcode = LCD_OPCODE_WRITE_COLOR;
    let start: [u8; 4] = [opcode, 0x00, command, 0x00];

    // let full_data_iter = start.iter().chain(data.iter());
    // let full_data: Vec<u8> = Vec::from_iter(full_data_iter);

    let mut buffer = Vec::with_capacity(4 + data.len());
    buffer.extend_from_slice(&start);
    buffer.extend_from_slice(data);

    println!("in:  {buffer:?}");
    let result = spi.transfer(&mut buffer);

    println!("out: {result:?}");
}

pub async fn test_2(spi: &mut Spi<'_, Blocking>) {
    tx_command_data(
        spi,
        SPD2010_CMD_SET,
        &[
            SPD2010_CMD_SET_BYTE0,
            SPD2010_CMD_SET_BYTE1,
            SPD2010_CMD_SET_USER,
        ],
    );
    tx_command_data(spi, LCD_CMD_MADCTL, &[0xA0]);
    tx_command_data(spi, LCD_CMD_COLMOD, &[0x01]);

    for (cmd, delay, data) in LCD_INIT_CMD {
        tx_command_data(spi, *cmd, data);
        Timer::after(Duration::from_millis(*delay as u64)).await;
    }
    
    let color_data: [u8; 3*3*3] = [
        0xFF, 0x66, 0x22,
        0xFF, 0x66, 0x22,
        0xFF, 0x66, 0x22,
        0xFF, 0x66, 0x22,
        0xFF, 0x66, 0x22,
        0xFF, 0x66, 0x22,
        0xFF, 0x66, 0x22,
        0xFF, 0x66, 0x22,
        0xFF, 0x66, 0x22,
    ];
    
    display_on(spi);
    
    tx_command(spi, LCD_CMD_INVON);
    
    println!("\nREADING\n");
    
    let opcode = LCD_OPCODE_READ_CMD;
    let mut data: [u8; 9] = [opcode, 0x00, LCD_CMD_RDDST, 0x00, 0x00, 0, 0, 0, 0];

    let result = spi.transfer(&mut data);
    println!("out: {result:?}");
    
    // tx_command_data(spi, LCD_CMD_RDD_MADCTL, &[0,0,0]);
    
    draw_bitmap(spi, 204, 204, 207, 207, &color_data);
}

pub fn display_on(spi: &mut Spi<'_, Blocking>) {
    tx_command_data(spi, LCD_CMD_DISPON, &[]);
}

pub fn draw_bitmap(spi: &mut Spi<'_, Blocking>, x1: u16, y1: u16, x2: u16, y2: u16, color_data: &[u8]) {
    // [ x1 (byte 2), x1 (byte 1), x2 (byte 2), x2 (byte 1) ]
    // 2 before 1 because Endian and stuff
    let x_set_data: [u8; 4] = [
        ((x1 >> 8) & 0xFF) as u8,
        (x1 & 0xFF) as u8,
        ((x2 >> 8) & 0xFF) as u8,
        (x2 & 0xFF) as u8,
    ];
    tx_command_data(spi, LCD_CMD_CASET, &x_set_data);
    
    let y_set_data: [u8; 4] = [
        ((y1 >> 8) & 0xFF) as u8,
        (y1 & 0xFF) as u8,
        ((y2 >> 8) & 0xFF) as u8,
        (y2 & 0xFF) as u8,
    ];
    tx_command_data(spi, LCD_CMD_RASET, &y_set_data);
    
    // Transfer frame buffer
    
    // let len = (x2 - x1) * (y2 - y1) * 3; // 3 bytes per pixel
    tx_color(spi, LCD_CMD_RAMWR, color_data)
}
