pub const DISPLAY_WIDTH: u32 = 412;
pub const DISPLAY_HEIGHT: u32 = 412;
pub const DISPLAY_X_MAX: u32 = DISPLAY_WIDTH - 1;
pub const DISPLAY_Y_MAX: u32 = DISPLAY_HEIGHT - 1;
pub const COLOR_BYTES: usize = 3;
pub const BUFFER_SIZE: u32 = DISPLAY_WIDTH * DISPLAY_HEIGHT * COLOR_BYTES as u32; // 3 bytes per pixel
pub const DMA_CHUNK_SIZE: usize = 412 * 4 * 3;

// The format is [ OPCODE, 0, CMD, 0, 0, PARAMS ]

// const LCD_OPCODE_WRITE_CMD: u8 = 0x02;
// const LCD_OPCODE_READ_CMD: u8 = 0x0B;
// const LCD_OPCODE_WRITE_COLOR: u8 = 0x32;
// const PARAMS_MAX_LEN: u8 = 4;

pub const ESP_PANEL_LCD_SPI_CLK_MHZ: u32 = 80; // 80Mhz
pub const SPD2010_CMD_SET: u8 = 0xFF;
pub const SPD2010_CMD_SET_BYTE0: u8 = 0x20;
pub const SPD2010_CMD_SET_BYTE1: u8 = 0x10;
pub const SPD2010_CMD_SET_USER: u8 = 0x00;
pub const EXIO_LCD_RESET_PIN: u8 = 1;

pub mod opcode {
    pub const WRITE_CMD: u8 = 0x02;
    pub const READ_CMD: u8 = 0x0B;
    pub const WRITE_COLOR: u8 = 0x32;
}

// Guessed values
// const LCD_CMD_MADCTL: u8 = 0x36;
// const LCD_CMD_COLMOD: u8 = 0x3A; // 0x2A;

pub mod lcd_command {
    pub const NOP: u8 = 0x00; // This command is empty command
    pub const SWRESET: u8 = 0x01; // Software reset registers (the built-in frame buffer is not affected)
    pub const RDDID: u8 = 0x04; // Read 24-bit display ID
    pub const RDDST: u8 = 0x09; // Read display status
    pub const RDDPM: u8 = 0x0A; // Read display power mode
    pub const RDD_MADCTL: u8 = 0x0B; // Read display MADCTL
    pub const RDD_COLMOD: u8 = 0x0C; // Read display pixel format
    pub const RDDIM: u8 = 0x0D; // Read display image mode
    pub const RDDSM: u8 = 0x0E; // Read display signal mode
    pub const RDDSR: u8 = 0x0F; // Read display self-diagnostic result
    pub const SLPIN: u8 = 0x10; // Go into sleep mode (DC/DC, oscillator, scanning stopped, but keeps content)
    pub const SLPOUT: u8 = 0x11; // Exit sleep mode
    pub const PTLON: u8 = 0x12; // Turns on partial display mode
    pub const NORON: u8 = 0x13; // Turns on normal display mode
    pub const INVOFF: u8 = 0x20; // Recover from display inversion mode
    pub const INVON: u8 = 0x21; // Go into display inversion mode
    pub const GAMSET: u8 = 0x26; // Select Gamma curve for current display
    pub const DISPOFF: u8 = 0x28; // Display off (disable frame buffer output)
    pub const DISPON: u8 = 0x29; // Display on (enable frame buffer output)
    pub const CASET: u8 = 0x2A; // Set column address
    pub const RASET: u8 = 0x2B; // Set row address
    pub const RAMWR: u8 = 0x2C; // Write frame memory
    pub const RAMRD: u8 = 0x2E; // Read frame memory
    pub const PTLAR: u8 = 0x30; // Define the partial area
    pub const VSCRDEF: u8 = 0x33; // Vertical scrolling definition
    pub const TEOFF: u8 = 0x34; // Turns off tearing effect
    pub const TEON: u8 = 0x35; // Turns on tearing effect
    pub const MADCTL: u8 = 0x36; // Memory data access control
    pub const VSCSAD: u8 = 0x37; // Vertical scroll start address
    pub const IDMOFF: u8 = 0x38; // Recover from IDLE mode
    pub const IDMON: u8 = 0x39; // Fall into IDLE mode (8 color depth is displayed)
    pub const COLMOD: u8 = 0x3A; // Defines the format of RGB picture data
    pub const RAMWRC: u8 = 0x3C; // Memory write continue
    pub const RAMRDC: u8 = 0x3E; // Memory read continue
    pub const STE: u8 = 0x44; // Set tear scan line, tearing effect output signal when display reaches line N
    pub const GDCAN: u8 = 0x45; // Get scan line
    pub const WRDISBV: u8 = 0x51; // Write display brightness
    pub const RDDISBV: u8 = 0x52; // Read display brightness value
}
