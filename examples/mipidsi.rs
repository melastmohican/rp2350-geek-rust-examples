#![no_std]
#![no_main]

use rp235x_hal::{self as hal, entry, gpio, spi, Clock};

use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_hal::delay::DelayNs;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::primitives::{Rectangle, PrimitiveStyleBuilder};
use embedded_graphics::image::{Image, ImageRaw, ImageRawLE};
use embedded_graphics::prelude::{Drawable, Primitive};
use embedded_graphics_core::draw_target::DrawTarget;
use embedded_graphics_core::pixelcolor::Rgb565;
use embedded_graphics_core::prelude::RgbColor;
use mipidsi::models::ST7789;
use mipidsi::options::{Orientation, Rotation};
use mipidsi::Builder;
use mipidsi::options::ColorInversion::Inverted;
use display_interface_spi::SPIInterface;
use embedded_hal::digital::OutputPin;
use tinybmp::Bmp;
use panic_probe as _;
use rp235x_hal::block::ImageDef;
use rp235x_hal::fugit::RateExtU32;
use rp235x_hal::gpio::FunctionSpi;

/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: ImageDef = hal::block::ImageDef::secure_exe();


/// External high-speed crystal on the Raspberry Pi Pico 2 board is 12 MHz.
/// Adjust if your board has a different frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();
    let _core = cortex_m::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
        .ok()
        .unwrap();

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins up according to their function on this particular board
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let dc = pins.gpio8.into_push_pull_output();
    let cs = pins.gpio9.into_push_pull_output();
    let sck = pins.gpio10.into_function::<FunctionSpi>();
    let mosi = pins.gpio11.into_function::<FunctionSpi>();
    let rst = pins.gpio12.into_push_pull_output_in_state(gpio::PinState::High);
    let miso = pins.gpio24.into_function::<FunctionSpi>();
    let mut bl = pins.gpio25.into_push_pull_output();

    let spi = spi::Spi::<_, _, _, 8>::new(pac.SPI1, (mosi, miso, sck)).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16_000_000u32.Hz(),
        embedded_hal::spi::MODE_0,
    );

    let mut delay = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    bl.set_high().ok();

    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let di = SPIInterface::new(spi_device, dc);

    let mut display = Builder::new(ST7789, di)
        .reset_pin(rst)
        .orientation(Orientation::new().rotate(Rotation::Deg90))
        .invert_colors(Inverted)
        .display_size(135, 240)
        .display_offset(52, 40)
        .init(&mut delay)
        .unwrap();

    // Give display time to fully initialize
    delay.delay_ns(100_000_000u32);

    // Clear display to black
    display.clear(Rgb565::BLACK).unwrap();
    // Draw images
    let image_raw: ImageRawLE<Rgb565> = ImageRaw::new(include_bytes!("ferris.raw"), 86);
    let image: Image<_> = Image::new(&image_raw, Point::new(150, 8));
    image.draw(&mut display).unwrap();
    
    let raw_image: Bmp<Rgb565> = Bmp::from_slice(include_bytes!("rust.bmp")).unwrap();
    let image = Image::new(&raw_image, Point::new(0, 0));
    image.draw(&mut display).unwrap();

    loop {
        cortex_m::asm::wfi(); // sleep infinitely
    }
}
