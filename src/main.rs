#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::mem::size_of;

use defmt::info;
use embassy_executor::Executor;
use embassy_futures::select::{select, Either};
use embassy_rp::{
    bind_interrupts,
    flash::{Async, Flash, ERASE_SIZE},
    gpio::{Input, Pull},
    i2c::{Config as I2cConfig, I2c},
    multicore::{spawn_core1, Stack},
    peripherals::{DMA_CH0, DMA_CH1, FLASH, I2C1, PIN_2, PIN_26, PIN_3, PIN_4, PIO0},
    pio::{Config as PioConfig, FifoJoin, InterruptHandler, Pio, ShiftDirection},
    rom_data::{float_funcs::fdiv, reset_to_usb_boot},
    Peripheral,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{with_timeout, Duration, Timer};

use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{ascii::FONT_9X15_BOLD, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use encoder::{EncoderDirection, DELAY_DEFAULT};
use fixed::types::U24F8;
use pio_proc::pio_asm;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

include!(concat!(env!("OUT_DIR"), "/pdm_table.rs"));
include!(concat!(env!("OUT_DIR"), "/fontmap.rs"));

const ADDR_OFFSET: u32 = 2 * 1024 * 1024 - 4096;
const FLASH_SIZE: usize = 2 * 1024 * 1024;

const SUBTONES: [f32; 51] = [
    67.0, 69.3, 71.9, 74.4, 77.0, 79.7, 82.5, 85.4, 88.5, 91.5, 94.8, 97.4, 100.0, 103.5, 107.2,
    110.9, 114.8, 118.8, 123.0, 127.3, 131.8, 136.5, 141.3, 146.2, 150.0, 151.4, 156.7, 159.8,
    162.2, 165.5, 167.9, 171.3, 173.8, 177.3, 179.9, 183.5, 186.2, 189.9, 192.8, 196.6, 199.5,
    203.5, 206.5, 210.7, 218.1, 225.7, 229.1, 233.6, 241.8, 250.3, 254.1,
];

const POS: [Point; 5] = [
    Point::new(0, 8),
    Point::new(25, 8),
    Point::new(50, 8),
    Point::new(75, 8),
    Point::new(100, 8),
];

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
struct Message {
    freq: f32,
    enabled: bool,
}

#[derive(Copy, Clone)]
struct Config {
    pub counter: usize,
    pub enabled: bool,
    pub rotation: DisplayRotation,
    pub brightness: Brightness,
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        let _o_rotation = other.rotation;
        self.counter == other.counter
            && self.enabled == other.enabled
            && self.brightness == other.brightness
            && matches!(self.rotation, _o_rotation)
    }
}

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static CHANNEL: Channel<CriticalSectionRawMutex, Message, 1> = Channel::new();

mod encoder;

type Encoder = encoder::Encoder<'static, PIN_2, PIN_4>;
type Button = Input<'static, PIN_3>;
type I2cChan1 = embassy_rp::i2c::I2c<'static, I2C1, embassy_rp::i2c::Blocking>;

type Display = Ssd1306<
    ssd1306::prelude::I2CInterface<I2cChan1>,
    ssd1306::prelude::DisplaySize128x64,
    BufferedGraphicsMode<ssd1306::prelude::DisplaySize128x64>,
>;

// let mut flash = Flash::<_, Async, FLASH_SIZE>::new(flash, dma_1);
#[inline]
fn read_config(flash: &mut Flash<'_, FLASH, Async, FLASH_SIZE>) -> Config {
    let ref mut flash_buf = [0u8; size_of::<Config>()];
    defmt::unwrap!(flash.blocking_read(ADDR_OFFSET, flash_buf));
    let mut cfg = unsafe { *(flash_buf as *const u8 as *const Config) };
    if cfg.counter >= SUBTONES.len() {
        cfg.counter = 0;
        cfg.enabled = true;
        cfg.brightness = Brightness::NORMAL;
        cfg.rotation = DisplayRotation::Rotate0;
    };
    cfg
}

#[inline]
fn write_config(flash: &mut Flash<'_, FLASH, Async, FLASH_SIZE>, cfg: Config) {
    if read_config(flash) == cfg {
        return;
    }
    // info!("Flashing!");
    defmt::unwrap!(flash.blocking_erase(ADDR_OFFSET, ADDR_OFFSET + ERASE_SIZE as u32));
    let buf: &[u8] = unsafe {
        core::slice::from_raw_parts(&cfg as *const Config as *const u8, size_of::<Config>())
    };
    flash.blocking_write(ADDR_OFFSET, buf).unwrap();
}

#[inline]
fn freq_2_divider(freq: f32) -> U24F8 {
    U24F8::from_num(fdiv(CLK_DIV_1HZ / 4.0, freq))
}

#[inline]
fn display_freq(display: &mut Display, counter: usize, enabled: bool) {
    let mut v = (SUBTONES[counter] * 10.0_f32) as usize;
    let mut z = (v / 1000).clamp(0, 9);

    v %= 1000;
    if z == 0 {
        CHARS[Font::Fspace as usize]
            .translate(POS[0])
            .draw(display)
            .unwrap();
    } else {
        CHARS[z].translate(POS[0]).draw(display).unwrap()
    }
    z = (v / 100).clamp(0, 9);
    v %= 100;
    CHARS[z].translate(POS[1]).draw(display).unwrap();
    z = (v / 10).clamp(0, 9);
    v %= 10;
    CHARS[z].translate(POS[2]).draw(display).unwrap();
    z = (v).clamp(0, 9);
    if enabled {
        CHARS[Font::Fdot as usize]
            .translate(POS[3])
            .draw(display)
            .unwrap();
    } else {
        CHARS[Font::Foff as usize]
            .translate(POS[3])
            .draw(display)
            .unwrap();
    }
    CHARS[z].translate(POS[4]).draw(display).unwrap();
    display.flush().unwrap();
}

#[cortex_m_rt::entry]
fn main() -> ! {
    embassy_time::block_for(DELAY_DEFAULT);
    let p = embassy_rp::init(Default::default());

    // info!("Init Input");
    let enc = encoder::Encoder::new(p.PIN_2, p.PIN_4);
    let button = Input::new(p.PIN_3, Pull::Up);

    // info!("Set up I2c");
    let sda = p.PIN_6;
    let scl = p.PIN_7;
    let mut i2c_config = I2cConfig::default();
    i2c_config.frequency = 400_000;
    let i2c = I2c::new_blocking(p.I2C1, scl, sda, i2c_config);

    spawn_core1(p.CORE1, unsafe { &mut CORE1_STACK }, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| {
            spawner
                .spawn(core1_task(p.PIN_26, p.PIO0, p.DMA_CH0))
                .unwrap()
        });
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        spawner
            .spawn(core0_task(enc, button, i2c, p.FLASH, p.DMA_CH1))
            .unwrap()
    });
}

