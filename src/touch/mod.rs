use embassy_time::{Duration, Timer};
use esp_hal::{i2c::master::I2c, Blocking};
use esp_println::{dbg, println};

use crate::exio::{self, PinDirection, PinState};

const SPD2010_ADDR: u8 = 0x53;
const EXIO_TOUCH_RESET_PIN: u8 = 0;

pub async fn reset(i2c: &mut I2c<'_, Blocking>) {
    let pin_direction = exio::read_pin_direction(i2c, EXIO_TOUCH_RESET_PIN);
    println!("direction: {:?}", pin_direction);
    exio::set_pin_direction(i2c, EXIO_TOUCH_RESET_PIN, PinDirection::Output);
    exio::set_pin(i2c, EXIO_TOUCH_RESET_PIN, PinState::Low);
    Timer::after(Duration::from_millis(50)).await;
    exio::set_pin(i2c, EXIO_TOUCH_RESET_PIN, PinState::High);
    Timer::after(Duration::from_millis(50)).await;
}

pub fn read_touch(i2c: &mut I2c<'_, Blocking>, reg_addr: u16, reg_data: &mut [u8]) {
    let mut buf_addr: [u8; 2] = [0; 2];
    buf_addr[0] = (reg_addr >> 8) as u8;
    buf_addr[1] = reg_addr as u8;
    i2c.write_read(SPD2010_ADDR, &buf_addr, reg_data);
}

pub async fn read_fw_version(i2c: &mut I2c<'_, Blocking>) {
    for reg in (0..127).rev() {
        let input: [u8; 2] = [0, reg];
        let mut read_buffer: [u8; 20] = [0; 20];

        let result = i2c.write_read(SPD2010_ADDR, &input, &mut read_buffer);
        dbg!(result);

        println!("{}: {:?}", reg, read_buffer);
        
        Timer::after(Duration::from_millis(50)).await;
    }

    let mut sample_data: [u8; 2] = [0; 2];
    sample_data[0] = 0x26;
    sample_data[1] = 0x00;

    let mut read_buffer: [u8; 2] = [0; 2];

    let result = i2c.write_read(SPD2010_ADDR, &sample_data, &mut read_buffer);

    esp_println::dbg!(result);

    // let dummy = [
    //     read_buffer[0],
    //     read_buffer[1],
    //     read_buffer[3],
    //     read_buffer[0],
    // ];
    // let dver = [read_buffer[5], read_buffer[4]];
    // let pid = [
    //     read_buffer[9],
    //     read_buffer[8],
    //     read_buffer[7],
    //     read_buffer[6],
    // ];
    // let ic_name_l = [
    //     read_buffer[13],
    //     read_buffer[12],
    //     read_buffer[11],
    //     read_buffer[10],
    // ];
    // let ic_name_h = [
    //     read_buffer[17],
    //     read_buffer[16],
    //     read_buffer[15],
    //     read_buffer[14],
    // ];

    println!("{:?}", read_buffer);

    // println!("dummy[{dummy:?}], DVer[{dver:?}], PID[{pid:?}], NAME[{ic_name_l:?}-{ic_name_h:?}]");

    //    let dummy: u32 = ((sample_data[0] as u32) << 24) | ((sample_data[1] as u32) << 16) | ((sample_data[3] as u32) << 8) | (sample_data[0] as u32);

    // let dver: u16 = ((sample_data[5] as u16) << 8) | (sample_data[4] as u16);
    // let pid: u32 = (((sample_data[9] as u32) << 24) | ((sample_data[8] as u32) << 16) | ((sample_data[7] as u32) << 8) | (sample_data[6] as u32));
    // let ic_name_l: u32 = ((sample_data[13] << 24) | (sample_data[12] << 16) | (sample_data[11] << 8) | (sample_data[10]));    // "2010"
    // ICName_H = ((sample_data[17] << 24) | (sample_data[16] << 16) | (sample_data[15] << 8) | (sample_data[14]));    // "SPD"
    // printf("Dummy[%ld], DVer[%d], PID[%ld], Name[%ld-%ld]\r\n", Dummy, DVer, PID, ICName_H, ICName_L);
}
