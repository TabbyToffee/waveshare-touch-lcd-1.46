use esp_hal::gpio::{InputConfig, Pull};
use esp_hal::peripherals::GPIO6;
use esp_hal::{gpio::Input};
use esp_println::println;


use embassy_time::{Duration, Timer};


async fn loop_btn_test(pwr_btn_pin: GPIO6<'_>) {
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
}