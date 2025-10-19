mod complex;
mod io;

use core::cell::RefCell;

use critical_section::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::{
    Async, Blocking, DriverMode,
    gpio::{Event, Input},
    i2c::{self, master::I2c},
    peripherals::GPIO4,
};
use esp_println::{dbg, println};
use heapless::Vec;

use crate::exio::{self, PinDirection, PinState};

const SPD2010_ADDR: u8 = 0x53;
const EXIO_TOUCH_RESET_PIN: u8 = 0;
const SPD2010_MAX_TOUCH_POINTS: usize = 10;

#[derive(Debug)]
pub enum Error {
    I2C(i2c::master::Error),
    InterruptStayedHigh,
}

// All touches and gesture info
#[derive(Default, Debug)]
pub struct TouchData {
    pub points: Vec<TouchPoint, SPD2010_MAX_TOUCH_POINTS>,
    pub touch_count: u8,
    pub gesture: u8,
    pub down: bool,
    pub up: bool,
    pub down_x: u16,
    pub down_y: u16,
    pub up_x: u16,
    pub up_y: u16,
}

// Single touch
#[derive(Default, Debug)]
pub struct TouchPoint {
    pub id: u8,
    pub x: u16,
    pub y: u16,
    pub weight: u8,
}

#[derive(Debug)]
struct StatusLow {
    pt_exist: bool,
    gesture: bool,
    key: bool,
    aux: bool,
    keep: bool,
    raw_or_pt: bool,
    none6: bool,
    none7: bool,
}

#[derive(Debug)]
struct StatusHigh {
    none0: bool,
    none1: bool,
    none2: bool,
    cpu_run: bool,
    tint_low: bool,
    tic_in_cpu: bool,
    tic_in_bios: bool,
    tic_busy: bool,
}

#[derive(Debug)]
struct TouchStatus {
    status_low: StatusLow,
    status_high: StatusHigh,
    read_len: u16,
}

struct HDPStatus {
    status: u8,
    next_packet_len: u16,
}

pub struct SPD2010Touch<'a, Dm: DriverMode> {
    touch_interrupt: Input<'a>,
    i2c: I2c<'a, Dm>,
}

static TOUCH_INTERRUPT: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

impl<'a, Dm: DriverMode> SPD2010Touch<'a, Dm> {
    pub fn new(i2c: I2c<'a, Dm>, mut touch_interrupt: Input<'a>) -> Self {
        touch_interrupt.listen(Event::FallingEdge);

        Self {
            i2c,
            touch_interrupt,
        }
    }

    async fn clear_interrupt(&mut self) -> Result<(), Error> {
        let ack: [u8; 2] = [0x01, 0x00]; // step 1: ACK (acknowledge interrupt)
        let rearm: [u8; 2] = [0x00, 0x00]; // step 2: re-arm (setup interrupt again)

        let mut try_count = 0;
        // keep re-trying every 2ms until interrupt is low or tried 5 times
        while self.touch_interrupt.is_low() || try_count == 0 {
            self.write_command(0x0002, &ack)?; // ack
            Timer::after(Duration::from_micros(200)).await;
            self.write_command(0x0002, &rearm)?; // re-arm
            if try_count > 4 {
                // Timeout
                return Err(Error::InterruptStayedHigh);
            }
            try_count += 1;
            Timer::after(Duration::from_millis(2)).await;
        }

        Ok(())
    }

    pub async fn reset(i2c: &mut I2c<'_, Async>) {
        exio::set_pin_direction(i2c, EXIO_TOUCH_RESET_PIN, PinDirection::Output);
        Timer::after(Duration::from_millis(50)).await;
        exio::set_pin(i2c, EXIO_TOUCH_RESET_PIN, PinState::High);
        Timer::after(Duration::from_millis(50)).await;
        exio::set_pin(i2c, EXIO_TOUCH_RESET_PIN, PinState::Low);
        Timer::after(Duration::from_millis(50)).await;
        exio::set_pin(i2c, EXIO_TOUCH_RESET_PIN, PinState::High);
    }
}
