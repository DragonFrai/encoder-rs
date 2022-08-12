use crate::{Clock, Instant};
use embedded_hal::digital::v2::InputPin;
use fugit::MillisDurationU32;

#[derive(Copy, Clone, Debug)]
pub enum ButtonAction {
    None,
    Press,
    Held,
    Click,
}

#[derive(Copy, Clone, Debug)]
pub enum TimeButtonAction {
    None,
    Press,
    Held(MillisDurationU32),
    Click(MillisDurationU32),
}

pub enum Error<K> {
    KPin(K),
}

#[inline]
fn update_state(state: &mut u8, pressed: bool) -> u8 {
    let s = match pressed {
        true => (*state >> 1) | 0b10,
        false => *state >> 1,
    };
    *state = s;
    s
}

pub struct Button<K, const INVERTED: bool = false>
where
    K: InputPin,
{
    k_pin: K,
    state: u8,
    handle_press: bool,
}

impl<K, const INVERTED: bool> Button<K, INVERTED>
where
    K: InputPin,
{
    pub fn new(k_pin: K) -> Self {
        Self {
            k_pin,
            state: 0u8,
            handle_press: false,
        }
    }

    pub fn handle_press(&mut self) {
        if matches!(self.state, 0b10 | 0b11) {
            self.handle_press = true;
        }
    }

    pub fn update(&mut self) -> Result<ButtonAction, Error<K::Error>> {
        let pressed = self.k_pin.is_high().map_err(Error::KPin)? ^ INVERTED;
        let s = update_state(&mut self.state, pressed);
        let r = match s {
            0b01 if self.handle_press => {
                self.handle_press = false;
                ButtonAction::None
            }
            0b11 if self.handle_press => ButtonAction::None,
            0b00 => ButtonAction::None,
            0b01 => ButtonAction::Click,
            0b10 => ButtonAction::Press,
            0b11 => ButtonAction::Held,
            _ => unreachable!(),
        };
        Ok(r)
    }
}

pub struct TimeButton<K, T, const INVERTED: bool = false>
where
    K: InputPin,
    T: Instant,
{
    button: Button<K, INVERTED>,
    press_at: T, // none when press handled
}

impl<K, T: Instant, const INVERTED: bool> TimeButton<K, T, INVERTED>
where
    K: InputPin,
{
    pub fn new(k_pin: K) -> Self {
        Self {
            button: Button::new(k_pin),
            press_at: T::zero(),
        }
    }

    pub fn handle_press(&mut self) {
        self.button.handle_press()
    }

    pub fn update(&mut self, now: T) -> Result<TimeButtonAction, Error<K::Error>> {
        let act = self.button.update()?;
        let act = match act {
            ButtonAction::None => TimeButtonAction::None,
            ButtonAction::Press => {
                self.press_at = now;
                TimeButtonAction::Press
            }
            ButtonAction::Held => TimeButtonAction::Held(now.duration_since(self.press_at)),
            ButtonAction::Click => TimeButtonAction::Click(now.duration_since(self.press_at)),
        };
        Ok(act)
    }
}

pub struct ClockButton<K, C, const INVERTED: bool = false>
where
    K: InputPin,
    C: Clock,
{
    button: TimeButton<K, C::Instant, INVERTED>,
    clock: C,
}

impl<K, C, const INVERTED: bool> ClockButton<K, C, INVERTED>
where
    K: InputPin,
    C: Clock,
{
    pub fn new(k_pin: K, clock: C) -> Self {
        Self {
            button: TimeButton::new(k_pin),
            clock,
        }
    }

    pub fn handle_press(&mut self) {
        self.button.handle_press()
    }

    pub fn update(&mut self) -> Result<TimeButtonAction, Error<K::Error>> {
        self.button.update(self.clock.now())
    }
}
