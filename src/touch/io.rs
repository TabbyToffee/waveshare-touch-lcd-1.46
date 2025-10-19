use esp_hal::DriverMode;

use super::{Error, SPD2010_ADDR, SPD2010Touch};

impl<'a, Dm: DriverMode> SPD2010Touch<'a, Dm> {
    pub fn read_register(&mut self, reg: u16, data: &mut [u8]) -> Result<(), Error> {
        let reg_bytes = [reg as u8, (reg >> 8) as u8];
        self.i2c
            .write(SPD2010_ADDR, &reg_bytes)
            .map_err(Error::I2C)?;
        self.i2c.read(SPD2010_ADDR, data).map_err(Error::I2C)?;
        Ok(())
    }

    pub fn write_command(&mut self, reg: u16, data: &[u8]) -> Result<(), Error> {
        let reg_bytes = [reg as u8, (reg >> 8) as u8];
        self.i2c
            .write(SPD2010_ADDR, &reg_bytes)
            .map_err(Error::I2C)?;
        self.i2c.write(SPD2010_ADDR, data).map_err(Error::I2C)?;
        Ok(())
    }
}
