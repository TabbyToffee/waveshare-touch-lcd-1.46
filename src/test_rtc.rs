use embassy_time::{Duration, Timer};
use esp_hal::{
    i2c::master::I2c,
    rtc_cntl::Rtc, Blocking,
};
use esp_println::println;
use pcf8563::{DateTime, Time, PCF8563};

async fn test_rtc(rtc: &mut PCF8563<I2c<'_, Blocking>>) {
    rtc.set_time(&Time {
        hours: 0,
        minutes: 0,
        seconds: 0,
    });

    loop {
        let now = rtc.get_datetime().unwrap();
        println!(
            "Today is {}, {} {} 20{:02} {:02}:{:02}:{:02}\r",
            weekday_name(now),
            now.day,
            month_name(now),
            now.year,
            now.hours,
            now.minutes,
            now.seconds
        );
        Timer::after(Duration::from_millis(100)).await;
    }
}

// Helper function to get the correct name of the day
fn weekday_name(datetime: DateTime) -> &'static str {
    let mut name = "Sunday";
    let day = datetime.weekday;
    match day {
        0 => name = "Sunday",
        1 => name = "Monday",
        2 => name = "Tuesday",
        3 => name = "Wednesday",
        4 => name = "Thursday",
        5 => name = "Friday",
        6 => name = "Saturday",
        _ => (),
    }
    name
}

// Helper function to get the correct name of the month
fn month_name(datetime: DateTime) -> &'static str {
    let mut name = "January";
    let month = datetime.month;
    match month {
        1 => name = "January",
        2 => name = "February",
        3 => name = "March",
        4 => name = "April",
        5 => name = "May",
        6 => name = "June",
        7 => name = "July",
        8 => name = "August",
        9 => name = "September",
        10 => name = "October",
        11 => name = "November",
        12 => name = "December",
        _ => (),
    }
    name
}
