use numerical_value::*;
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add,Sub,Mul,Div,Rem,Neg};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum BoundedValue<T> {
    Min,
    Raw(T),
    Max,
}

impl<T> BoundedValue<T> {
    pub fn map<U, F>(self, f: F) -> BoundedValue<U> where F: FnOnce(T) -> U {
        use self::BoundedValue::*;
        match self {
            Min => Min,
            Raw(t) => Raw(f(t)),
            Max => Max,
        }
    }
}

impl<T> BoundedValue<T> where T: MinMax {
    pub fn unwrap(self) -> T {
        use self::BoundedValue::*;
        match self {
            Min => T::min_value(),
            Raw(t) => t,
            Max => T::max_value(),
        }
    }
}

impl Add for BoundedValue<i64> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        use self::BoundedValue::*;
        match (self, other) {
            (Min, Min) => Min,
            (Min, Raw(_)) => Min,
            (Min, Max) => unimplemented!(),
            (Max, Min) => unimplemented!(),
            (Max, Raw(_)) => Max,
            (Max, Max) => Max,
            (Raw(a), Min) => Min,
            (Raw(a), Raw(b)) => Raw(a + b),
            (Raw(a), Max) => Max,
        }
    }
}

impl Mul for BoundedValue<i64> {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        use self::BoundedValue::*;
        match (self, other) {
            (Min, Min) => Min,
            (Min, Raw(b)) => {
                if b > 0 { Min }
                else if b == 0 { Raw(b) }
                else { Max }
            },
            (Min, Max) => unimplemented!(),
            (Max, Min) => unimplemented!(),
            (Max, Raw(b)) => {
                if b > 0 { Max }
                else if b == 0 { Raw(b) }
                else { Min }
            },
            (Max, Max) => Max,
            (Raw(a), Min) => {
                if a > 0 { Min }
                else if a == 0 { Raw(a) }
                else { Max }
            },
            (Raw(a), Raw(b)) => Raw(a * b),
            (Raw(a), Max) => {
                if a > 0 { Max }
                else if a == 0 { Raw(a) }
                else { Min }
            },
        }
    }
}

impl Div for BoundedValue<i64> {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        use self::BoundedValue::*;
        match (self, other) {
            (Min, Min) => Max,
            (Min, Raw(b)) => {
                if b > 0 { Min }
                else if b == 0 { unimplemented!() }
                else { Max }
            },
            (Min, Max) => Min,
            (Max, Min) => Min,
            (Max, Raw(b)) => {
                if b > 0 { Max }
                else if b == 0 { unimplemented!() }
                else { Min }
            },
            (Max, Max) => Max,
            (Raw(a), Min) => Raw(0),
            (Raw(a), Raw(b)) => Raw(a / b),
            (Raw(a), Max) => Raw(0),
        }
    }
}

impl Neg for BoundedValue<i64> {
    type Output = Self;
    fn neg(self) -> Self {
        use self::BoundedValue::*;
        match self {
            Min => Max,
            Raw(t) => Raw(-t),
            Max => Min,
        }
    }
}

impl BoundedValue<i64> {
    pub fn abs(self) -> Self {
        use self::BoundedValue::*;
        match self {
            Min => Max,
            Raw(t) => Raw(t.abs()),
            Max => Max,
        }
    }
}

impl<T> fmt::Debug for BoundedValue<T> where T: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::BoundedValue::*;
        match self {
            Min => write!(f, "-inf"),
            Raw(t) => write!(f, "{:?}", t),
            Max => write!(f, "inf"),
        }
    }
}

impl<T> Ord for BoundedValue<T> where T: Ord {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<T> PartialOrd for BoundedValue<T> where T: PartialOrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use self::BoundedValue::*;
        match (self, other) {
            (Min, Min) => Some(Ordering::Equal),
            (Min, _) => Some(Ordering::Less),
            (_, Min) => Some(Ordering::Greater),
            (Max, Max) => Some(Ordering::Equal),
            (Max, _) => Some(Ordering::Greater),
            (_, Max) => Some(Ordering::Less),
            (Raw(a), Raw(b)) => a.partial_cmp(b),
        }
    }
}

impl<T> PartialEq<T> for BoundedValue<T> where T: PartialEq {
    fn eq(&self, other: &T) -> bool {
        match self {
            BoundedValue::Raw(s) => s == other,
            _ => false,
        }
    }
}
impl<T> PartialOrd<T> for BoundedValue<T> where T: PartialOrd {
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        match self {
            BoundedValue::Min => Some(Ordering::Less),
            BoundedValue::Raw(s) => s.partial_cmp(other),
            BoundedValue::Max => Some(Ordering::Greater),
        }
    }
}

