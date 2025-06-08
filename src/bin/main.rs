#![no_std]
#![no_main]

use esp_hal::gpio::rtc_io::LowPowerOutputOpenDrain;
use esp_hal::gpio::{DriveMode, Flex, InputConfig, Level, Output, OutputConfig, Pull, RtcPin};
use esp_hal::peripherals::Peripherals;
use esp_hal::Blocking;
use watch_playground::display;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{clock::CpuClock, gpio::Input};
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use embedded_hal;
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
use pcf8563::{self, PCF8563, DateTime};
// use pcf85063a::{self, Control, DateTime};

const TCA9554_ADDRESS: u8 = 0x20;
const TCA9554_INPUT_REG: u8 = 0x00;
const TCA9554_OUTPUT_REG: u8 = 0x01;
const TCA9554_POLARITY_REG: u8 = 0x02;
const TCA9554_CONFIG_REG: u8 = 0x03;

// RTC
const PCF85063_ADDRESS: u8 = 0x51;

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
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {
        
    }
    // loop {
    //     println!("Crash")
    // }
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
    let x = Output::new(i2c_sda_pin, Level::Low, OutputConfig::default().with_pull(Pull::None).with_drive_mode(DriveMode::OpenDrain));
    // let mut x = Flex::new(i2c_sda_pin);
    // x.pull_direction(Pull::Up);
    // let mut y = Flex::new(i2c_scl_pin);
    // y.pull_direction(Pull::Up);
    let y = Output::new(i2c_scl_pin, Level::Low, OutputConfig::default().with_pull(Pull::None).with_drive_mode(DriveMode::OpenDrain));
    
    let frequency = Rate::from_khz(400);
    let mut i2c = I2c::new(
        peripherals.I2C0,
        Config::default().with_frequency(frequency),
    )
    .unwrap()
    .with_sda(x)
    .with_scl(y);
    
    // println!("start");
    
    // for addr in 0..u8::MAX {
    //     let result = i2c.write(addr, &[0, 4]);
    //     if result.is_ok() || true {
    //         println!("{:?}", result);
    //     }
    // }
    // println!("end");
    // loop {}
    // i2c.write(PCF85063_ADDRESS, &[1, 0]);
    
    // let register = 0;
    // let mut data = [0];
    // i2c.write_read(PCF85063_ADDRESS, &[register], &mut data);
    
    // println!("response: {:?}", data);
    
    // .into_async();
    
    let mut rtc = PCF8563::new(i2c);
    rtc.rtc_init().unwrap();
    
    let now = rtc.get_datetime().unwrap();
    println!("Today is {}, {} {} 20{:02} {:02}:{:02}:{:02}\r", 
                    weekday_name(now),
                    now.day, 
                    month_name(now),
                    now.year, 
                    now.hours, 
                    now.minutes, 
                    now.seconds
    );

    let config = InputConfig::default().with_pull(Pull::Up);
    let mut button = Input::new(pwr_btn_pin, config);
    
    let mut was_pressed = false;
    loop {
        let is_pressed = button.is_low();
        if is_pressed && !was_pressed {
            println!("Button pressed!");
        }
        was_pressed = is_pressed;
        Timer::after(Duration::from_millis(100)).await;
    }
    
    let mut ledc: Ledc = Ledc::new(ledc_pin);
    
    display::backlight_init(&mut ledc, backlight_pwm_pin);

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

// Helper function to get the correct name of the day
fn weekday_name(datetime: DateTime) -> &'static str  {
    let mut name = "Sunday";
    let day = datetime.weekday;
    match day {
        0 => {name = "Sunday"}
        1 => {name = "Monday"}
        2 => {name = "Tuesday"}
        3 => {name = "Wednesday"}
        4 => {name = "Thursday"}
        5 => {name = "Friday"}
        6 => {name = "Saturday"}
        _ => ()
    }
    name
}

// Helper function to get the correct name of the month
fn month_name(datetime: DateTime) -> &'static str  {
    let mut name = "January";
    let month = datetime.month;
    match month {
        1 => {name = "January"}
        2 => {name = "February"}
        3 => {name = "March"}
        4 => {name = "April"}
        5 => {name = "May"}
        6 => {name = "June"}
        7 => {name = "July"}
        8 => {name = "August"}
        9 => {name = "September"}
        10 => {name = "October"}
        11 => {name = "November"}
        12 => {name = "December"}        
        _ => ()
    }
    name
}