use esp_hal::{dma::DmaChannel0, dma_buffers, i2s::master::{DataFormat, I2s, RegisterAccess, Standard}, peripheral::Peripheral, peripherals::{I2S0}, time::Rate, Blocking};

// DMA buffer size
const I2S_BYTES: usize = 4092;

// pub fn init(i2s_peripheral: I2S0, dma_channel_peripheral: DmaChannel0) -> I2s<'_, Blocking> {
//     let (mut rx_buffer, rx_descriptors, _, tx_descriptors) = dma_buffers!(0, 4 * I2S_BYTES);
//     I2s::new(i2s_peripheral, Standard::Philips, DataFormat::Data32Channel32, Rate::from_hz(22050), dma_channel_peripheral, rx_descriptors, tx_descriptors)
// }