impl<T> From<T> for BoundedValue<T> {
    fn from(t: T) -> Self { BoundedValue::Raw(t) }
}

impl<T> MinMax for BoundedValue<T> {
    fn min_value() -> Self { BoundedValue::Min }
    fn max_value() -> Self { BoundedValue::Max }
}

impl Add for Range<BoundedValue<i64>> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        use Inclusivity::*;
        Range {
            min: match (self.min.inclusivity, other.min.inclusivity) {
                (Inclusive, Inclusive) => MinPair {
                    value: self.min.value.add(other.min.value),
                    inclusivity: Inclusive,
                },
                (Exclusive, Exclusive) => MinPair {
                    value: self.min.value.add(other.min.value.map(|r| r + 1)),
                    inclusivity: Exclusive,
                },
                (_, _) => MinPair {
                    value: self.min.value.add(other.min.value),
                    inclusivity: Exclusive,
                },
            },
            max: match (self.max.inclusivity, other.max.inclusivity) {
                (Inclusive, Inclusive) => MaxPair {
                    value: self.max.value.add(other.max.value),
                    inclusivity: Inclusive,
                },
                (Exclusive, Exclusive) => MaxPair {
                    value: self.max.value.add(other.max.value.map(|r| r - 1)),
                    inclusivity: Exclusive,
                },
                (_, _) => MaxPair {
                    value: self.max.value.add(other.max.value),
                    inclusivity: Exclusive,
                },
            },
        }
    }
}

impl Sub for Range<BoundedValue<i64>> {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        self + Range::new(-other.max.value, other.max.inclusivity,
                          -other.min.value, other.min.inclusivity)
    }
}

impl Mul for Range<BoundedValue<i64>> {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        use Inclusivity::*;
        Range {
            min: match (self.min.inclusivity, other.min.inclusivity) {
                (Inclusive, Inclusive) => MinPair {
                    value: self.min.value.mul(other.min.value),
                    inclusivity: Inclusive,
                },
                (Exclusive, Exclusive) => MinPair {
                    value: self.min.value.mul(other.min.value.map(|r| r + 1)),
                    inclusivity: Exclusive,
                },
                (_, _) => MinPair {
                    value: self.min.value.mul(other.min.value),
                    inclusivity: Exclusive,
                },
            },
            max: match (self.max.inclusivity, other.max.inclusivity) {
                (Inclusive, Inclusive) => MaxPair {
                    value: self.max.value.mul(other.max.value),
                    inclusivity: Inclusive,
                },
                (Exclusive, Exclusive) => MaxPair {
                    value: self.max.value.mul(other.max.value.map(|r| r - 1)),
                    inclusivity: Exclusive,
                },
                (_, _) => MaxPair {
                    value: self.max.value.mul(other.max.value),
                    inclusivity: Exclusive,
                },
            },
        }
    }
}

impl Div for Range<BoundedValue<i64>> {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        use Inclusivity::*;
        if other.min == other.max {
            Range::new(self.min.value.div(other.min.value), self.min.inclusivity,
                       self.max.value.div(other.max.value), self.max.inclusivity)
        } else {
            let biggest_negative; // most negative (ie -inf)
            let smallest_negative; // least negative (ie -1)
            let smallest_positive; // least positive (ie 1)
            let biggest_positive; // most positive (ie inf)
            if other.min.value < 0 {
                biggest_negative = Some(other.min.value);
                if other.max.value >= -1 {
                    smallest_negative = Some(BoundedValue::Raw(-1));
                } else {
                    smallest_negative = Some(other.max.value);
                }
            } else {
                biggest_negative = None;
                smallest_negative = None;
            }
            if other.max.value > 0 {
                biggest_positive = Some(other.max.value);
                if other.min.value <= 1 {
                    smallest_positive = Some(1.into());
                } else {
                    smallest_positive = Some(other.min.value);
                }
            } else {
                biggest_positive = None;
                smallest_positive = None;
            }
            println!("{:?} {:?} {:?} {:?}", biggest_negative, smallest_negative, smallest_positive, biggest_positive);
            match (biggest_negative, smallest_negative, smallest_positive, biggest_positive) {
                (_, Some(n), Some(p), _) => {
                    // 4 / [-2, 4] = [-4, 4]
                    // [4, 8] / [-2, 4] = [-8, 8]
                    // [-8, -4] / [-2, 4] = [-8, 8]
                    // [-8, 4] / [-2, 4] = [-8, 8]
                    // [-4, 8] / [-2, 4] = [-8, 8]
                    if self.min.value > 0 && self.max.value > 0 {
                        Range::new(self.max.value.div(n), Inclusive, self.max.value.div(p), Inclusive)
                    } else if self.max.value < 0 && self.max.value < 0 {
                        Range::new(self.min.value.div(p), Inclusive, self.min.value.div(n), Inclusive)
                    } else if self.min.value.abs() < self.max.value {
                        Range::new(self.max.value.div(n), Inclusive, self.max.value.div(p), Inclusive)
                    } else {
                        Range::new(self.min.value.div(p), Inclusive, self.min.value.div(n), Inclusive)
                    }
                },
                (Some(b), Some(s), _, _) =>
                // 4 / [-4, -2] = [-2, -1]
                // [4, 8] / [-4, -2] = [-4, -1]
                    Range::new(self.max.value.div(s), Inclusive, self.min.value.div(b), Inclusive),
                (_, _, Some(s), Some(b)) =>
                // 4 / [2, 4] = [1, 2]
                // [4, 8] / [2, 4] = [1, 4]
                // 32 / [4, inf] = [0, 8]
                    Range::new(self.min.value.div(b), Inclusive, self.max.value.div(s), Inclusive),
                (None, None, None, None) => unimplemented!(),
                _ => unreachable!(),
            }
        }
    }
}

