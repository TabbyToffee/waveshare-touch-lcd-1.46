#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(allocator_api, new_zeroed_alloc)]

use core::cell::RefCell;

use alloc::format;
use critical_section::Mutex;
use display::config::DMA_CHUNK_SIZE;

use embassy_time::{Duration, Ticker, Timer};
use embedded_graphics::{
    Drawable,
    pixelcolor::Rgb888,
    prelude::{Point, Primitive, RgbColor, Size},
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle, StyledDrawable},
};
use esp_alloc::HeapStats;
// use embedded_hal::spi::SpiBus;
use esp_hal::{
    Async,
    dma::{DmaRxBuf, DmaTxBuf},
    gpio::{Event, Input, Io, Pull},
};
use esp_hal::{dma_buffers, psram};
use esp_hal::{gpio::InputConfig, psram::PsramSize};
use pcf8563::{PCF8563, Time};
// use smoltcp::time::Duration;
use watch_playground::{
    display, test_rtc,
    touch::{SPD2010Touch, TouchData},
};
use watch_playground::{
    display::{config::ESP_PANEL_LCD_SPI_CLK_MHZ, draw::Spd2010},
    speaker, touch,
};

use embassy_executor::Spawner;
use esp_backtrace::arch::backtrace;
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;

use esp_hal::{
    i2c::master::{Config, I2c},
    ledc::Ledc,
    spi,
    spi::master::Spi,
    time::Rate,
};
use esp_println::println;
use u8g2_fonts::{
    FontRenderer, fonts,
    types::{FontColor, HorizontalAlignment, VerticalPosition},
};

esp_bootloader_esp_idf::esp_app_desc!();

const ESP_PANEL_LCD_SPI_IO_TE: u8 = 18;
const ESP_PANEL_LCD_SPI_IO_SCK: u8 = 40;
const ESP_PANEL_LCD_SPI_IO_DATA0: u8 = 46;
const ESP_PANEL_LCD_SPI_IO_DATA1: u8 = 45;
const ESP_PANEL_LCD_SPI_IO_DATA2: u8 = 42;
const ESP_PANEL_LCD_SPI_IO_DATA3: u8 = 41;
const ESP_PANEL_LCD_SPI_IO_CS: u8 = 21;
const EXAMPLE_LCD_PIN_NUM_RST: i8 = -1; // EXIO2
const EXAMPLE_LCD_PIN_NUM_BK_LIGHT: u8 = 5;

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

static TOUCH_INTERRUPT: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    // generator version: 0.3.1

    let mut psram_config = psram::PsramConfig::default();
    psram_config.size = PsramSize::Size(4 * 1024 * 1024);

    let config = esp_hal::Config::default()
        .with_cpu_clock(CpuClock::max())
        .with_psram(psram_config);
    let peripherals = esp_hal::init(config);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

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
    let i2c_scl_pin = peripherals.GPIO10;
    let i2c_sda_pin = peripherals.GPIO11;

    let frequency = Rate::from_khz(400);
    let mut i2c = I2c::new(
        peripherals.I2C0,
        Config::default().with_frequency(frequency),
    )
    .unwrap()
    .with_sda(i2c_sda_pin)
    .with_scl(i2c_scl_pin)
    .into_async();

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

    let mut ledc: Ledc = Ledc::new(ledc_pin);

    display::backlight_init(&mut ledc, backlight_pwm_pin);

    let mut spd2010 = Spd2010::new(spi, peripherals.GPIO18);

    spd2010.init().await.unwrap();

    let config = InputConfig::default().with_pull(Pull::Up);
    let mut touch_interrupt = Input::new(peripherals.GPIO4, config);

    critical_section::with(|cs| {
        touch_interrupt.listen(Event::FallingEdge);
        TOUCH_INTERRUPT.borrow_ref_mut(cs).replace(touch_interrupt)
    });

    Timer::after(Duration::from_millis(500)).await;
    SPD2010Touch::<Async>::reset(&mut i2c).await;
    Timer::after(Duration::from_millis(100)).await;

    let mut touch = SPD2010Touch::new(i2c, touch_interrupt);

    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(handler);

    touch.read_fw_version().unwrap();

    let font = FontRenderer::new::<fonts::u8g2_font_logisoso92_tn>();
    // let text = "Welcome to SteadyTickOS";
    // let text = "Welcome to SteadyTickOS\n13:06";
    // let text = "13:06";

    // let mut rtc = pcf85063a::PCF85063::new(&mut i2c);

    // test_rtc::test_rtc(&mut rtc).await;

    let mut ticker = Ticker::every(Duration::from_secs(1));

    // rtc.set_time(&time::Time::MIDNIGHT);
    // rtc.set_time(&Time {
    //     hours: 0,
    //     minutes: 0,
    //     seconds: 0,
    // });

    let white = PrimitiveStyleBuilder::new()
        .fill_color(Rgb888::WHITE)
        .build();

    spd2010.fill();
    spd2010.flush().await.unwrap();

    loop {
        let mut touch_data = TouchData::default();
        touch.read_touch_data(&mut touch_data).await.unwrap();

        if touch_data.points.len() > 0 {
            let point = &touch_data.points[0];
            let circle = Circle::new(
                Point {
                    x: point.x as i32,
                    y: point.y as i32,
                },
                5,
            );
            circle.draw_styled(&white, &mut spd2010).unwrap();
            spd2010.flush().await.unwrap();
        }

        Timer::after(Duration::from_millis(500)).await;
    }

    // loop {
    //     let time = rtc.get_datetime().await.unwrap();
    //     let time_text = format!(
    //         "{:02}:{:02}:{:02}",
    //         time.hour(),
    //         time.minute(),
    //         time.second()
    //     );

    //     spd2010.fill();

    //     font.render_aligned(
    //         time_text.as_ref(),
    //         Point::new(206, 206),
    //         VerticalPosition::Center,
    //         HorizontalAlignment::Center,
    //         FontColor::Transparent(Rgb888::WHITE),
    //         &mut spd2010,
    //     )
    //     .unwrap();

    //     spd2010.flush().await.unwrap();

    //     ticker.next().await;
    // }

    // Timer::after(Duration::from_secs(1)).await;

    // let white = PrimitiveStyleBuilder::new()
    //     .fill_color(Rgb888::WHITE)
    //     .build();

    // let black = PrimitiveStyleBuilder::new()
    //     .fill_color(Rgb888::BLACK)
    //     .build();

    // for _ in 0..10 {
    //     Rectangle::new(Point::new(0, 0), Size::new(412, 412))
    //         .into_styled(white)
    //         .draw(&mut spd2010)
    //         .unwrap();
    //     spd2010.flush().await.unwrap();
    //     Timer::after(Duration::from_millis(1000)).await;
    //     Rectangle::new(Point::new(0, 0), Size::new(412, 412))
    //         .into_styled(black)
    //         .draw(&mut spd2010)
    //         .unwrap();
    //     spd2010.flush().await.unwrap();
    //     Timer::after(Duration::from_millis(1000)).await;
    // }

    // println!("Goodbye");
    // let stats: HeapStats = esp_alloc::HEAP.stats();
    // println!("{}", stats);
}

#[handler]
#[ram]
fn handler() {
