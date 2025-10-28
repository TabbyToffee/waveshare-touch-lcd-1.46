#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(allocator_api, new_zeroed_alloc)]

use core::cell::RefCell;

use critical_section::Mutex;
use display::config::DMA_CHUNK_SIZE;

use embassy_time::{Delay, Duration, Ticker, Timer};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Point, RgbColor},
    primitives::{Circle, PrimitiveStyleBuilder, StyledDrawable},
};
use esp_alloc::HeapStats;
use esp_hal::{
    clock::CpuClock,
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{Event, Input, InputConfig, Io, OutputPin, Pull},
    handler,
    i2c::master::{Config, I2c},
    ledc::Ledc,
    spi::{self, master::Spi},
    time::Rate,
    timer::systimer::SystemTimer,
};
use lib::display::{self, config::ESP_PANEL_LCD_SPI_CLK_MHZ, draw::Spd2010};
use spd2010::touch::{self, InterruptInput, SPD2010Touch, TouchData};
use waveshare_touch_lcd_1_46 as lib;

use embassy_executor::Spawner;
use esp_backtrace::arch::backtrace;

use esp_println::println;

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

struct TouchInterrupt<'a> {
    interrupt_input: Input<'a>,
    interrupt_flag: bool,
}

impl<'a> TouchInterrupt<'a> {
    fn new(interrupt_input: Input<'a>) -> Self {
        Self {
            interrupt_input,
            interrupt_flag: false,
        }
    }
    fn possible_interrupt(&mut self) {
        if self.interrupt_input.is_interrupt_set() {
            println!("Interrupt!");
            self.interrupt_flag = true;
            self.interrupt_input.clear_interrupt();
        }
    }
}

impl<'a> InterruptInput for TouchInterrupt<'a> {
    fn get_interrupt_flag(&self) -> bool {
        self.interrupt_flag
    }
    fn clear_interrupt_flag(&mut self) {
        self.interrupt_flag = false
    }
    fn get_interrupt_state(&self) -> bool {
        self.interrupt_input.is_high()
    }
}

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

// critical_section mutex requires RefCell for interior mutability
static TOUCH_INTERRUPT: Mutex<RefCell<Option<TouchInterrupt>>> = Mutex::new(RefCell::new(None));

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::_80MHz);
    let peripherals = esp_hal::init(config);

    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(handler);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

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

    let mut exio = port_expander::Tca6408a::new(&mut i2c, false);
    let exio_pins = exio.split();
    let mut touch_reset_pin = exio_pins.io0.into_output().unwrap();
    let mut display_reset_pin = exio_pins.io1.into_output().unwrap();

    // let pwr_btn_pin = peripherals.GPIO6;
    let ledc_pin = peripherals.LEDC;
    let backlight_pwm_pin = peripherals.GPIO5;

    // let i2s_peripheral = peripherals.I2S0;
    // let i2s_dma_channel = peripherals.DMA_CH0;
    // // Bit clock (SCLK / BCK)
    // let i2s_bclk = peripherals.GPIO48;
    // // Audio Data
    // let i2s_din = peripherals.GPIO47;
    // // Word Select (Left / Right channel) (LCLK / LRCK)
    // let i2s_ws = peripherals.GPIO38;

    // // speaker::test(i2s_peripheral, i2s_dma_channel, i2s_bclk, i2s_din, i2s_ws);

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

    let spi = Spi::new(
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

    display::reset(&mut display_reset_pin).await;

    let mut ledc: Ledc = Ledc::new(ledc_pin);

    display::backlight_init(&mut ledc, backlight_pwm_pin);

    let mut spd2010 = Spd2010::new(spi, peripherals.GPIO18);

    spd2010.init().await.unwrap();

    let config = InputConfig::default().with_pull(Pull::Up);
    let mut touch_interrupt = Input::new(peripherals.GPIO4, config);
    touch_interrupt.listen(Event::FallingEdge);

    let touch_interrupt_handler = TouchInterrupt {
        interrupt_input: touch_interrupt,
        interrupt_flag: false,
    };

    critical_section::with(|cs| {
        TOUCH_INTERRUPT
            .borrow_ref_mut(cs)
            .replace(touch_interrupt_handler);
    });

    Timer::after(Duration::from_millis(200)).await;
    touch::reset(&mut Delay, &mut touch_reset_pin)
        .await
        .unwrap();
    Timer::after(Duration::from_millis(200)).await;
    let mut touch = SPD2010Touch::new(&mut i2c, &TOUCH_INTERRUPT);
    Timer::after(Duration::from_millis(200)).await;

    println!("{}", touch.read_fw_version().unwrap());

    // let font = FontRenderer::new::<fonts::u8g2_font_logisoso92_tn>();
    // // let text = "Welcome to SteadyTickOS";
    // // let text = "Welcome to SteadyTickOS\n13:06";
    // // let text = "13:06";

    // // let mut rtc = pcf85063a::PCF85063::new(&mut i2c);

    // // test_rtc::test_rtc(&mut rtc).await;

    // let mut ticker = Ticker::every(Duration::from_secs(1));

    // // rtc.set_time(&time::Time::MIDNIGHT);
    // // rtc.set_time(&Time {
    // //     hours: 0,
    // //     minutes: 0,
    // //     seconds: 0,
    // // });

    let white = PrimitiveStyleBuilder::new()
        .fill_color(Rgb888::WHITE)
        .build();

    spd2010.fill();
    spd2010.flush().await.unwrap();

    // loop {
    //     let interrupt = touch_interrupt.is_interrupt_set();
    //     spd2010.fill();
    //     let rect = Rectangle::new(
    //         Point {
    //             x: 204,
    //             y: 152 + interrupt as i32 * 100,
    //         },
    //         Size::new_equal(4),
    //     );
    //     rect.draw_styled(&white, &mut spd2010);
    //     spd2010.flush().await.unwrap();
    //     touch_interrupt.clear_interrupt();
    //     Timer::after(Duration::from_millis(500)).await;
    // }

    loop {
        let predicted_available = touch.available();
        if !predicted_available {
            continue;
        }

        let mut touch_data = TouchData::default();
        let new_data = touch.read(&mut Delay, &mut touch_data).await.unwrap();

        for point in touch_data.points {
            let circle = Circle::new(
                Point {
                    x: point.x as i32,
                    y: point.y as i32,
                },
                5,
            );
            circle.draw_styled(&white, &mut spd2010).unwrap();
        }
        spd2010.flush().await.unwrap();
    }
}

#[handler]
fn handler() {
    critical_section::with(|cs| {
        TOUCH_INTERRUPT
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .possible_interrupt();
    });
}
