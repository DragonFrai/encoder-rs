use embedded_hal::digital::v2::InputPin;
use fugit::MillisDurationU32;
use crate::rotary::{Rotary, RotaryError, Rotation};
use crate::button::Button;
use crate::button;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Action {
    None,
    Press,
    Held,
    Click,
    Rotate(Rotation),
    RotatePressed(Rotation),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TimeAction {
    None,
    Press,
    Held(MillisDurationU32),
    Click(MillisDurationU32),
    Rotate(Rotation),
    RotatePressed(Rotation),
}

// ----------------
// # EncoderError #
// ----------------

pub enum EncoderError<A, B, K>
{
    APin(A),
    BPin(B),
    KPin(K),
}

impl<A, B, K> From<RotaryError<A, B>> for EncoderError<A, B, K>
{
    fn from(re: RotaryError<A, B>) -> Self {
        match re {
            RotaryError::APin(e) => Self::APin(e),
            RotaryError::BPin(e) => Self::BPin(e),
        }
    }
}

impl<A, B, K> From<button::Error<K>> for EncoderError<A, B, K>
{
    fn from(e: button::Error<K>) -> Self {
        match e {
            button::Error::KPin(e) => Self::KPin(e),
        }
    }
}

// -----------
// # Encoder #
// -----------

// Энкодер с кнопкой
pub struct Encoder<A, B, K> where A: InputPin, B: InputPin, K: InputPin {
    rotary: Rotary<A, B>,
    button: Button<K, true>,
    rotated_on_hold: bool,
}

impl<A, B, K> Encoder<A, B, K>
where
    A: InputPin,
    B: InputPin,
    K: InputPin,
{
    pub fn new(a_pin: A, b_pin: B, k_pin: K) -> Self {
        let rotary = Rotary::new(a_pin, b_pin);
        let button = Button::new(k_pin);
        Self {
            rotary,
            button,
            rotated_on_hold: false,
        }
    }

    pub fn handle_press(&mut self) {
        self.rotated_on_hold = false;
        self.button.handle_press()
    }

    pub fn update(&mut self) -> Result<Action, EncoderError<A::Error, B::Error, K::Error>> {
        let rotation = self.rotary.update()?;
        let btn_action = self.button.update()?;

        let act = match (self.rotated_on_hold, rotation.is_zero(), btn_action) {
            (false, false, button::Action::None) => Action::Rotate(rotation),
            (false, true, button::Action::None) => Action::None,
            (true, false, button::Action::None) => unreachable!(),
            (true, true, button::Action::None) => unreachable!(),

            (false, false, button::Action::Press) => {
                self.rotated_on_hold = true;
                Action::RotatePressed(rotation)
            },
            (false, true, button::Action::Press) => Action::Press,
            (true, false, button::Action::Press) => unreachable!(),
            (true, true, button::Action::Press) => unreachable!(),

            (false, false, button::Action::Held) => {
                self.rotated_on_hold = true;
                Action::RotatePressed(rotation)
            },
            (false, true, button::Action::Held) => Action::Held,
            (true, false, button::Action::Held) => Action::None,
            (true, true, button::Action::Held) => Action::None,

            (false, false, button::Action::Click) => Action::Click,
            (false, true, button::Action::Click) => Action::Click,
            (true, false, button::Action::Click) => {
                self.rotated_on_hold = false;
                Action::Click
            },
            (true, true, button::Action::Click) => {
                self.rotated_on_hold = false;
                Action::Click
            },
        };

        Ok(act)
    }
}
