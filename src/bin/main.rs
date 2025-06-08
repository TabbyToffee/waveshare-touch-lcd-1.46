#![no_std]
#![no_main]

use esp_hal::gpio::rtc_io::LowPowerOutputOpenDrain;
use esp_hal::gpio::{DriveMode, Flex, InputConfig, Level, Output, OutputConfig, Pull, RtcPin};
use esp_hal::peripherals::Peripherals;
use esp_hal::Blocking;
use watch_playground::exio::{PinDirection, PinState};
use watch_playground::{display, exio};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_hal;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, gpio::Input};
// use fugit;

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
};

use esp_hal::{
    gpio::{GpioPin, Io},
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
    
    exio::write_register(&mut i2c, 1, 55);
    let reg_value = exio::read_register(&mut i2c, 1);
    println!("55? : {}", reg_value);
    
    exio::set_pin(&mut i2c, 4, PinState::High);
    let pin_state = exio::read_pin(&mut i2c, 4);
    println!("High? : {:?}", pin_state);
    
    exio::set_pin_direction(&mut i2c, 4, PinDirection::Output);
    let pin_direction = exio::read_pin_direction(&mut i2c, 4);
    println!("Output?: {:?}", pin_direction);
    
    exio::set_pin_direction(&mut i2c, 4, PinDirection::Input);
    let pin_direction = exio::read_pin_direction(&mut i2c, 4);
    println!("Input?: {:?}", pin_direction);
    
    exio::set_pin(&mut i2c, 2, PinState::High);
    Timer::after(Duration::from_millis(100)).await;
    exio::set_pin(&mut i2c, 2, PinState::Low);
    Timer::after(Duration::from_millis(100)).await;

    let mut rtc = PCF8563::new(i2c);
    rtc.rtc_init().unwrap();

    let mut ledc: Ledc = Ledc::new(ledc_pin);

    display::backlight_init(&mut ledc, backlight_pwm_pin);

    // Reset EXIO
    // i2c.write(TCA9554_CONFIG_REG, &[0x00]);

    // Reset LCD
    // set_pin(&mut i2c, 0x02, false);
    // Timer::after(Duration::from_millis(100)).await;
    // set_pin(&mut i2c, 0x02, true);
    // Timer::after(Duration::from_millis(100)).await;

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
            .with_frequency(frequency),
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
    // return
    // println!("data: {:x?}", data);

    // spi.transfer(&[32]);
    // spi.write_bytes(&[32]);

    println!("Before");

    {
        // display::Spd2010::new(spi).await;
        // let mut xfsdffdv = display::Spd2010::new(spi).await;
        // xfsdffdv.init();
        // xfsdffdv.draw();
    }

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }

    // display::Spd2010::new(spi);

    // let x = display::Spd2010 {

    // }

    // display.set_pixel(10, 20, 0xf00);

    // Draw a circle with top-left at `(22, 22)` with a diameter of `20` and a white stroke
    // let circle = Circle::new(Point::new(22, 22), 20)
    //     .into_styled(PrimitiveStyle::with_stroke(Rgb888::WHITE, 1));

    // circle.draw(&mut display);

    // Update the display
    // display.flush().unwrap();
}

fn backlight_init() {}

// pub fn set_pin(i2c: &mut I2c<Async>, pin: u8, state: bool) {
//     println!("Set Pin");

//     let mut return_bytes: &mut [u8] = &mut [0; 20];
//     let mut data: u8 = 0;
//     let bits_status: u8 = 0;
//     let _ = i2c.read(TCA9554_ADDRESS, &mut return_bytes);

//     for byte in return_bytes {
//         println!("byte: {:#b}", byte);
//     }

//     if pin < 9 && pin > 0 {
//         if state {
//             // (0x01 << (pin-1)) -> Byte with 1 in pin position
//             // (0x01 << (pin-1)) | bits_status -> bits_status with 1 at pin position
//             data = (0x01 << (pin - 1)) | bits_status;
//         } else {
//             data = !(0x01 << (pin - 1)) & bits_status;
//         }
//         println!("Writing to pin: {:#b}", data);
//         let _ = i2c.write(TCA9554_OUTPUT_REG, &[data]);
//     } else {
//         println!("Parameter error, please enter the correct parameter!");
//     }
// }