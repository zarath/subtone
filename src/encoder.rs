use defmt::info;
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{Flex, Pin, Pull};
use embassy_rp::Peripheral;
use embassy_time::{Duration, Timer};

pub enum EncoderDirection {
    Up,
    Down,
}

pub static DELAY_DEFAULT: Duration = Duration::from_millis(5);

pub struct Encoder<'d, T: Pin, V: Pin> {
    pin_a: Flex<'d, T>,
    pin_b: Flex<'d, V>,
}

impl<'d, T: Pin, V: Pin> Encoder<'d, T, V> {
    #[inline]
    pub fn new(pin_a: impl Peripheral<P = T> + 'd, pin_b: impl Peripheral<P = V> + 'd) -> Self {
        let mut pin_a = Flex::new(pin_a);
        let mut pin_b = Flex::new(pin_b);
        pin_a.set_as_input();
        pin_a.set_pull(Pull::Up);
        pin_b.set_as_input();
        pin_b.set_pull(Pull::Up);
        Self { pin_a, pin_b }
    }

    #[allow(dead_code)]
    #[inline]
    pub fn state(&self) -> (bool, bool) {
        (self.pin_a.is_high(), self.pin_b.is_high())
    }

    #[inline]
    pub async fn wait_for(&mut self) -> EncoderDirection {
        let delay = DELAY_DEFAULT;
        join(self.pin_a.wait_for_high(), self.pin_b.wait_for_high()).await;
        Timer::after(delay).await;
        match select(self.pin_a.wait_for_low(), self.pin_b.wait_for_low()).await {
            Either::First(_) => {
                Timer::after(delay).await;
                join(self.pin_a.wait_for_low(), self.pin_b.wait_for_low()).await;
                Timer::after(delay).await;
                join(self.pin_a.wait_for_high(), self.pin_b.wait_for_low()).await;
                Timer::after(delay).await;
                join(self.pin_a.wait_for_high(), self.pin_b.wait_for_high()).await;
                info!("Up");
                EncoderDirection::Up
            }
            Either::Second(_) => {
                Timer::after(delay).await;
                join(self.pin_a.wait_for_low(), self.pin_b.wait_for_low()).await;
                Timer::after(delay).await;
                join(self.pin_a.wait_for_low(), self.pin_b.wait_for_high()).await;
                Timer::after(delay).await;
                join(self.pin_a.wait_for_high(), self.pin_b.wait_for_high()).await;
                info!("Down");
                EncoderDirection::Down
            }
        }
    }
}
