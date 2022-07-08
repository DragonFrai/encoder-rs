#![no_std]

use core::convert::Infallible;
use embedded_hal::digital::v2::InputPin;

// -------------
// # Direction #
// -------------

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Direction {
    None,
    Cw,
    Ccw,
}

// ----------------
// # RotaryError #
// ----------------

pub enum RotaryError<A, B>
where
    A: InputPin,
    B: InputPin,
{
    APin(A::Error),
    BPin(B::Error),
}

// ----------
// # Rotary #
// ----------

/// Энкодер, который не имеет кнопки и может только вращаться
pub struct Rotary<A, B> {
    a_pin: A,
    b_pin: B,
    ab_history: u8, // содержит 4 состояния пинов a и b
}

impl<A, B> Rotary<A, B>
where
    A: InputPin,
    B: InputPin,
{
    pub fn new(a_pin: A, b_pin: B) -> Self {
        Self {
            a_pin,
            b_pin,
            ab_history: 0b11111111,
        }
    }

    /// Возвращает направление вращение энкодера
    pub fn update(&mut self) -> Result<Direction, RotaryError<A, B>> {
        let a_high = self.a_pin.is_high().map_err(RotaryError::APin)?;
        let b_high = self.b_pin.is_high().map_err(RotaryError::BPin)?;

        let as_num = |b| if b { 1u8 } else { 0u8 };
        let bits_state = (as_num(a_high) << 1) | as_num(b_high);

        // Обновляем историю состояний энкодера
        if (self.ab_history & 0b11) != bits_state {
            self.ab_history = (self.ab_history << 2) | bits_state;
        }

        // match по истории с текущим состоянием
        let direction = match self.ab_history {
            0b10000111 => {
                self.ab_history = 0xFF;
                Direction::Cw
            }
            0b01001011 => {
                self.ab_history = 0xFF;
                Direction::Ccw
            }
            _ => Direction::None,
        };

        Ok(direction)
    }
}

// --------------------
// # RotaryInfallible #
// --------------------

/// Энкодер, который не имеет кнопки и может только вращаться
pub struct RotaryInfallible<A, B>(Rotary<A, B>);

impl<A, B> RotaryInfallible<A, B>
where
    A: InputPin<Error = Infallible>,
    B: InputPin<Error = Infallible>,
{
    pub fn new(a_pin: A, b_pin: B) -> Self {
        Self(Rotary::new(a_pin, b_pin))
    }

    /// Возвращает направление вращение энкодера
    pub fn update(&mut self) -> Direction {
        unsafe { self.0.update().unwrap_unchecked() }
    }
}

// ----------
// # Action #
// ----------

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Action {
    None,
    Cw,
    Ccw,
    CwPressed,
    CcwPressed,
    Click,
}

impl Action {
    pub fn direction(&self) -> Direction {
        match *self {
            Action::None | Action::Click => Direction::None,
            Action::Cw | Action::CwPressed => Direction::Cw,
            Action::Ccw | Action::CcwPressed => Direction::Ccw,
        }
    }
}

// ----------------
// # EncoderState #
// ----------------

#[derive(Copy, Clone)]
pub struct EncoderState {
    direction: Direction,
    pressed: bool,
    just_key_changed: bool,
    rotated_before_key_change: bool,
}

impl EncoderState {
    /// Вращение ротора
    #[inline(always)]
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Нажат ли ротор.
    #[inline(always)]
    pub fn pressed(&self) -> bool {
        self.pressed
    }

    /// Флаг указывающий что состояние нажатия изменилось только что.
    #[inline(always)]
    pub fn just_key_changed(&self) -> bool {
        self.just_key_changed
    }

    /// true когда ротор вращался в нажатом или отпущенном состоянии до изменения состояния.
    /// Может быть true, только когда [just_key_changed] = true.
    #[inline(always)]
    pub fn rotated_before_key_change(&self) -> bool {
        self.rotated_before_key_change
    }
}

// ----------------
// # EncoderError #
// ----------------

