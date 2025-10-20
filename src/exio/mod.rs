use esp_hal::{Async, i2c::master::I2c};
use esp_println::println;

const TCA9554_ADDRESS: u8 = 0x20;
const TCA9554_INPUT_REG: u8 = 0x00;
const TCA9554_OUTPUT_REG: u8 = 0x01;
const TCA9554_POLARITY_REG: u8 = 0x02;
const TCA9554_CONFIG_REG: u8 = 0x03;

#[derive(PartialEq, Debug)]
pub enum PinDirection {
    Input,
    Output,
}

#[derive(PartialEq, Debug)]
pub enum PinState {
    High,
    Low,
}

pub fn read_register(i2c: &mut I2c<Async>, register_address: u8) -> u8 {
    let mut return_bytes: &mut [u8] = &mut [0; 1];
    i2c.write_read(TCA9554_ADDRESS, &[register_address], &mut return_bytes);
    return_bytes[0]
}

pub fn write_register(i2c: &mut I2c<Async>, register_address: u8, value: u8) {
    let result = i2c.write(TCA9554_ADDRESS, &[register_address, value]);
    println!("{:?}", result);
}

fn write_register_bit(i2c: &mut I2c<Async>, register: u8, bit: u8, value: bool) {
    // C code starts at 1, this starts at 0!
    if bit > 7 {
        println!("Invalid setting");
        return;
    }

    // Have to read register to only change 1 pin and keep others intact
    let mut reg_value = read_register(i2c, register);

    println!("Current register value = {reg_value:0b}");

    // default pin direction is input (bit value = 1)
    // set bit to zero for output
    match value {
        true => {
            reg_value |= 1 << (bit);
        }
        false => {
            reg_value &= !(1 << (bit));
        }
    }

    println!("Set register value = {reg_value:0b}");

    write_register(i2c, register, reg_value);

    let new_value = read_register(i2c, register);

    println!("New register value = {new_value:0b}");
}

fn read_register_bit(i2c: &mut I2c<Async>, register: u8, bit: u8) -> bool {
    // C code starts at 1, this starts at 0!
    if bit > 7 {
        println!("Invalid setting");
        return false;
    }

    let reg_value = read_register(i2c, register);

    (reg_value >> bit) & 1 == 1 // Shift so requested bit is at right of byte, set other bits to 0
}

pub fn set_pin_direction(i2c: &mut I2c<Async>, pin: u8, direction: PinDirection) {
    write_register_bit(
        i2c,
        TCA9554_CONFIG_REG,
        pin,
        direction == PinDirection::Input,
    );
}

pub fn set_pin(i2c: &mut I2c<Async>, pin: u8, state: PinState) {
    write_register_bit(i2c, TCA9554_OUTPUT_REG, pin, state == PinState::High);
}

pub fn read_pin_direction(i2c: &mut I2c<Async>, pin: u8) -> PinDirection {
    let bit_status = read_register_bit(i2c, TCA9554_CONFIG_REG, pin);

    if bit_status {
        PinDirection::Input
    } else {
        PinDirection::Output
    }
}

pub fn read_pin(i2c: &mut I2c<Async>, pin: u8) -> PinState {
    let bit_status = read_register_bit(i2c, TCA9554_OUTPUT_REG, pin);
    if bit_status {
        PinState::High
    } else {
        PinState::Low
    }
}