#[embassy_executor::task]
async fn core0_task(
    mut enc: Encoder,
    mut button: Button,
    i2c: I2cChan1,
    flash: FLASH,
    dma_1: DMA_CH1,
) {
    let mut flash = Flash::<_, Async, FLASH_SIZE>::new(flash, dma_1);
    let mut cfg = read_config(&mut flash);

    let interface = I2CDisplayInterface::new(i2c);
    let ref mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    display.set_brightness(cfg.brightness).unwrap();

    if button.is_low() {
        display.set_rotation(DisplayRotation::Rotate0).unwrap();
        display.set_brightness(Brightness::NORMAL).unwrap();
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_9X15_BOLD)
            .text_color(BinaryColor::On)
            .build();

        Text::with_baseline("UF2 Boot Mode", Point::zero(), text_style, Baseline::Top)
            .draw(display)
            .unwrap();
        display.flush().unwrap();
        reset_to_usb_boot(0, 0);
    }

    loop {
        CHANNEL
            .send(Message {
                freq: SUBTONES[cfg.counter],
                enabled: cfg.enabled,
            })
            .await;
        display_freq(display, cfg.counter, cfg.enabled);

        match select(enc.wait_for(), button.wait_for_low()).await {
            Either::First(direction) => match direction {
                EncoderDirection::Up => cfg.counter = (cfg.counter + 1) % (SUBTONES.len() - 1),
                EncoderDirection::Down => {
                    if cfg.counter == 0 {
                        cfg.counter = SUBTONES.len() - 1;
                    } else {
                        cfg.counter -= 1;
                    }
                }
            },
            Either::Second(_) => {
                Timer::after(DELAY_DEFAULT).await;
                match with_timeout(Duration::from_millis(750), button.wait_for_high()).await {
                    Ok(_) => cfg.enabled = !cfg.enabled,
                    Err(_) => {
                        CHARS[Font::Fmem as usize]
                            .translate(POS[3])
                            .draw(display)
                            .unwrap();
                        write_config(&mut flash, cfg);
                        display.flush().unwrap();
                        Timer::after(Duration::from_millis(750)).await;
                    }
                };
                Timer::after(DELAY_DEFAULT).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn core1_task(pdm_pin: PIN_26, pio_0: PIO0, dma_0: DMA_CH0) {
    bind_interrupts!(struct Irqs {
        PIO0_IRQ_0 => InterruptHandler<PIO0>;
    });

    let Pio {
        mut common,
        sm0: mut sm,
        ..
    } = Pio::new(pio_0, Irqs);
    let out_pin = common.make_pio_pin(pdm_pin);
    let Message { freq, enabled } = CHANNEL.receive().await;

    let prg = pio_asm!(
        ".origin 0",
        "set pindirs, 1",
        ".wrap_target",
        "out pins,1",
        ".wrap",
    );

    let mut cfg = PioConfig::default();
    cfg.use_program(&common.load_program(&prg.program), &[]);
    cfg.fifo_join = FifoJoin::TxOnly;
    cfg.set_out_pins(&[&out_pin]);
    cfg.set_set_pins(&[&out_pin]);
    cfg.clock_divider = freq_2_divider(freq);
    cfg.shift_out.auto_fill = true;
    cfg.shift_out.direction = ShiftDirection::Left;

    sm.set_config(&cfg);
    sm.set_enable(enabled);

    let mut dma_out_ref = dma_0.into_ref();

    loop {
        match select(
            CHANNEL.receive(),
            sm.tx().dma_push(dma_out_ref.reborrow(), &PDM_TABLE.0),
        )
        .await
        {
            Either::First(Message { freq, enabled }) => {
                info!("Got Message: {} {}", freq, enabled);
                sm.set_enable(false);
                cfg.clock_divider = freq_2_divider(freq);
                sm.set_config(&cfg);
                sm.set_enable(enabled);
            }
            Either::Second(_) => (),
        }
    }
}