pub enum EncoderError<A, B, K>
where
    A: InputPin,
    B: InputPin,
    K: InputPin,
{
    APin(A::Error),
    BPin(B::Error),
    KPin(K::Error),
}

impl<A, B, K> From<RotaryError<A, B>> for EncoderError<A, B, K>
where
    A: InputPin,
    B: InputPin,
    K: InputPin,
{
    fn from(re: RotaryError<A, B>) -> Self {
        match re {
            RotaryError::APin(e) => Self::APin(e),
            RotaryError::BPin(e) => Self::BPin(e),
        }
    }
}

// -----------
// # Encoder #
// -----------

// Энкодер с кнопкой
pub struct Encoder<A, B, K> {
    rotary: Rotary<A, B>,

    k_pin: K,
    pressed: bool,
    rotated_after_key_change: bool,
}

impl<A, B, K> Encoder<A, B, K>
where
    A: InputPin,
    B: InputPin,
    K: InputPin,
{
    pub fn new(a_pin: A, b_pin: B, k_pin: K) -> Self {
        let rotary = Rotary::new(a_pin, b_pin);
        Self {
            rotary,
            k_pin,
            pressed: false,
            rotated_after_key_change: false,
        }
    }

    /// Возвращает "Сырое" представление состояния энкодера.
    /// Может быть полезно для реализации более сложного поведения чем в обычном [update]
    pub fn update_raw(&mut self) -> Result<EncoderState, EncoderError<A, B, K>> {
        let direction = self.rotary.update()?;
        let k_high = self.k_pin.is_high().map_err(EncoderError::KPin)?;

        // Получаем текущее состояние с учетом старого
        let pressed = !k_high;
        let was_pressed = self.pressed;
        let just_key_changed = pressed != was_pressed;
        let rotated_before_key_change = just_key_changed & self.rotated_after_key_change;
        let rotated_after_key_change =
            (direction != Direction::None) | (!just_key_changed & self.rotated_after_key_change);

        // Запоминаем текущее состояние
        self.pressed = pressed;
        self.rotated_after_key_change = rotated_after_key_change;
        if just_key_changed {
            let initial = if pressed { 0x00 } else { 0xFF };
            self.rotary.ab_history = initial;
        }

        // Возвращаем текущее состояние
        Ok(EncoderState {
            direction,
            pressed,
            just_key_changed,
            rotated_before_key_change,
        })
    }

    /// Возвращает одно из действий энкодера
    pub fn update(&mut self) -> Result<Action, EncoderError<A, B, K>> {
        let raw = self.update_raw()?;
        let action = match (raw.direction, raw.pressed) {
            (Direction::None, false) if raw.just_key_changed && !raw.rotated_before_key_change => {
                Action::Click
            }
            (Direction::Cw, true) => Action::CwPressed,
            (Direction::Cw, false) => Action::Cw,
            (Direction::Ccw, true) => Action::CcwPressed,
            (Direction::Ccw, false) => Action::Ccw,
            _ => Action::None,
        };
        Ok(action)
    }
}

// ---------------------
// # EncoderInfallible #
// ---------------------

pub struct EncoderInfallible<A, B, K>(Encoder<A, B, K>);

impl<A, B, K> EncoderInfallible<A, B, K>
where
    A: InputPin<Error = Infallible>,
    B: InputPin<Error = Infallible>,
    K: InputPin<Error = Infallible>,
{
    pub fn new(a_pin: A, b_pin: B, k_pin: K) -> Self {
        Self(Encoder::new(a_pin, b_pin, k_pin))
    }

    /// Возвращает "Сырое" представление состояния энкодера.
    /// Может быть полезно для реализации более сложного поведения чем в обычном [update]
    pub fn update_raw(&mut self) -> EncoderState {
        unsafe { self.0.update_raw().unwrap_unchecked() }
    }

    /// Возвращает одно из действий энкодера
    pub fn update(&mut self) -> Action {
        unsafe { self.0.update().unwrap_unchecked() }
    }
}
