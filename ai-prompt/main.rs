#![no_std]
#![no_main]

use watch_playground::display;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
// use fugit;

use embedded_graphics::{
    pixelcolor::{Rgb888},
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
};

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
    spi,
    spi::master::Spi,
    i2c::master::{I2c, Config},
    peripherals::LEDC,
    gpio::{Io, GpioPin},
    time::Rate,
    Async,
};
use esp_println::{dbg, println};

const TCA9554_ADDRESS: u8 = 0x20;
const TCA9554_INPUT_REG: u8 = 0x00;
const TCA9554_OUTPUT_REG: u8 = 0x01;
const TCA9554_POLARITY_REG: u8 = 0x02;
const TCA9554_CONFIG_REG: u8 = 0x03;

const ESP_PANEL_LCD_SPI_IO_TE: u8 = 18;
const ESP_PANEL_LCD_SPI_IO_SCK: u8 = 40;
const ESP_PANEL_LCD_SPI_IO_DATA0: u8 = 46;
const ESP_PANEL_LCD_SPI_IO_DATA1: u8 = 45;
const ESP_PANEL_LCD_SPI_IO_DATA2: u8 = 42;
const ESP_PANEL_LCD_SPI_IO_DATA3: u8 = 41;
const ESP_PANEL_LCD_SPI_IO_CS: u8 = 21;
const EXAMPLE_LCD_PIN_NUM_RST: i8 = -1;    // EXIO2
const EXAMPLE_LCD_PIN_NUM_BK_LIGHT: u8 = 5;

const ESP_PANEL_LCD_SPI_CLK_MHZ: u32 = 80; // 80Mhz

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        println!("Crash")
    }
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

    let ledc_pin = peripherals.LEDC;
    let backlight_pwm_pin = peripherals.GPIO5;
    
    display::backlight_init(ledc_pin, backlight_pwm_pin);

    let mut i2c = I2c::new(
        peripherals.I2C0,
        Config::default(),
    )
    .unwrap()
    .with_sda(peripherals.GPIO10)
    .with_scl(peripherals.GPIO11)
    .into_async();
    
    // Reset EXIO
    i2c.write(TCA9554_CONFIG_REG, &[0x00]);
    
    // Reset LCD
    set_pin(&mut i2c, 0x02, false);
    Timer::after(Duration::from_millis(100)).await;
    set_pin(&mut i2c, 0x02, true);
    Timer::after(Duration::from_millis(100)).await;
    
    let sck = peripherals.GPIO40;
    let cs = peripherals.GPIO21;
    let sio0 = peripherals.GPIO46;
    let sio1 = peripherals.GPIO45;
    let sio2 = peripherals.GPIO42;
    let sio3 = peripherals.GPIO41;

    let frequency = Rate::from_mhz(ESP_PANEL_LCD_SPI_CLK_MHZ);
    
    let mut spi: Spi<Async> = Spi::new(
        peripherals.SPI2,
        spi::master::Config::default()
            .with_mode(spi::Mode::_0)
            .with_frequency(frequency)
    )
    .unwrap()
    .with_sck(sck)
    .with_cs(cs)
    .with_sio0(sio0)
    .with_sio1(sio1)
    .with_sio2(sio2)
    .with_sio3(sio3)
    .into_async();
    
    let mut data = [0x01, 0x02, 0x03, 0x04, 35, 36, 37, 38];
    spi.transfer(&mut data).unwrap();
    
    display::Spd2010::new(spi).await;
}

fn backlight_init() {
    
}

pub fn set_pin(i2c: &mut I2c<Async>, pin: u8, state: bool) {
    println!("Set Pin");
    
    let mut return_bytes: &mut [u8] = &mut [0; 20];
    let mut data: u8 = 0;
    let bits_status: u8 = 0;
    let _ = i2c.read(TCA9554_ADDRESS, &mut return_bytes);

    for byte in return_bytes {
        println!("byte: {:#b}", byte);
    }
    
    if pin < 9 && pin > 0 {
        if state {
            // (0x01 << (pin-1)) -> Byte with 1 in pin position
            // (0x01 << (pin-1)) | bits_status -> bits_status with 1 at pin position
            data = (0x01 << (pin-1)) | bits_status;
        } else {
            data = !(0x01 << (pin-1)) & bits_status;
        }
        println!("Writing to pin: {:#b}", data);
        let _ = i2c.write(TCA9554_OUTPUT_REG, &[data]);
    } else {
        println!("Parameter error, please enter the correct parameter!");
    }
}