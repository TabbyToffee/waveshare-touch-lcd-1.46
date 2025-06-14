use esp_hal::{dma::DmaChannel0, dma_buffers, gpio::GpioPin, i2s::master::{DataFormat, I2s, Standard}, peripherals::{self, Peripherals}, time::Rate, Blocking};
use esp_println::{dbg, println};
use libm;

use crate::alloc::vec::Vec;

const mid: u16 = u16::MAX / 2;

pub fn init(i2s: &mut I2s<Blocking>) {
    
}

pub fn sin_wave(time: usize) -> u16 {
    let sined = libm::sin((time as f64) / 32f64);
    ((sined * 32f64) + mid as f64) as u16
}

pub struct Noise {
    pub current: u8,
    pub len: usize,
}

impl Iterator for Noise {
    type Item = u8;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let sined = libm::sin(self.len as f64);
        let amplified = ((sined * 4f64) + 0x7f as f64) as u8;
        Some(amplified)
    }
}

pub fn test(i2s_peripheral: peripherals::I2S0, dma_channel: DmaChannel0, bclk: GpioPin<48>, din: GpioPin<47>, ws: GpioPin<38>) {
    // let i2s_peripheral = peripherals.I2S0;
    // let i2s_dma_channel = peripherals.DMA_CH0;
    // Bit clock (SCLK / BCK)
    // let bclk = peripherals.GPIO48;
    // Audio Data
    // let din = peripherals.GPIO47;
    // Word Select (Left / Right channel) (LCLK / LRCK)
    // let ws = peripherals.GPIO38; 
    // i2s::init(i2s_peripheral, i2s_dma_channel);
    const I2S_BYTES: usize = 4096;
    let (mut rx_buffer, rx_descriptors, _, tx_descriptors) = dma_buffers!(32 * I2S_BYTES, 32 * I2S_BYTES);
    let mut i2s = I2s::new(i2s_peripheral, Standard::Philips, DataFormat::Data16Channel16, Rate::from_hz(22050), dma_channel, rx_descriptors, tx_descriptors);
    // let mut i2s = I2s::new(i2s_peripheral, Standard::Philips, DataFormat::Data16Channel16, Rate::from_hz(100), i2s_dma_channel, rx_descriptors, tx_descriptors);
    
    // let mut i2s_rx = i2s.i2s_rx.with_bclk(bclk).with_ws(ws).with_din(din).build();
    
    // let mut transfer = i2s_rx.read_dma_circular(&mut rx_buffer).unwrap();
    
    let mut i2s_tx = i2s.i2s_tx.with_bclk(bclk).with_ws(ws).with_dout(din).build();
    
    let mut transfer = i2s_tx.write_dma_circular(&mut rx_buffer).unwrap();
    
    let mut i: usize = 0;
    
    loop {
        // Timer::after(Duration::from_millis(1)).await;
        
        // Check this (crackle)
        let available = transfer.available().unwrap();
        dbg!(available);
        
        if available == 0 { continue };
        
        let data_size = available.min(512);
        let mut data: Vec<u8> = Vec::with_capacity(data_size);
        let next_i = i.wrapping_add((data_size) / 2);
        println!("calls: {}", next_i - i);
        for x in i..next_i {
            // let val = speaker::sin_wave(x);
            // dbg!(val);
            // data.push(val as u8);
            // data.push((val >> 8) as u8);
            if (x % 200) > 100 {
                data.push(0x00);
                data.push(0x10);
            } else {
                data.push(0x00);
                data.push(0x18);
            };
            // data.push(val);
            // data.push(0x10);
        }
        
        dbg!(i);
        
        i = next_i;
        
        dbg!(data.len());
        
        // let noise = speaker::Noise { current: 0x20, len: available};
        match transfer.push(&data) {
            Err(e) => {
                dbg!(e);
                None::<u8>.unwrap();
                break;
            }
            Ok(bytes_written) => {
                // i = i.wrapping_add(bytes_written);
            }
        }
        // break;
    }
}