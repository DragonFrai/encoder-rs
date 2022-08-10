use embedded_hal::digital::v2::InputPin;
use fugit::MillisDurationU32;
use crate::rotary::{Rotary, RotaryError, Rotation, TimeRotary};
use crate::button::{Button, TimeButton};
use crate::{button, Clock, Instant};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum EncoderAction {
    None,
    Press,
    Held,
    Click,
    Rotate(Rotation),
    RotatePressed(Rotation),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TimeEncoderAction {
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
pub struct Encoder<A, B, K, const ROTATION_DIVIDER: i8> where A: InputPin, B: InputPin, K: InputPin {
    rotary: Rotary<A, B>,
    button: Button<K, true>,
    rotated_on_hold: bool,
}

impl<A, B, K, const ROTATION_DIVIDER: i8> Encoder<A, B, K, ROTATION_DIVIDER>
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

    pub fn update(&mut self) -> Result<EncoderAction, EncoderError<A::Error, B::Error, K::Error>> {
        let rotation = self.rotary.update()?;
        let btn_action = self.button.update()?;

        let act = match (self.rotated_on_hold, rotation.is_zero(), btn_action) {
            (false, false, button::ButtonAction::None) => EncoderAction::Rotate(rotation),
            (false, true, button::ButtonAction::None) => EncoderAction::None,
            (true, false, button::ButtonAction::None) => {
                self.rotated_on_hold = false;
                EncoderAction::None
            },
            (true, true, button::ButtonAction::None) => {
                self.rotated_on_hold = false;
                EncoderAction::None
            },

            (false, false, button::ButtonAction::Press) => {
                self.rotated_on_hold = true;
                EncoderAction::RotatePressed(rotation)
            },
            (false, true, button::ButtonAction::Press) => EncoderAction::Press,
            (true, false, button::ButtonAction::Press) => {
                EncoderAction::RotatePressed(rotation)
            },
            (true, true, button::ButtonAction::Press) =>
                EncoderAction::None,

            (false, false, button::ButtonAction::Held) => {
                self.rotated_on_hold = true;
                EncoderAction::RotatePressed(rotation)
            },
            (false, true, button::ButtonAction::Held) => EncoderAction::Held,
            (true, false, button::ButtonAction::Held) => {
                EncoderAction::RotatePressed(rotation)
            },
            (true, true, button::ButtonAction::Held) => EncoderAction::None,

            (false, false, button::ButtonAction::Click) => EncoderAction::Click,
            (false, true, button::ButtonAction::Click) => EncoderAction::Click,
            (true, false, button::ButtonAction::Click) => {
                self.rotated_on_hold = false;
                EncoderAction::None
            },
            (true, true, button::ButtonAction::Click) => {
                self.rotated_on_hold = false;
                EncoderAction::None
            },
        };

        Ok(act)
    }
}


// Энкодер с кнопкой
pub struct TimeEncoder<A, B, K, T, const ROTATION_DIVIDER: i8> where A: InputPin, B: InputPin, K: InputPin, T: Instant {
    rotary: TimeRotary<A, B, T, ROTATION_DIVIDER>,
    button: TimeButton<K, T, true>,
    rotated_on_hold: bool,
}

impl<A, B, K, T, const ROTATION_DIVIDER: i8> TimeEncoder<A, B, K, T, ROTATION_DIVIDER>
    where
        A: InputPin,
        B: InputPin,
        K: InputPin,
        T: Instant,
{
    pub fn new(a_pin: A, b_pin: B, k_pin: K) -> Self {
        let rotary = TimeRotary::new(a_pin, b_pin);
        let button = TimeButton::new(k_pin);
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

    pub fn update(&mut self, now: T) -> Result<TimeEncoderAction, EncoderError<A::Error, B::Error, K::Error>> {
        let rotation = self.rotary.update(now)?;
        let btn_action = self.button.update(now)?;

        let act = match (self.rotated_on_hold, rotation.is_zero(), btn_action) {
            (false, false, button::TimeButtonAction::None) => TimeEncoderAction::Rotate(rotation),
            (false, true, button::TimeButtonAction::None) => TimeEncoderAction::None,
            (true, false, button::TimeButtonAction::None) => {
                self.rotated_on_hold = false;
                TimeEncoderAction::None
            },
            (true, true, button::TimeButtonAction::None) => {
                self.rotated_on_hold = false;
                TimeEncoderAction::None
            },

            (false, false, button::TimeButtonAction::Press) => {
                self.rotated_on_hold = true;
                TimeEncoderAction::RotatePressed(rotation)
            },
            (false, true, button::TimeButtonAction::Press) => TimeEncoderAction::Press,
            (true, false, button::TimeButtonAction::Press) => TimeEncoderAction::RotatePressed(rotation),
            (true, true, button::TimeButtonAction::Press) => TimeEncoderAction::None,

            (false, false, button::TimeButtonAction::Held(_t)) => {
                self.rotated_on_hold = true;
                TimeEncoderAction::RotatePressed(rotation)
            },
            (false, true, button::TimeButtonAction::Held(t)) => TimeEncoderAction::Held(t),
            (true, false, button::TimeButtonAction::Held(t)) => {
                TimeEncoderAction::RotatePressed(rotation)
            },
            (true, true, button::TimeButtonAction::Held(_)) => TimeEncoderAction::None,

            (false, false, button::TimeButtonAction::Click(t)) => TimeEncoderAction::Click(t),
            (false, true, button::TimeButtonAction::Click(t)) => TimeEncoderAction::Click(t),
            (true, false, button::TimeButtonAction::Click(_)) => {
                self.rotated_on_hold = false;
                TimeEncoderAction::None
            },
            (true, true, button::TimeButtonAction::Click(_)) => {
                self.rotated_on_hold = false;
                TimeEncoderAction::None
            },
        };

        Ok(act)
    }
}

// Энкодер с кнопкой
pub struct ClockEncoder<A, B, K, C, const ROTATION_DIVIDER: i8> where A: InputPin, B: InputPin, K: InputPin, C: Clock {
    encoder: TimeEncoder<A, B, K, C::Instant, ROTATION_DIVIDER>,
    clock: C,
}

impl<A, B, K, C, const ROTATION_DIVIDER: i8> ClockEncoder<A, B, K, C, ROTATION_DIVIDER>
    where
        A: InputPin,
        B: InputPin,
        K: InputPin,
        C: Clock,
{
    pub fn new(a_pin: A, b_pin: B, k_pin: K, clock: C) -> Self {
        Self { encoder: TimeEncoder::new(a_pin, b_pin, k_pin), clock }
    }

    pub fn handle_press(&mut self) {
        self.encoder.handle_press()
    }

    pub fn update(&mut self) -> Result<TimeEncoderAction, EncoderError<A::Error, B::Error, K::Error>> {
        self.encoder.update(self.clock.now())
    }
}
