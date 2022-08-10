use fugit::{ExtU32, MillisDurationU32};

pub trait Instant: Copy {
    fn duration_since(self, other: Self) -> MillisDurationU32;
    fn zero() -> Self;
}

pub trait Clock {
    type Instant: Instant;
    fn now(&mut self) -> Self::Instant;
}

#[derive(Copy, Clone)]
pub struct ZeroInstant;
impl Instant for ZeroInstant {
    #[inline(always)]
    fn duration_since(self, _: Self) -> MillisDurationU32 {
        0.millis()
    }
    #[inline(always)]
    fn zero() -> Self { ZeroInstant }
}

pub struct ZeroClock;
impl Clock for ZeroClock {
    type Instant = ZeroInstant;

    #[inline(always)]
    fn now(&mut self) -> Self::Instant {
        ZeroInstant
    }
}

impl<F, T> Clock for F
where
    F: FnMut() -> T,
    T: Instant,
{
    type Instant = T;

    fn now(&mut self) -> Self::Instant {
        self()
    }
}

impl<const NOM: u32, const DENOM: u32> Instant for fugit::Instant<u32, NOM, DENOM> {
    #[inline]
    fn duration_since(self, other: Self) -> MillisDurationU32 {
        let dur = self - other;
        dur.to_millis().millis()
    }

    #[inline(always)]
    fn zero() -> Self {
        Self::from_ticks(0)
    }
}

impl<const NOM: u32, const DENOM: u32> Instant for fugit::Instant<u64, NOM, DENOM> {
    #[inline]
    fn duration_since(self, other: Self) -> MillisDurationU32 {
        let dur = self - other;
        let millis = dur.to_millis();
        if millis <= u32::MAX as u64 {
            millis as u32
        } else {
            u32::MAX
        }.millis()
    }

    #[inline(always)]
    fn zero() -> Self {
        Self::from_ticks(0)
    }
}
