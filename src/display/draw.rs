use alloc::boxed::Box;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    framebuffer, pixelcolor::Rgb888, prelude::{Dimensions, DrawTarget, Point, RgbColor, Size}, primitives::Rectangle, Pixel
};
use esp_alloc::HeapStats;
use esp_hal::{
    gpio::{Input, InputConfig, InputPin, Pull}, spi::{
        self, master::{Address, Command, SpiDmaBus}, DataMode
    }, DriverMode
};
use esp_println::{dbg, println};

use crate::display::COLOR_BYTES;

use super::{
    init_cmd::LCD_INIT_CMD, lcd_command, opcode, BUFFER_SIZE, DISPLAY_HEIGHT, DISPLAY_WIDTH, DISPLAY_X_MAX, DISPLAY_Y_MAX, DMA_CHUNK_SIZE
};

pub struct Spd2010<'a, Dm>
where
    Dm: DriverMode,
    
{
    qspi: SpiDmaBus<'a, Dm>,
    pub framebuffer: Box<[u8]>,
    tear_input: Input<'a>,
}

impl<'a, Dm> Spd2010<'a, Dm>
where
    Dm: DriverMode,
{
    pub fn new(qspi: SpiDmaBus<'a, Dm>, tear_pin: impl InputPin + 'a) -> Self {
        // let mut buffer = [0_u8; 1024];

        println!("new 1");
        let framebuffer =
            unsafe { Box::<[u8]>::new_zeroed_slice(BUFFER_SIZE as usize).assume_init() };

        println!("new 2");

        // let stats: HeapStats = esp_alloc::HEAP.stats();
        // println!("{}", stats);

        
        let config = InputConfig::default().with_pull(Pull::Up);
        let mut tear_input = Input::new(tear_pin, config);

        Self { qspi, framebuffer, tear_input }
    }

    fn set_draw_pos(&mut self, x1: u16, y1: u16, x2: u16, y2: u16) -> Result<(), spi::Error> {
        // [ x1 (byte 2), x1 (byte 1), x2 (byte 2), x2 (byte 1) ]
        // 2 before 1 because Endian and stuff
        let x_set_data: [u8; 4] = [
            (x1 >> 8) as u8,
            (x1 & 0xFF) as u8,
            (x2 >> 8) as u8,
            (x2 & 0xFF) as u8,
        ];
        // dbg!(&x_set_data);
        self.send_command(lcd_command::CASET, &x_set_data)?;

        let y_set_data: [u8; 4] = [
            (y1 >> 8) as u8,
            (y1 & 0xFF) as u8,
            (y2 >> 8) as u8,
            (y2 & 0xFF) as u8,
        ];
        // dbg!(&y_set_data);
        self.send_command(lcd_command::RASET, &y_set_data)?;

        Ok(())
    }

    fn send_command(&mut self, cmd: u8, data: &[u8]) -> Result<(), spi::Error> {
        let address_value = (cmd as u32) << 8;
        self.qspi.half_duplex_write(
            DataMode::Single,
            Command::_8Bit(opcode::WRITE_CMD as u16, DataMode::Single),
            Address::_24Bit(address_value, DataMode::Single),
            0,
            data,
        )?;
        Ok(())
    }

    fn send_pixels(qspi: &mut SpiDmaBus<'a, Dm>, cmd: u8, pixels: &[u8]) -> Result<(), spi::Error> {
        let address_value = (cmd as u32) << 8;
        qspi.half_duplex_write(
            DataMode::Quad,
            Command::_8Bit(opcode::WRITE_COLOR as u16, DataMode::Single),
            Address::_24Bit(address_value, DataMode::Single),
            0,
            pixels,
        )?;
        Ok(())
    }
    
    pub fn draw_rect(&mut self, x1: u16, y1: u16, x2: u16, y2: u16, r: u8, g: u8, b: u8) {
        for x in x1..x2+1 {
            for y in y1..y2+1 {
                let index: u32 = 3 * ((y as u32 * DISPLAY_WIDTH) + x as u32);
                self.framebuffer[(index) as usize] = r;
                self.framebuffer[(index + 1) as usize] = g;
                self.framebuffer[(index + 2) as usize] = b;
            }
        }
    }

    pub async fn flush(&mut self) -> Result<(), spi::Error> {
        self.set_draw_pos(0, 0, (DISPLAY_WIDTH as u16) -1, (DISPLAY_HEIGHT as u16) -1)?;

        // let mut i: u8 = 0;
        // for byte in self.framebuffer.iter_mut() {
        //     *byte = i;
        //     i = i.wrapping_add(1);
        //     // if i == 0 {
        //     //     Timer::after(Duration::from_millis(10 as u64)).await;
        //     // }
        // }
        
        // Timer::after(Duration::from_millis(100 as u64)).await;
        
        // dbg!(&self.framebuffer);
        
        // let mut chunk_buffer: [u8; DMA_CHUNK_SIZE] = [0; DMA_CHUNK_SIZE];
        
        while self.tear_input.is_low() {}
        let mut is_first = true;
        for chunk in self.framebuffer.chunks(DMA_CHUNK_SIZE) {
            // chunk_buffer.clone_from_slice(chunk);
            
            if is_first {
                // dbg!(chunk);
                Self::send_pixels(&mut self.qspi, lcd_command::RAMWR, chunk)?;
                is_first = false;
            } else {
                // continue;
                Self::send_pixels(&mut self.qspi, lcd_command::RAMWRC, chunk)?;
            }
            
            // Timer::after(Duration::from_millis(50 as u64)).await;
        }

        Ok(())
    }

    pub async fn init(&mut self) -> Result<(), spi::Error> {
        for (cmd, delay, data) in LCD_INIT_CMD {
            self.send_command(*cmd, &data)?;
            Timer::after(Duration::from_millis(*delay as u64)).await;
        }
        
        self.send_command(lcd_command::DISPON, &[])?;

        Ok(())
    }

    pub fn fill(&mut self) {
        for i in 0..(self.framebuffer.len() -1) {
            self.framebuffer[i] = 0xff;
            
            // dbg!(self.framebuffer[i]);
        }
        
        
        // for pixel in self.framebuffer.iter_mut() {
        //     // dbg!(index % 3);
        //     *pixel = 0x55;
        //     // match (index % 3) {
        //     //     0 => {
        //     //         *pixel = 0xff;
        //     //     }
        //     //     1 => {
        //     //         *pixel = 0xff;
        //     //     }
        //     //     2 => {
        //     //         *pixel = 0xff;
        //     //     }
        //     //     _ => {
        //     //         panic!("WTF!")
        //     //     }
        //     // }
        // }
        // // dbg!(&self.framebuffer);
    }
    
    pub fn fill_2(&mut self) -> Result<(), spi::Error> {
        // let line_data =
        //     Box::<[u8]>::try_new_uninit_slice(DISPLAY_WIDTH as usize * COLOR_BYTES).unwrap();
        // // We must write to all of line_data before reading
        // let mut line_data = unsafe { line_data.assume_init() };
        
        let mut line_data: [u8; 412 * 3] = [0; 412 * 3];
    
        let mut rand: u8 = 0;
    
        // ranges dont include the end value so this runs for 0 - 411
        for line in 0..(DISPLAY_HEIGHT) as u16 {
            for x in 0..(DISPLAY_WIDTH) as usize {
                // for color_byte in 0..3 {
                //     line_data[x * 3 + color_byte] = rand;
                //     rand = rand.wrapping_add(1);
                // }
                // line_data[x * 3 + 0] = 0x00;
                // line_data[x * 3 + 1] = 90;
                // line_data[x * 3 + 2] = 60;
                line_data[x * 3 + 0] = 255 - (x / 2) as u8;
                line_data[x * 3 + 1] = 255 - (line / 2) as u8;
                line_data[x * 3 + 2] = 0xff;
            }
    
            self.set_draw_pos(0, line as u16, DISPLAY_WIDTH as u16 - 1, line + 1 as u16)?;
            
            Self::send_pixels(&mut self.qspi, lcd_command::RAMWR, &line_data)?;
        }
        Ok(())
    }
}

impl<'a, Dm> DrawTarget for Spd2010<'a, Dm>
where
    Dm: DriverMode,
{
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if let Ok((x @ 0..=DISPLAY_X_MAX, y @ 0..=DISPLAY_Y_MAX)) = coord.try_into() {
                // Calculate the index in the framebuffer.
                let pixel_index: u32 = ((y * (DISPLAY_WIDTH)) + x) * COLOR_BYTES as u32;
                // println!("{x}, {y} -> {pixel_index}");
                self.framebuffer[pixel_index as usize] = color.r();
                self.framebuffer[pixel_index as usize + 1] = color.g();
                self.framebuffer[pixel_index as usize + 2] = color.b();
                // self.framebuffer[pixel_index as usize] = 255;
                // self.framebuffer[pixel_index as usize + 1] = 255;
                // self.framebuffer[pixel_index as usize + 2] = 255;
                // println!("{}", self.framebuffer[pixel_index as usize]);
                
            }
        }

        Ok(())
    }
}

impl<'a, Dm> Dimensions for Spd2010<'a, Dm>
where
    Dm: DriverMode,
{
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT))
    }
}
