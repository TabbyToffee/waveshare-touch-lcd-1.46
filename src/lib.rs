#![no_std]
#![feature(allocator_api, new_zeroed_alloc)]
#![feature(inherent_str_constructors)]

extern crate alloc;

pub mod display;
pub mod exio;
pub mod gyroscope;
pub mod interface;
pub mod power_btn;
pub mod speaker;
pub mod test_rtc;
pub mod touch;
