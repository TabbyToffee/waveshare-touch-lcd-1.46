pub mod config;
pub mod draw;
mod init_cmd;

use config::EXIO_LCD_RESET_PIN;
use embassy_time::{Duration, Timer};

extern crate alloc;

use embedded_hal::digital::OutputPin;
use esp_hal::{
    Async,
    i2c::master::I2c,
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
    },
    peripherals::GPIO5,
    time::Rate,
};
use esp_println::println;

use crate::display::config::{COLOR_BYTES, lcd_command, opcode};

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
            duty_pct: 20,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .unwrap();
}

pub async fn reset<R: OutputPin>(reset_pin: &mut R) {
    println!("Reset display");

    reset_pin.set_low().unwrap();
    Timer::after(Duration::from_millis(100)).await;
    reset_pin.set_high().unwrap();
    Timer::after(Duration::from_millis(100)).await;
}
