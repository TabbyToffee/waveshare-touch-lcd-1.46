use esp_hal::{
    Blocking, dma_buffers,
    i2s::master::{DataFormat, I2s, Standard},
    peripherals::{self, DMA_CH0, GPIO38, GPIO47, GPIO48, Peripherals},
    time::Rate,
};
use esp_println::{dbg, println};
use libm;

use crate::alloc::vec::Vec;

const mid: u16 = u16::MAX / 2;

pub fn init(i2s: &mut I2s<Blocking>) {}

pub fn sin_wave(time: usize) -> u16 {
    let sined = libm::sin((time as f64) / 32f64);
    ((sined * 16f64) + mid as f64) as u16
}

pub fn square_wave(time: usize) -> u16 {
    match time % 256 {
        0..128 => 0x0000,
        128.. => 0xF000,
    }
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

pub fn test(
    i2s_peripheral: peripherals::I2S0,
    dma_channel: DMA_CH0,
    bclk: GPIO48,
    din: GPIO47,
    ws: GPIO38,
) {
    const I2S_BYTES: usize = 4096;
    let (mut rx_buffer, rx_descriptors, _, tx_descriptors) =
        dma_buffers!(32 * I2S_BYTES, 32 * I2S_BYTES);
    let mut i2s = I2s::new(
        i2s_peripheral,
        Standard::Philips,
        DataFormat::Data32Channel16,
        Rate::from_khz(32),
        dma_channel,
    );
    // let mut i2s = I2s::new(i2s_peripheral, Standard::Philips, DataFormat::Data16Channel16, Rate::from_hz(100), i2s_dma_channel, rx_descriptors, tx_descriptors);

    // let mut i2s_rx = i2s.i2s_rx.with_bclk(bclk).with_ws(ws).with_din(din).build();

    // let mut transfer = i2s_rx.read_dma_circular(&mut rx_buffer).unwrap();

    let mut i2s_tx = i2s
        .i2s_tx
        .with_bclk(bclk)
        .with_ws(ws)
        .with_dout(din)
        .build(tx_descriptors);

    let mut transfer = i2s_tx.write_dma_circular(&mut rx_buffer).unwrap();

    // let mut sound_index: usize = 0;
    let mut generated: usize = 0;

    loop {
        // Timer::after(Duration::from_millis(1)).await;

        // Check this (crackle)
        let available = transfer.available().unwrap();
        // dbg!(available);

        if available == 0 {
            continue;
        };

        let data_size = available.min(8192);
        let mut data: Vec<u8> = Vec::with_capacity(data_size);

        // dbg!(data_size);

        // Generates sound from current index to next index
        // let next_sound_index = sound_index.wrapping_add(data_size);

        let stop_time: usize = generated.wrapping_add(data_size / 2 * 2);

        while generated != stop_time {
            let amplitude = sin_wave(generated);

            data.push((amplitude >> 8) as u8);
            data.push((amplitude & 0xff) as u8);
            // println!("loop");

            generated = generated.wrapping_add(2);
        }

        dbg!(generated);

        // for i in 0..data_size {
        //     data.push(0x80);
        // }

        // let next_i = i.wrapping_add((data_size) / 2);
        // println!("calls: {}", next_i - i);
        // for x in i..next_i {
        //     // let val = speaker::sin_wave(x);
        //     // dbg!(val);
        //     // data.push(val as u8);
        //     // data.push((val >> 8) as u8);
        //     if (x % 200) > 100 {
        //         data.push(0x00);
        //         data.push(0x10);
        //     } else {
        //         data.push(0x00);
        //         data.push(0x18);
        //     };
        //     // data.push(val);
        //     // data.push(0x10));
        // }

        // dbg!(i);

        // sound_index = ;

        // dbg!(data.len());

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
