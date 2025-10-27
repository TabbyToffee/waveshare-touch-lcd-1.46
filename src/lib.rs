#![no_std]
#![feature(allocator_api, new_zeroed_alloc, type_alias_impl_trait)]

extern crate alloc;

pub mod display;
pub mod exio;
pub mod gyroscope;
pub mod interface;
pub mod power_btn;
pub mod speaker;
pub mod touch;
