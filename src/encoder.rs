use defmt::info;
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::Input;
use embassy_time::{Duration, Timer};

pub enum EncoderDirection {
    Up,
    Down,
}

pub static DELAY_DEFAULT: Duration = Duration::from_millis(5);

pub struct Encoder<'d> {
    pin_a: Input<'d>,
    pin_b: Input<'d>,
}

impl<'d> Encoder<'d> {
    #[inline]
    pub fn new(pin_a: Input<'static>, pin_b: Input<'static>) -> Self {
        Self { pin_a, pin_b }
    }

    #[allow(dead_code)]
    #[inline]
    pub fn state(&mut self) -> (bool, bool) {
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
