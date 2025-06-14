#![no_std]
#![no_main]
#![allow(dead_code)]

use alloc::vec::Vec;
use embedded_hal::spi::SpiBus;
use esp_hal::dma_buffers;
use esp_hal::i2s::master::{DataFormat, I2s, Standard};
use watch_playground::exio::{PinDirection, PinState};
use watch_playground::interface::i2s;
use watch_playground::{display, exio, speaker, touch};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock};
// use fugit;

// use embedded_graphics::{
//     pixelcolor::Rgb888,
//     prelude::*,
//     primitives::{Circle, PrimitiveStyle},
// };

use esp_hal::{
    i2c::master::{Config, I2c},
    ledc::{
        channel, channel::ChannelIFace, timer, timer::TimerIFace, LSGlobalClkSource, Ledc, LowSpeed,
    },
    peripherals::LEDC,
    spi,
    spi::master::Spi,
    time::Rate,
    Async,
};
use esp_println::{dbg, println};
use pcf8563::{self, DateTime, Time, PCF8563};
// use pcf85063a::{self, Control, DateTime};

const ESP_PANEL_LCD_SPI_IO_TE: u8 = 18;
const ESP_PANEL_LCD_SPI_IO_SCK: u8 = 40;
const ESP_PANEL_LCD_SPI_IO_DATA0: u8 = 46;
const ESP_PANEL_LCD_SPI_IO_DATA1: u8 = 45;
const ESP_PANEL_LCD_SPI_IO_DATA2: u8 = 42;
const ESP_PANEL_LCD_SPI_IO_DATA3: u8 = 41;
const ESP_PANEL_LCD_SPI_IO_CS: u8 = 21;
const EXAMPLE_LCD_PIN_NUM_RST: i8 = -1; // EXIO2
const EXAMPLE_LCD_PIN_NUM_BK_LIGHT: u8 = 5;

const ESP_PANEL_LCD_SPI_CLK_MHZ: u32 = 80; // 80Mhz

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("Paniced: {}", info);
    loop {}
}

extern crate alloc;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.3.1

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let _init = esp_wifi::init(
        timer1.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();
    
    let pwr_btn_pin = peripherals.GPIO6;
    let ledc_pin = peripherals.LEDC;
    let backlight_pwm_pin = peripherals.GPIO5;
    
    let i2s_peripheral = peripherals.I2S0;
    let i2s_dma_channel = peripherals.DMA_CH0;
    // Bit clock (SCLK / BCK)
    let i2s_bclk = peripherals.GPIO48;
    // Audio Data
    let i2s_din = peripherals.GPIO47;
    // Word Select (Left / Right channel) (LCLK / LRCK)
    let i2s_ws = peripherals.GPIO38;
    
    // speaker::test(i2s_peripheral, i2s_dma_channel, i2s_bclk, i2s_din, i2s_ws);
    
    // I2C
    let i2c_sda_pin = peripherals.GPIO11;
    let i2c_scl_pin = peripherals.GPIO10;

    let frequency = Rate::from_khz(400);
    let mut i2c = I2c::new(
        peripherals.I2C0,
        Config::default().with_frequency(frequency),
    )
    .unwrap()
    .with_sda(i2c_sda_pin)
    .with_scl(i2c_scl_pin);
    
    // SPI
    let sck = peripherals.GPIO40;
    let cs = peripherals.GPIO21;
    let sio0 = peripherals.GPIO46;
    let sio1 = peripherals.GPIO45;
    let sio2 = peripherals.GPIO42;
    let sio3 = peripherals.GPIO41;

    let frequency = Rate::from_mhz(ESP_PANEL_LCD_SPI_CLK_MHZ);

    let mut spi = Spi::new(
        peripherals.SPI2,
        spi::master::Config::default()
            .with_mode(spi::Mode::_3)
            .with_frequency(frequency),
    )
    .unwrap()
    .with_sck(sck)
    .with_cs(cs)
    .with_sio0(sio0)
    .with_sio1(sio1)
    .with_sio2(sio2)
    .with_sio3(sio3);
    // .with_dma(peripherals.DMA_CH0);
    // .with_buffers(dma_rx_buf, dma_tx_buf);

    display::reset(&mut i2c).await;
    display::test_2(&mut spi).await;
    
    println!("Reset touch");
    touch::reset(&mut i2c).await;
    // touch::read_fw_version(&mut i2c).await;

    let mut rtc = PCF8563::new(&mut i2c);
    rtc.rtc_init().unwrap();

    let mut ledc: Ledc = Ledc::new(ledc_pin);

    display::backlight_init(&mut ledc, backlight_pwm_pin);
}