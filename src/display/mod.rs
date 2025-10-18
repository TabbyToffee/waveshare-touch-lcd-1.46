pub mod config;
pub mod draw;
mod init_cmd;

use config::EXIO_LCD_RESET_PIN;
use embassy_time::{Duration, Timer};
use init_cmd::LCD_INIT_CMD;

extern crate alloc;
use alloc::boxed::Box;

use esp_hal::{
    Blocking,
    i2c::master::I2c,
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
    },
    peripherals::GPIO5,
    spi::master::{Address, Command, DataMode, SpiDmaBus},
    time::Rate,
};
use esp_println::{dbg, println};

use crate::{
    display::config::{COLOR_BYTES, DISPLAY_HEIGHT, DISPLAY_WIDTH, lcd_command, opcode},
    exio::{self, PinDirection, PinState},
};

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
        draw_bitmap(
            qspi,
            0,
            line as u16,
            DISPLAY_WIDTH as u16 - 1,
            line + 1 as u16,
            &line_data,
        );
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