impl Rem for Range<BoundedValue<i64>> {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use self::BoundedValue::*;

    #[test]
    fn div_1() {
        assert_eq!(Raw(4) / Raw(2), Raw(2));
        assert_eq!(Raw(4) / Min, Raw(0));
    }
}

#[cfg(test)]
mod operator_tests {
    use super::*;
    use self::BoundedValue::*;
    use self::Inclusivity::*;

    #[test]
    fn div_1() {
        assert_eq!(
            Range::from(Raw(32)) / Range::from(Raw(4)),
            Range::from(Raw(8)));
    }

    #[test]
    fn div_2() {
        // 32 / -99999
        // 32 / -1
        // 32 / 1
        // 32 / 4
        assert_eq!(
            Range::from(Raw(32)) / Range::new(Min, Inclusive, Raw(4), Inclusive),
            Range::new(Raw(-32), Inclusive, Raw(32), Inclusive));
    }

    #[test]
    fn div_3() {
        // 32 / 1
        // 32 / 2
        // 32 / 3
        // 32 / 4
        assert_eq!(
            Range::from(Raw(32)) / Range::new(Raw(1), Inclusive, Raw(4), Inclusive),
            Range::new(Raw(8), Inclusive, Raw(32), Inclusive));
    }

    #[test]
    fn div_4() {
        // 32 / 4
        // 32 / 16
        // 32 / 32
        // 32 / inf
        assert_eq!(
            Range::from(Raw(32)) / Range::new(Raw(4), Inclusive, Max, Inclusive),
            Range::new(Raw(0), Inclusive, Raw(8), Inclusive));
    }

    #[test]
    fn div_5() {
        // -100 / 4
        // -20 / 4
        // 8 / 4
        // 32 / 4
        assert_eq!(
            Range::new(Min, Inclusive, Raw(32), Inclusive) / Range::from(Raw(4)),
            Range::new(Min, Inclusive, Raw(8), Inclusive));
    }

    #[test]
    fn div_6() {
        assert_eq!(
            Range::from(Raw(32)) / Range::new(Raw(-4), Inclusive, Max, Inclusive),
            Range::new(Raw(-32), Inclusive, Raw(32), Inclusive));
    }

    #[test]
    fn div_7() {
        assert_eq!(
            Range::from(Raw(32)) / Range::new(Raw(1), Inclusive, Max, Inclusive),
            Range::new(Raw(0), Inclusive, Raw(32), Inclusive));
    }

    #[test]
    fn div_8() {
        assert_eq!(
            Range::new(Raw(4), Inclusive, Raw(8), Inclusive) /
                Range::new(Raw(-2), Inclusive, Raw(4), Inclusive),
            Range::new(Raw(-8), Inclusive, Raw(8), Inclusive));
    }

    #[test]
    fn div_9() {
        assert_eq!(
            // 32 / [-inf, 4] = [-32, 32]
            Range::from(Raw(32)) / Range::new(Min, Inclusive, Raw(4), Inclusive),
            Range::new(Raw(-32), Inclusive, Raw(32), Inclusive));
    }

    #[test]
    fn div_10() {
        assert_eq!(
            // 32 / [4, inf] = [0, 8]
            Range::from(Raw(32)) / Range::new(Raw(4), Inclusive, Max, Inclusive),
            Range::new(Raw(0), Inclusive, Raw(8), Inclusive));
    }

    #[test]
    fn mod_1() {
        assert_eq!(
            Range::universe() % Range::new(Raw(0), Inclusive, Raw(4), Inclusive),
            Range::new(Raw(0), Inclusive, Raw(4), Inclusive));
    }
}
