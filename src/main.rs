#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::mem::size_of;

use defmt::{info, unwrap};
use embassy_executor::Executor;
use embassy_futures::select::{select, Either};
use embassy_rp::flash::{Async, Flash, ERASE_SIZE};
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, FLASH, I2C1, PIN_2, PIN_26, PIN_3, PIN_4, PIO0};
use embassy_rp::pio::{Config as PioConfig, FifoJoin, InterruptHandler, Pio, ShiftDirection};
use embassy_rp::{bind_interrupts, Peripheral};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{with_timeout, Duration, Timer};

use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::*,
};
use encoder::{EncoderDirection, DELAY_DEFAULT};

use fixed::types::U24F8;
use pio_proc::pio_asm;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

include!(concat!(env!("OUT_DIR"), "/pdm_table.rs"));
include!(concat!(env!("OUT_DIR"), "/fontmap.rs"));

const ADDR_OFFSET: u32 = 2 * 1024 * 1024 - 4096;
const FLASH_SIZE: usize = 2 * 1024 * 1024;

#[cfg(not(feature = "call-tone"))]
const SUBTONES: [f32; 51] = [
    67.0, 69.3, 71.9, 74.4, 77.0, 79.7, 82.5, 85.4, 88.5, 91.5, 94.8, 97.4, 100.0, 103.5, 107.2,
    110.9, 114.8, 118.8, 123.0, 127.3, 131.8, 136.5, 141.3, 146.2, 150.0, 151.4, 156.7, 159.8,
    162.2, 165.5, 167.9, 171.3, 173.8, 177.3, 179.9, 183.5, 186.2, 189.9, 192.8, 196.6, 199.5,
    203.5, 206.5, 210.7, 218.1, 225.7, 229.1, 233.6, 241.8, 250.3, 254.1,
];
#[cfg(feature = "call-tone")]
const SUBTONES: [f32; 52] = [
    67.0, 69.3, 71.9, 74.4, 77.0, 79.7, 82.5, 85.4, 88.5, 91.5, 94.8, 97.4, 100.0, 103.5, 107.2,
    110.9, 114.8, 118.8, 123.0, 127.3, 131.8, 136.5, 141.3, 146.2, 150.0, 151.4, 156.7, 159.8,
    162.2, 165.5, 167.9, 171.3, 173.8, 177.3, 179.9, 183.5, 186.2, 189.9, 192.8, 196.6, 199.5,
    203.5, 206.5, 210.7, 218.1, 225.7, 229.1, 233.6, 241.8, 250.3, 254.1, 1750.0,
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
struct Message<'a> {
    divider: U24F8,
    pdm_table: &'a [u32],
    enabled: bool,
}

#[derive(Clone, Copy, PartialEq)]
struct Config {
    counter: usize,
    enabled: bool,
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

macro_rules! send_msg {
    ($dividers:ident, $pdm_tables:ident, $counter:expr, $enabled:expr) => {
        CHANNEL
            .send(Message {
                divider: $dividers[$counter],
                pdm_table: $pdm_tables[$counter],
                enabled: $enabled,
            })
            .await;
        Timer::after(Duration::from_millis(10)).await;
    };
}

#[inline]
fn read_config<'a>(flash: &mut Flash<'a, FLASH, Async, FLASH_SIZE>) -> Config {
    let ref mut flash_buf = [0u8; size_of::<Config>()];
    flash.read(ADDR_OFFSET, flash_buf).unwrap();
    unsafe { *(flash_buf as *const u8 as *const Config) }
}

#[inline]
fn write_config(flash: &mut Flash<'_, FLASH, Async, FLASH_SIZE>, cfg: Config) {
    if read_config(flash) == cfg {
        return;
    }
    info!("Flashing!");
    flash
        .erase(ADDR_OFFSET, ADDR_OFFSET + ERASE_SIZE as u32)
        .unwrap();
    let buf: &[u8] = unsafe {
        core::slice::from_raw_parts(&cfg as *const Config as *const u8, size_of::<Config>())
    };
    flash.write(ADDR_OFFSET, buf).unwrap();
}

#[inline]
fn freq_2_divider(freq: f32) -> U24F8 {
    #[cfg(not(feature = "call-tone"))]
    {
        U24F8::from_num(CLK_DIV_1HZ / freq)
    }
    #[cfg(feature = "call-tone")]
    {
        if freq < 1000.0 {
            U24F8::from_num(CLK_DIV_1HZ / freq)
        } else {
            U24F8::from_num(CLK_DIV_8HZ / freq)
        }
    }
}

#[inline]
fn freq_2_pdm_table(freq: f32) -> &'static [u32] {
    if freq < 1000.0 {
        &PDM_TABLE
    } else {
        &PDM8_TABLE
    }
}

