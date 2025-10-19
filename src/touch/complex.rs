use embassy_time::{Duration, Timer};
use esp_hal::DriverMode;
use esp_println::println;

use super::{
    Error, SPD2010_MAX_TOUCH_POINTS, SPD2010Touch, StatusHigh, StatusLow, TouchData, TouchPoint,
    TouchStatus,
};

impl<'a, Dm: DriverMode> SPD2010Touch<'a, Dm> {
    pub fn read_fw_version(&mut self) -> Result<(), Error> {
        let mut data: [u8; 18] = [0; 18];
        self.read_register(0x2600, &mut data)?;
        let dummy: u32 = ((data[0] as u32) << 24)
            | ((data[1] as u32) << 16)
            | ((data[3] as u32) << 8)
            | data[0] as u32;
        let dver: u16 = ((data[5] as u16) << 8) | data[4] as u16;
        let pid: u32 = ((data[9] as u32) << 24)
            | ((data[8] as u32) << 16)
            | ((data[7] as u32) << 8)
            | data[6] as u32;
        let ic_name_l: u32 = ((data[13] as u32) << 24)
            | ((data[12] as u32) << 16)
            | ((data[11] as u32) << 8)
            | data[10] as u32;
        let ic_name_h: u32 = ((data[9] as u32) << 17)
            | ((data[16] as u32) << 16)
            | ((data[15] as u32) << 8)
            | data[14] as u32;

        println!(
            "SPD2010 - Dummy: {dummy}, Version: {dver}, PID: {pid}, IC Name: {ic_name_h}-{ic_name_l}"
        );
        Ok(())
    }

    fn read_status_length(&mut self) -> Result<TouchStatus, Error> {
        let mut data: [u8; 4] = [0; 4];
        self.read_register(0x0020, &mut data)?;

        let mut read_len: u16 = ((data[3] as u16) << 8) | data[2] as u16;
        if read_len < 4 || read_len > 64 {
            read_len = 0
        };

        // println!("len: {}", read_len);

        // println!("pt_exist: {}", (data[0] & 0x01));
        // println!("gesture: {}", (data[0] & 0x02));
        // println!("aux: {}", (data[0] & 0x08));
        // println!("tic_busy: {}", (data[1] & 0x80) >> 7);
        // println!("tic_in_bios: {}", (data[1] & 0x40) >> 6);
        // println!("tic_in_cpu: {}", (data[1] & 0x20) >> 5);
        // println!("tint_low: {}", (data[1] & 0x10) >> 4);
        // println!("cpu_run: {}", (data[1] & 0x08) >> 3);

        let status_low = StatusLow {
            pt_exist: (data[0] & 0x01) == 1,
            gesture: (data[0] & 0x02) == 1,
            key: false,
            aux: (data[0] & 0x08) == 1,
            keep: false,
            raw_or_pt: false,
            none6: false,
            none7: false,
        };

        let status_high = StatusHigh {
            none0: false,
            none1: false,
            none2: false,
            cpu_run: (data[1] & 0x08) == 1,
            tint_low: (data[1] & 0x10) == 1,
            tic_in_cpu: (data[1] & 0x20) == 1,
            tic_in_bios: (data[1] & 0x40) == 1,
            tic_busy: (data[1] & 0x80) == 1,
        };

        Ok(TouchStatus {
            status_low,
            status_high,
            read_len,
        })
    }

    fn read_hdp(&mut self, touch_status: TouchStatus, touch: &mut TouchData) -> Result<(), Error> {
        let mut data: [u8; 64] = [0; 64]; // Maximum expected data size

        self.read_register(0x0300, &mut data)?; // CHECK: Data may be to big
        // println!("{:?}", data);

        let check_id = data[4];

        if check_id <= 0x0A && touch_status.status_low.pt_exist {
            touch.touch_count =
                (((touch_status.read_len - 4) / 6) as u8).min(SPD2010_MAX_TOUCH_POINTS as u8);

            for touch_index in 0..(touch.touch_count as usize) {
                let offset: usize = touch_index * 6;
                let touch_point = TouchPoint {
                    id: data[4 + offset],
                    x: ((data[7 + offset] as u16 & 0xF0) << 4) | data[5 + offset] as u16,
                    y: ((data[7 + offset] as u16 & 0x0F) << 8) | data[6 + offset] as u16,
                    weight: data[8 + offset],
                };
                touch.points.push(touch_point);
            }

            // For slide gesture recognition
            if touch.points[0].weight != 0 && !touch.down {
                touch.down = true;
                touch.up = false;
                touch.down_x = touch.points[0].x;
                touch.down_y = touch.points[0].y;
            } else if touch.points[0].weight == 0 && touch.down {
                touch.up = true;
                touch.down = false;
                touch.up_x = touch.points[0].x;
                touch.up_y = touch.points[0].y;
            }
        } else if check_id == 0xF6 && touch_status.status_low.gesture {
            touch.touch_count = 0;
            touch.up = false;
            touch.down = false;
            touch.gesture = data[6] & 0x07;
        } else {
            touch.touch_count = 0;
            touch.gesture = 0;
        }

        Ok(())
    }

    pub async fn read_touch_data(&mut self, touch_data: &mut TouchData) -> Result<bool, Error> {
        // True = New data
        // False = No data

        let touch_status = self.read_status_length()?;

        println!("{:?}", touch_status);

        if touch_status.status_high.tic_in_bios {
            self.clear_interrupt().await?; // ACK+re-arm+verify
            // write CPU start command
            self.write_command(0x0004, &[0x01, 0x00])?;
            return Ok(false);
        }

        if touch_status.status_high.tic_in_cpu {
            // write point mode command
            self.write_command(0x0050, &[0x00, 0x00])?;
            // write start command
            self.write_command(0x0046, &[0x00, 0x00])?;
            self.clear_interrupt().await?;
            return Ok(false);
        }

        if touch_status.status_high.cpu_run && touch_status.read_len == 0 {
            self.clear_interrupt().await?;
            return Ok(false);
        }

        if touch_status.status_low.pt_exist || touch_status.status_low.gesture {
            println!("data!");
            self.clear_interrupt().await?;

            let touch_data = self.read_hdp(touch_status, touch_data).unwrap();
            println!("{:?}", touch_data);
            return Ok(true);
        }

        if touch_status.status_high.cpu_run && touch_status.status_low.aux {
            self.clear_interrupt().await?; // clear & re-arm
        }

        Ok(false)
    }
}
