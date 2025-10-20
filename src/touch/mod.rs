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
    xtensa_lx_rt::interrupt,
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

#[derive(Default)]
struct HDPStatus {
    status: u8,
    next_packet_len: u16,
}

pub struct SPD2010Touch<'a, Dm: DriverMode, Ti: InterruptInput> {
    i2c: I2c<'a, Dm>,
    touch_interrupt: &'a Mutex<RefCell<Option<Ti>>>,
}

impl<'a, Dm: DriverMode, Ti: InterruptInput> SPD2010Touch<'a, Dm, Ti> {
    pub fn new(i2c: I2c<'a, Dm>, touch_interrupt: &'a Mutex<RefCell<Option<Ti>>>) -> Self {
        // touch_interrupt.listen(Event::FallingEdge);

        Self {
            i2c,
            touch_interrupt,
        }
    }

    fn clear_interrupt_flag(&self) {
        critical_section::with(|cs| {
            self.touch_interrupt
                .borrow_ref_mut(cs)
                .as_mut()
                .unwrap()
                .clear_interrupt();
        });
    }

    fn get_interrupt_flag(&self) -> bool {
        critical_section::with(|cs| {
            self.touch_interrupt
                .borrow_ref(cs)
                .as_ref()
                .unwrap()
                .get_interrupt()
        })
    }
}

pub trait InterruptInput {
    fn get_interrupt(&self) -> bool;
    fn clear_interrupt(&mut self);
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
