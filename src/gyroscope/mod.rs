use embassy_time::Timer;
use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};
use esp_println::{dbg, println};
use qmi8658::{
    command::register::{
        ctrl1::Ctrl1Register,
        ctrl3::{Ctrl3Register, GyroscopeFS, GyroscopeODR},
        ctrl7::Ctrl7Register,
    },
    Qmi8658,
};

pub fn init<'a>(i2c: &'a mut I2c<'a, Blocking>) -> Qmi8658<&'a mut I2c<'a, Blocking>, Delay> {
    let mut gyroscope = Qmi8658::new_secondary_address(i2c, Delay::new());

    match gyroscope.get_device_id() {
        Ok(rev) => {
            println!("QMI8658 Device ID: {:?}", rev);
        }
        Err(err) => {
            println!("QMI8658 not found {err:?}");
        }
    };

    match gyroscope.get_device_revision_id() {
        Ok(rev) => {
            println!("QMI8658 Device Revision ID: {:x}", rev);
        }
        Err(err) => {
            println!("QMI8658 not found");
        }
    };

    dbg!(gyroscope.gyroscope_test());
    dbg!(gyroscope.accelerometer_test());
    gyroscope.set_pedometer_enable(true);

    let mut ctrl1 = Ctrl1Register(0);
    ctrl1.set_int1_enable(true);
    ctrl1.set_be(true); // Big endian
    ctrl1.set_int2_enable(true);
    dbg!(gyroscope.set_ctrl1(ctrl1));

    let mut ctrl7 = Ctrl7Register(0);
    ctrl7.set_gyroscope_enable(true);
    ctrl7.set_accelerometer_enable(true);
    ctrl7.set_sync_sample_enable(false);
    ctrl7.set_data_ready_disable(true);
    if let Err(e) = gyroscope.set_ctrl7(ctrl7) {
        println!("QMI8658 write set_ctrl7 error: {:?}", e);
    }

    let mut ctrl3: Ctrl3Register = Ctrl3Register(0);
    ctrl3.set_godr(GyroscopeODR::NormalGORD8);
    ctrl3.set_gfs(GyroscopeFS::DPS256);
    ctrl3.set_gst(false);

    if let Err(e) = gyroscope.set_ctrl3(ctrl3) {
        println!("QMI8658 write set_ctrl7 error: {:?}", e);
    }

    gyroscope
}

pub fn read(gyroscope: &mut Qmi8658<&mut I2c<Blocking>, Delay>) {
    let temp = gyroscope.get_temperature();
    let accel = gyroscope.get_acceleration().unwrap();
    let steps = gyroscope.get_step_cnt();
    let steps2 = gyroscope.get_pedometer_step_count();
    println!("temp: {:?}", temp);
    println!(
        "accel: {{ x: {}, y: {}, z: {} }}",
        accel.x, accel.y, accel.z
    );
    println!("steps: {:?}, {:?}", steps, steps2);
}
