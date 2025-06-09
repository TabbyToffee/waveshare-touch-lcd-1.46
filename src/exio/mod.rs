use esp_hal::{
    i2c::master::{Config, I2c},
    Blocking,
};
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

pub fn read_register(i2c: &mut I2c<Blocking>, register_address: u8) -> u8 {
    let mut return_bytes: &mut [u8] = &mut [0; 1];
    i2c.write_read(TCA9554_ADDRESS, &[register_address], &mut return_bytes);
    return_bytes[0]
}

pub fn write_register(i2c: &mut I2c<Blocking>, register_address: u8, value: u8) {
    let result = i2c.write(TCA9554_ADDRESS, &[register_address, value]);
    println!("{:?}", result);
}

fn write_register_bit(i2c: &mut I2c<Blocking>, register: u8, bit: u8, value: bool) {
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

fn read_register_bit(i2c: &mut I2c<Blocking>, register: u8, bit: u8) -> bool {
    // C code starts at 1, this starts at 0!
    if bit > 7 {
        println!("Invalid setting");
        return false;
    }

    let reg_value = read_register(i2c, register);

    let bit_status = (reg_value >> bit) & 1 == 1; // Shift so requested bit is at right of byte, set other bits to 0

    bit_status
}

pub fn set_pin_direction(i2c: &mut I2c<Blocking>, pin: u8, direction: PinDirection) {
    write_register_bit(i2c, TCA9554_CONFIG_REG, pin, direction == PinDirection::Input);
}

pub fn set_pin(i2c: &mut I2c<Blocking>, pin: u8, state: PinState) {
    write_register_bit(i2c, TCA9554_OUTPUT_REG, pin, state == PinState::High);
}

pub fn read_pin_direction(i2c: &mut I2c<Blocking>, pin: u8) -> PinDirection {
    let bit_status = read_register_bit(i2c, TCA9554_CONFIG_REG, pin);
    
    if bit_status {
        PinDirection::Input
    } else {
        PinDirection::Output
    }
}

pub fn read_pin(i2c: &mut I2c<Blocking>, pin: u8) -> PinState {
    let bit_status = read_register_bit(i2c, TCA9554_OUTPUT_REG, pin);
    if bit_status {
        PinState::High
    } else {
        PinState::Low
    }
}

// pub fn set_pin(i2c: &mut I2c<Blocking>, pin: u8, state: PinState) {
//     println!("Set Pin");

//     let mut return_bytes: &mut [u8] = &mut [0; 20];
//     let mut data: u8 = 0;
//     let mut bits_status: u8 = 0;
//     i2c.read(TCA9554_ADDRESS, &mut return_bytes);

//     // for byte in return_bytes {
//     //     println!("byte: {:#b}", byte);
//     // }

//     if pin < 1 || pin > 8 {
//         println!("Invalid setting");
//         return;
//     }

//     match state {
//         // (0x01 << (pin-1)) -> Byte with 1 in pin position
//         // (0x01 << (pin-1)) | bits_status -> bits_status with 1 at pin position
//         PinState::High => {
//             bits_status |= 0x01 << (pin - 1);
//         }
//         PinState::Low => {
//             bits_status &= !(0x01 << (pin - 1));
//         }
//     }
//     println!("Writing to pin: {:#b}", bits_status);
//     i2c.write(TCA9554_OUTPUT_REG, &[bits_status]);
// }


// pub fn set_pin_direction(i2c: &mut I2c<Blocking>, pin: u8, direction: PinDirection) {
//     let direction_text = match direction {
//         PinDirection::Output => "output",
//         PinDirection::Input => "input",
//     };
//     println!("Set External IO Pin {pin} to {}", direction_text);

//     if pin < 1 || pin > 8 {
//         println!("Invalid setting");
//         return;
//     }

//     let mut config = read_register(i2c, TCA9554_CONFIG_REG);

//     println!("Current config register value = {config}");

//     // default pin direction is input (bit value = 1)
//     // set bit to zero for output
//     match direction {
//         PinDirection::Input => {
//             config |= 1 << (pin - 1);
//         }
//         PinDirection::Output => {
//             config &= !(1 << (pin - 1));
//         }
//     }

//     println!("Set config register value = {config}");

//     write_register(i2c, TCA9554_CONFIG_REG, config);

//     let new_config = read_register(i2c, TCA9554_CONFIG_REG);

//     println!("New config register value = {new_config}");
// }