#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(allocator_api, new_zeroed_alloc)]

use alloc::boxed::Box;
use alloc::vec::Vec;
use display::DMA_CHUNK_SIZE;

use embedded_graphics::Drawable;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{Point, Primitive, WebColors};
use embedded_graphics::primitives::{Circle, PrimitiveStyle};
use esp_alloc::HeapStats;
// use embedded_hal::spi::SpiBus;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio::{Event, Input, InputConfig, Level, Pull};
use esp_hal::psram::PsramSize;
use esp_hal::{dma_buffers, peripherals, psram};
use watch_playground::display::draw::Spd2010;
// use esp_hal::i2s::master::{DataFormat, I2s, Standard};
// use watch_playground::exio::{PinDirection, PinState};
// use watch_playground::interface::i2s;
use watch_playground::{display, exio, speaker, touch};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_backtrace::arch::backtrace;
// use embedded_graphics::{
//     pixelcolor::Rgb888,
//     prelude::*,
//     primitives::{Circle, PrimitiveStyle},
// };

use esp_hal::{
    Async,
    i2c::master::{Config, I2c},
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed, channel, channel::ChannelIFace, timer, timer::TimerIFace,
    },
    peripherals::LEDC,
    spi,
    spi::master::Spi,
    time::Rate,
};
use esp_println::{dbg, print, println};
use pcf8563::{self, DateTime, PCF8563, Time};
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

const ESP_PANEL_LCD_SPI_CLK_MHZ: u32 = 20; // 80Mhz

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    backtrace();
    let backtrace = esp_backtrace::Backtrace::capture();
    let frames = backtrace.frames();
    println!("BACKTRACE:");
    for frame in frames {
        println!("{:#x}", frame.program_counter())
    }
    loop {}
}

extern crate alloc;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.3.1

    let mut psram_config = psram::PsramConfig::default();
    psram_config.size = PsramSize::Size(4 * 1024 * 1024);
    // psram_config.size = PsramSize::Size(1024);

    println!("{:?}", psram_config);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max())
        .with_psram(psram_config);
    let peripherals = esp_hal::init(config);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    // esp_alloc::heap_allocator!(size: 256 * 1024);
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

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

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(DMA_CHUNK_SIZE);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let stats: HeapStats = esp_alloc::HEAP.stats();
    println!("{}", stats);

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
    .with_sio3(sio3)
    .with_dma(peripherals.DMA_CH1)
    .with_buffers(dma_rx_buf, dma_tx_buf);

    display::reset(&mut i2c).await;

    // println!("new 1");
    // let framebuffer = Box::<[u8]>::new_zeroed_slice(412 * 412 * 3);
    // println!("new 2");
    // let mut framebuffer = unsafe { framebuffer.assume_init() };
    // println!("new 3");

    // framebuffer[7] = 54;
    // println!("{}", framebuffer[7]);

    // let stats: HeapStats = esp_alloc::HEAP.stats();
    // println!("{}", stats);

    // println!("DONE");

    // Timer::after(Duration::from_millis(500)).await;

    // println!("ALL DONE");

    // return;

    let mut ledc: Ledc = Ledc::new(ledc_pin);

    display::backlight_init(&mut ledc, backlight_pwm_pin);

    let mut spd2010 = Spd2010::new(spi, peripherals.GPIO18);


    // return;

    let result = spd2010.init().await;

    // let mut i: u8 = 0;
    // for byte in spd2010.framebuffer.iter_mut() {
    //     *byte = i;
    //     i = i.wrapping_add(1);
    // }

    // dbg!(result);
    // dbg!(spd2010.fill_2());


    let mut x: u16 = 0;
    let mut y: u16 = 0;
    
    // let config = InputConfig::default().with_pull(Pull::Up);
    // let mut button = Input::new(pwr_btn_pin, config);

    

    // for frame in 0..200u8 {
    //     // let nx = (x + 64) % 191;
    //     // let ny = ((x as i32 - 64).rem_euclid(191)) as u16;
    //     // let nx = (x as i32 + 8).rem_euclid(411) as u16;
    //     // let ny = (y as i32 + 4).rem_euclid(411) as u16;
    //     // spd2010.draw_rect(x, y, nx, ny, 62, 74, 59);
    //     spd2010.draw_rect(0, 0, 411, 411, 255,  255 - ((frame.wrapping_mul(133)) % 255), (frame.wrapping_mul(26)) % 255);
    //     // while tear_input.is_low() {}
    //     spd2010.flush().await;
    //     Timer::after(Duration::from_millis(50)).await;
    //     // spd2010.draw_rect(x, y, nx, ny, 0, 0, 0);
    //     // x = nx;
    //     // y = ny;
    // }
    
    // for frame in 0..200 {
    //     // let nx = (x + 64) % 191;
    //     // let ny = ((x as i32 - 64).rem_euclid(191)) as u16;
    //     let nx = (x as i32 + 8).rem_euclid(411) as u16;
    //     let ny = (y as i32 + 4).rem_euclid(411) as u16;
    //     // spd2010.draw_rect(x, y, nx, ny, 62, 74, 59);
    //     spd2010.draw_rect(x, y, nx, ny, 235, 160, 221);
    //     // while tear_input.is_low() {}
    //     spd2010.flush().await;
    //     Timer::after(Duration::from_millis(50)).await;
    //     spd2010.draw_rect(x, y, nx, ny, 0, 0, 0);
    //     x = nx;
    //     y = ny;
    // }
    spd2010.draw_rect(0 , 0, 411, 411, 60, 95, 60);
    // spd2010.draw_rect(63, 63, 127, 127, 62, 74, 59);
    // spd2010.draw_rect(73, 73, 117, 117, 235, 160, 221);

    // for i in 0..(127) {
    //     let index = ((64 * 128) + i) * 3;
    //     print!("{} ", spd2010.framebuffer[index]);
    //     print!("{} ", spd2010.framebuffer[index + 1]);
    //     println!("{}", spd2010.framebuffer[index + 2]);
    // }
    // println!("");

    let result = spd2010.flush().await;
    dbg!(result);

    Timer::after(Duration::from_secs(2)).await;
    let circle = Circle::new(Point::new_equal(205), 50)
        .into_styled(PrimitiveStyle::with_stroke(Rgb888::CSS_PURPLE, 1));
    let result = circle.draw(&mut spd2010);
    dbg!(result);
    
    spd2010.flush();
    
    Timer::after(Duration::from_secs(2)).await;

    // let config = InputConfig::default().with_pull(Pull::Up);
    // let mut tear_input = Input::new(peripherals.GPIO18, config);

    // let mut last_level = Level::Low;
    // loop {
    //     let level = tear_input.level();
    //     if level != last_level {
    //         dbg!(level);
    //         last_level = level;
    //     }
    //     if button.is_low() {
    //         break;
    //     }
    // }

    // display::test(&mut spi).await;

    println!("Reset touch");
    touch::reset(&mut i2c).await;
    // touch::read_fw_version(&mut i2c).await;

    // let mut rtc = PCF8563::new(&mut i2c);
    // rtc.rtc_init().unwrap();

}
