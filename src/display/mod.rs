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
    
    pub fn init(&mut self) {
        for (cmd, delay, data) in LCD_INIT_CMD {
            let mut cmd32 = *cmd as u32;
            cmd32 &= 0xff;
            cmd32 <<= 8;
            cmd32 |= LCD_OPCODE_WRITE_CMD << 24;
            let mut full_data: Vec<u8, 10> = Vec::new();
            full_data.push(*cmd);
            full_data.extend_from_slice(*data);
            self.spi.write(&full_data);
            self.spi.flush();
        }
    }
    
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