use crate::time::{Clock, Instant};
use embedded_hal::digital::v2::InputPin;

const SINGLE_ROTATION_MS: u32 = 250;
const LIMITED_ROTATION_MS: u32 = 50;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Direction {
    None,
    Cw,
    Ccw,
}

impl Direction {
    pub fn to_rotation(self) -> Rotation {
        match self {
            Direction::None => Rotation(0),
            Direction::Cw => Rotation(1),
            Direction::Ccw => Rotation(-1),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(transparent)]
pub struct Rotation(i32);

impl Rotation {
    pub fn new(angle: i32) -> Self {
        Self(angle)
    }

    #[inline]
    pub fn direction(self) -> Direction {
        match self.0 {
            0 => Direction::None,
            1..=i32::MAX => Direction::Cw,
            i32::MIN..=-1 => Direction::Ccw,
        }
    }

    #[inline(always)]
    pub fn angle(self) -> i32 {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }
}

pub enum RotaryError<A, B> {
    APin(A),
    BPin(B),
}

// TODO: Use const generic of enum
///
pub struct Rotary<A, B, const ROTATION_DIVIDER: i8 = 4> {
    a_pin: A,
    b_pin: B,
    state: u8,
    switches: i8,
}

impl<A, B, const ROTATION_DIVIDER: i8> Rotary<A, B, ROTATION_DIVIDER>
where
    A: InputPin,
    B: InputPin,
{
    pub fn new(a_pin: A, b_pin: B) -> Self {
        Self {
            a_pin,
            b_pin,
            state: 0,
            switches: 0,
        }
    }

    pub fn update(&mut self) -> Result<Rotation, RotaryError<A::Error, B::Error>> {
        let a_low = self.a_pin.is_low().map_err(RotaryError::APin)?;
        let b_low = self.b_pin.is_low().map_err(RotaryError::BPin)?;

        let mut ab_state = (self.state & 0x0F) >> 2;
        if a_low {
            ab_state |= 0b1000;
        }
        if b_low {
            ab_state |= 0b0100;
        }
        self.state = ab_state;

        let switch_rot = match ab_state {
            0b0001 | 0b0111 | 0b1000 | 0b1110 => 1i8,
            0b0010 | 0b0100 | 0b1011 | 0b1101 => -1i8,
            _ => 0i8,
        };
        let switches = self.switches + switch_rot;

        let rotation = if switches.abs() == ROTATION_DIVIDER {
            self.switches = 0;
            Rotation(switches.signum() as i32)
        } else {
            self.switches = switches;
            Rotation(0)
        };

        Ok(rotation)
    }
}

pub struct TimeRotary<A, B, T, const ROTATION_DIVIDER: i8 = 4> where T: Instant {
    rotary: Rotary<A, B, ROTATION_DIVIDER>,
    last_rot_at: Option<T>,
    acceleration: u16,
}

impl<A, B, T, const ROTATION_DIVIDER: i8> TimeRotary<A, B, T, ROTATION_DIVIDER>
    where
        A: InputPin,
        B: InputPin,
        T: Instant,
{
    pub fn set_acceleration(&mut self, acceleration: u16) {
        self.acceleration = acceleration;
    }

    pub fn new(a_pin: A, b_pin: B) -> Self {
        Self::with_acceleration(a_pin, b_pin, 1)
    }

    pub fn with_acceleration(a_pin: A, b_pin: B, acceleration: u16) -> Self {
        Self {
            rotary: Rotary::new(a_pin, b_pin),
            last_rot_at: None,
            acceleration,
        }
    }

    pub fn update(&mut self, now: T) -> Result<Rotation, RotaryError<A::Error, B::Error>> {
        let rot = self.rotary.update()?;
        match rot {
            Rotation(0) => Ok(rot),
            Rotation(base) => match self.last_rot_at.replace(now) {
                None => Ok(Rotation(base)),
                Some(last) => {
                    let dt = now.duration_since(last);
                    match dt {
                        dt if dt <= LIMITED_ROTATION_MS => Ok(Rotation(base * self.acceleration as i32)),
                        dt if dt >= SINGLE_ROTATION_MS => Ok(Rotation(base)), // handle 0 acceleraton?
                        _ => {
                            let low_plus_dt = dt - LIMITED_ROTATION_MS;
                            let size = SINGLE_ROTATION_MS - LIMITED_ROTATION_MS;
                            let acc = self.acceleration as u32;
                            let rot = acc - (acc * low_plus_dt / size);
                            Ok(Rotation(base * rot as i32))
                        }
                    }
                }
            },
        }
    }
}

pub struct ClockRotary<A, B, C, const ROTATION_DIVIDER: i8 = 4>
    where
        A: InputPin,
        B: InputPin,
        C: Clock,
{
    rotary: TimeRotary<A, B, C::Instant, ROTATION_DIVIDER>,
    clock: C,
}

impl<A, B, C, const ROTATION_DIVIDER: i8> ClockRotary<A, B, C, ROTATION_DIVIDER>
    where
        A: InputPin,
        B: InputPin,
        C: Clock,
{
    pub fn set_acceleration(&mut self, acceleration: u16) {
        self.rotary.set_acceleration(acceleration);
    }

    pub fn new(a_pin: A, b_pin: B, clock: C) -> Self {
        Self::with_acceleration(a_pin, b_pin, clock, 1)
    }

    pub fn with_acceleration(a_pin: A, b_pin: B, clock: C, acceleration: u16) -> Self {
        Self {
            rotary: TimeRotary::with_acceleration(a_pin, b_pin, acceleration),
            clock,
        }
    }

    pub fn update(&mut self) -> Result<Rotation, RotaryError<A::Error, B::Error>> {
        self.rotary.update(self.clock.now())
    }
}
