use crate::time::{Clock, Instant};
use embedded_hal::digital::v2::InputPin;

const SINGLE_ROTATION_MS: u32 = 100;
const LIMITED_ROTATION_MS: u32 = 20;

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
            1..=i32::MAX => Direction::Ccw,
            i32::MIN..=-1 => Direction::Cw,
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

        let state = self.state >> 2 | match (a_low, b_low) {
            (false, false) => 0b0000,
            (false, true) => 0b0100,
            (true, false) => 0b1000,
            (true, true) => 0b1100,
        };
        self.state = state;

        let overflow_switches = |switch_origin: &mut i8, switches: i8| {
            if switches.abs() >= ROTATION_DIVIDER {
                *switch_origin = 0;
                Rotation(switches.signum() as i32)
            } else {
                *switch_origin = switches;
                Rotation(0)
            }
        };

        let rot = match state {
            0b0001 | 0b0111 | 0b1110 | 0b1000 | 0b0110 => {
                let switches = self.switches - 1;
                overflow_switches(&mut self.switches, switches)
            },
            0b0010 | 0b1011 | 0b1101 | 0b0100 | 0b1001 => {
                let switches = self.switches + 1;
                overflow_switches(&mut self.switches, switches)
            },
            0b0000 | 0b0011 => {
                let s = self.switches;
                self.switches = 0;
                Rotation(s.signum() as i32)
            }
            _ => Rotation(0),
        };
        Ok(rot)
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
                    match dt.to_millis() {
                        dt if dt <= LIMITED_ROTATION_MS => Ok(Rotation(base * self.acceleration as i32)),
                        dt if dt >= SINGLE_ROTATION_MS => Ok(Rotation(base)), // handle 0 acceleraton?
                        dt => {
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