#[inline]
fn display_freq(display: &mut Display, counter: usize, enabled: bool) {
    // one decimal place
    let mut v = (SUBTONES[counter] * 10.0_f32) as usize;
    if v >= 10000 {
        let mut leading_zero = true;
        for p in 0..5 {
            let divider = 10_usize.pow(5 - p);
            let z = (v / divider).clamp(0, 9);
            v %= divider;
            if leading_zero && z == 0 {
                CHARS[Font::Fspace as usize]
                    .translate(POS[p as usize])
                    .draw(display)
                    .unwrap();
            } else {
                leading_zero = false;
                CHARS[z].translate(POS[p as usize]).draw(display).unwrap()
            }
        }
    } else {
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
    }
    display.flush().unwrap();
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    info!("Init Input");
    let enc = encoder::Encoder::new(Input::new(p.PIN_2, Pull::Up), Input::new(p.PIN_4, Pull::Up));
    let button = Input::new(p.PIN_3, Pull::Up);

    info!("Set up I2c");
    let sda = p.PIN_6;
    let scl = p.PIN_7;
    let i2c = I2c::new_blocking(p.I2C1, scl, sda, I2cConfig::default());

    spawn_core1(p.CORE1, unsafe { &mut CORE1_STACK }, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| unwrap!(spawner.spawn(core1_task(p.PIN_26, p.PIO0, p.DMA_CH0))));
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0
        .run(|spawner| unwrap!(spawner.spawn(core0_task(enc, button, i2c, p.FLASH, p.DMA_CH1))));
}

#[embassy_executor::task]
async fn core0_task(
    mut enc: Encoder,
    mut button: Button,
    i2c: I2cChan1,
    flash: FLASH,
    dma_1: DMA_CH1,
) {
    info!("Hello from core 0");

    let dividers = SUBTONES.map(freq_2_divider);
    let pdm_tables = SUBTONES.map(freq_2_pdm_table);

    let mut flash = Flash::<_, Async, FLASH_SIZE>::new(flash, dma_1);
    let Config {
        mut counter,
        mut enabled,
    } = read_config(&mut flash);

    info!("Set up Display");
    let interface = I2CDisplayInterface::new(i2c);
    let ref mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate180)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    display.set_brightness(Brightness::DIM).unwrap();

    if counter >= SUBTONES.len() {
        counter = 0;
    }

    loop {
        #[cfg(not(feature = "call-tone"))]
        send_msg!(dividers, pdm_tables, counter, enabled);
        #[cfg(feature = "call-tone")]
        {
            if SUBTONES[counter] < 1000.0 {
                send_msg!(dividers, pdm_tables, counter, enabled);
            } else {
                send_msg!(dividers, pdm_tables, counter, false);
            }
        }
        display_freq(display, counter, enabled);

        match select(enc.wait_for(), button.wait_for_low()).await {
            Either::First(direction) => match direction {
                EncoderDirection::Up => counter = (counter + 1) % (SUBTONES.len() - 1),
                EncoderDirection::Down => {
                    if counter == 0 {
                        counter = SUBTONES.len() - 1;
                    } else {
                        counter -= 1;
                    }
                }
            },
            Either::Second(_) => {
                info!("Button pressed");
                Timer::after(DELAY_DEFAULT).await;
                match with_timeout(Duration::from_millis(750), button.wait_for_high()).await {
                    Ok(_) => {
                        #[cfg(not(feature = "call-tone"))]
                        {
                            enabled = !enabled
                        }
                        #[cfg(feature = "call-tone")]
                        {
                            if SUBTONES[counter] < 1000.0 {
                                enabled = !enabled
                            } else {
                                // send the tone for just a second
                                CHARS[Font::Ftone as usize]
                                    .translate(POS[0])
                                    .draw(display)
                                    .unwrap();
                                display.flush().unwrap();
                                send_msg!(dividers, pdm_tables, counter, true);
                                Timer::after(Duration::from_millis(1000)).await;
                                send_msg!(dividers, pdm_tables, counter, false);
                            }
                        }
                    }
                    Err(_) => {
                        CHARS[Font::Fmem as usize]
                            .translate(POS[if SUBTONES[counter] < 1000.0 { 3 } else { 0 }])
                            .draw(display)
                            .unwrap();
                        // flashing struggles if pdm is disabled?
                        send_msg!(dividers, pdm_tables, counter, true);
                        write_config(&mut flash, Config { counter, enabled });
                        send_msg!(dividers, pdm_tables, counter, enabled);
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
    info!("Hello from core 1");

    bind_interrupts!(struct Irqs {
        PIO0_IRQ_0 => InterruptHandler<PIO0>;
    });

    let Pio {
        mut common,
        sm0: mut sm,
        ..
    } = Pio::new(pio_0, Irqs);
    let out_pin = common.make_pio_pin(pdm_pin);
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
    cfg.shift_out.auto_fill = true;
    cfg.shift_out.direction = ShiftDirection::Left;
    let mut dma_out_ref = dma_0.into_ref();

    let Message {
        divider,
        mut pdm_table,
        enabled,
    } = CHANNEL.recv().await;

    cfg.clock_divider = divider;
    sm.set_config(&cfg);
    sm.set_enable(enabled);

    loop {
        match select(
            CHANNEL.recv(),
            sm.tx().dma_push(dma_out_ref.reborrow(), pdm_table),
        )
        .await
        {
            Either::First(Message {
                divider,
                pdm_table: pdm,
                enabled,
            }) => {
                sm.set_enable(false);
                cfg.clock_divider = divider;
                sm.set_config(&cfg);
                sm.set_enable(enabled);
                pdm_table = pdm;
            }
            Either::Second(_) => (),
        }
    }
}
