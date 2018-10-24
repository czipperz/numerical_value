use std::collections::BTreeSet;
use std::cmp::Ordering;
use std::fmt;
use std::clone::Clone;

#[derive(PartialEq, Eq, Clone)]
pub struct NumericalValue<T> {
    ranges: BTreeSet<Range<T>>,
}

#[derive(PartialEq, Eq, Clone)]
struct Range<T> {
    min: MinPair<T>,
    max: MaxPair<T>,
}

#[derive(PartialEq, Eq, Clone)]
struct MinPair<T> {
    value: T,
    inclusivity: Inclusivity,
}

#[derive(PartialEq, Eq, Clone)]
struct MaxPair<T> {
    value: T,
    inclusivity: Inclusivity,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Inclusivity {
    Inclusive, Exclusive,
}

impl<T: Ord> NumericalValue<T> {
    pub fn new() -> Self {
        NumericalValue { ranges: BTreeSet::new() }
    }

    pub fn new_value(min_v: T, min_i: Inclusivity, max_v: T, max_i: Inclusivity) -> Self {
        let mut v = NumericalValue::new();
        v.ranges.insert(Range {
            min: MinPair {
                value: min_v,
                inclusivity: min_i,
            },
            max: MaxPair {
                value: max_v,
                inclusivity: max_i,
            },
        });
        v
    }
}
impl<T> NumericalValue<T> where T: Ord, T: Clone {
    pub fn union(&self, other: &Self) -> Self {
        let mut other = other.ranges.iter().fuse();
        let mut working_other = None;
        let mut new_ranges = BTreeSet::new();
        for r in self.ranges.iter() {
            if working_other.is_none() {
                working_other = other.next().cloned();
            }
            loop {
                match working_other.take() {
                    Some(w) => {
                        // [ ]
                        //   [ ]
                        if r.max.value == w.min.value &&
                            (r.max.inclusivity == Inclusivity::Inclusive ||
                             w.min.inclusivity == Inclusivity::Inclusive) {
                            working_other = Some(Range { min: r.min.clone(), max: w.max });
                        }
                        //   [ ]
                        // [ ]
                        else if r.min.value == w.max.value &&
                            (r.min.inclusivity == Inclusivity::Inclusive ||
                             w.max.inclusivity == Inclusivity::Inclusive) {
                            working_other = Some(Range { min: w.min, max: r.max.clone() });
                        }
                        // [ ]
                        //  [ ]
                        else if r.min <= w.min && r.max >= w.min {
                            if r.max > w.max {
                                working_other = Some(Range { min: r.min.clone(), max: r.max.clone() });
                            } else {
                                working_other = Some(Range { min: r.min.clone(), max: w.max });
                            }
                        }
                        //  [ ]
                        // [ ]
                        else if w.min <= r.min && w.max >= r.min {
                            if w.max > r.max {
                                working_other = Some(Range { min: w.min, max: w.max });
                            } else {
                                working_other = Some(Range { min: w.min, max: r.max.clone() });
                            }
                        }
                        // [ ]
                        //     [ ]
                        else if r.max < w.min {
                            new_ranges.insert(r.clone());
                            working_other = Some(w);
                        }
                        //     [ ]
                        // [ ]
                        else {
                            new_ranges.insert(w);
                            working_other = other.next().cloned();
                            continue;
                        }
                        break;
                    },
                    None => {
                        new_ranges.insert(r.clone());
                        break;
                    },
                }
            }
        }
        match working_other {
            Some(r) => {
                let w = other.next();
                match w {
                    Some(w) => {
                        // [ ]
                        //   [ ]
                        if r.max.value == w.min.value &&
                            (r.max.inclusivity == Inclusivity::Inclusive ||
                             w.min.inclusivity == Inclusivity::Inclusive) {
                            new_ranges.insert(Range { min: r.min.clone(), max: w.max.clone() });
                        }
                        // [ ]
                        //  [ ]
                        else if r.min <= w.min && r.max >= w.min {
                            if r.max > w.max {
                                new_ranges.insert(Range { min: r.min.clone(), max: r.max.clone() });
                            } else {
                                new_ranges.insert(Range { min: r.min.clone(), max: w.max.clone() });
                            }
                        } else {
                            new_ranges.insert(r.clone());
                            new_ranges.insert(w.clone());
                        }
                    },
                    None => {
                        new_ranges.insert(r.clone());
                    }
                }
            },
            None => {},
        }
        for w in other {
            new_ranges.insert(w.clone());
        }
        NumericalValue { ranges: new_ranges }
    }

    pub fn union_value(&self, min_v: T, min_i: Inclusivity, max_v: T, max_i: Inclusivity) -> Self {
        self.union(&NumericalValue::new_value(min_v, min_i, max_v, max_i))
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let mut other = other.ranges.iter().fuse();
        let mut working_other = other.next().cloned();
        let mut new_ranges = BTreeSet::<Range<T>>::new();
        for r in self.ranges.iter() {
            if working_other.is_none() {
                working_other = other.next().cloned();
            }
            loop {
                match working_other.take() {
                    Some(w) => {
                        // [  ]]]
                        //  [ ]]]
                        if r.min <= w.min && r.max >= w.min {
                            if r.max > w.max {
                                // [   ]
                                //  [ ]
                                new_ranges.insert(w);
                                working_other = other.next().cloned();
                                continue;
                            } else {
                                // [ ]
                                //  [ ]
                                new_ranges.insert(Range { min: w.min, max: r.max.clone() });
                            }
                        }
                        //  [ ]]]
                        // [  ]]]
                        else if w.min <= r.min && w.max >= r.min {
                            if w.max > r.max {
                                //  [ ]
                                // [   ]
                                new_ranges.insert(r.clone());
                                working_other = Some(w);
                            } else {
                                //  [ ]
                                // [ ]
                                new_ranges.insert(Range { min: w.min, max: r.max.clone() });
                                working_other = other.next().cloned();
                                continue;
                            }
                        }
                        // [ ]
                        //     [ ]
                        else if r.max < w.min {
                            working_other = Some(w);
                        }
                        //     [ ]
                        // [ ]
                        else {
                            working_other = other.next().cloned();
                            continue;
                        }
                    },
                    None => {},
                }
                break;
            }
        }
        NumericalValue { ranges: new_ranges }
    }

    pub fn intersect_value(&self, min_v: T, min_i: Inclusivity, max_v: T, max_i: Inclusivity) -> Self {
        self.intersect(&NumericalValue::new_value(min_v, min_i, max_v, max_i))
    }
}

pub trait MinMax<T = Self> {
    fn min_value() -> T;
    fn max_value() -> T;
}

impl MinMax for i32 {
    fn min_value() -> Self { i32::min_value() }
    fn max_value() -> Self { i32::max_value() }
}

impl<T> NumericalValue<T> where T: MinMax<T>, T: Ord, T: Clone {
    pub fn inverse(&self) -> Self {
        use Inclusivity::*;
        
        let mut last_end: MaxPair<T> = MaxPair { value: T::min_value(),
                                                 inclusivity: Exclusive };
        let mut new_ranges = BTreeSet::new();
        for r in self.ranges.iter() {
            new_ranges.insert(Range {
                min: MinPair { value: last_end.value.clone(),
                               inclusivity: last_end.inclusivity.flip() },
                max: MaxPair { value: r.min.value.clone(),
                               inclusivity: r.min.inclusivity.flip() }
            });
            last_end = r.max.clone();
        }
        let max = MaxPair { value: T::max_value(),
                            inclusivity: Inclusivity::Inclusive };
        if last_end != max {
            new_ranges.insert(Range {
                min: MinPair { value: last_end.value,
                               inclusivity: last_end.inclusivity.flip() },
                max,
            });
        }
        NumericalValue { ranges: new_ranges }
    }
}

impl Inclusivity {
    fn flip(&self) -> Self {
        use Inclusivity::*;
        match self {
            Inclusive => Exclusive,
            Exclusive => Inclusive,
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for NumericalValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for r in &self.ranges {
            if first {
                first = false;
            } else {
                try!(write!(f, " U "));
            }
            try!(write!(f, "{:?}", r));
        }
        if first {
            try!(write!(f, "(0, 0)"));
        }
        Ok(())
    }
}

impl<T: fmt::Debug> fmt::Debug for Range<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let seperator_min;
        if self.min.inclusivity == Inclusivity::Inclusive {
            seperator_min = "[";
        } else {
            seperator_min = "(";
        }
        let seperator_max;
        if self.max.inclusivity == Inclusivity::Inclusive {
            seperator_max = "]";
        } else {
            seperator_max = ")";
        }
        write!(f, "{}{:?}, {:?}{}", seperator_min, self.min.value, self.max.value, seperator_max)
    }
}

impl<T: Ord> Ord for Range<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        let min_ord = self.min.cmp(&other.min);
        if (min_ord == Ordering::Equal) {
            self.max.cmp(&other.max)
        } else {
            min_ord
        }
    }
}

impl<T: Ord> PartialOrd for Range<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> Ord for MinPair<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<T: Ord> Ord for MaxPair<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<T: PartialOrd> PartialOrd for MaxPair<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value).map(|ordering| {
            if ordering == Ordering::Equal {
                if (self.inclusivity == Inclusivity::Inclusive &&
                    other.inclusivity == Inclusivity::Exclusive) {
                    Ordering::Greater
                } else if (self.inclusivity == Inclusivity::Exclusive &&
                           other.inclusivity == Inclusivity::Inclusive) {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            } else {
                ordering
            }
        })
    }
}

impl<T: PartialOrd> PartialOrd for MinPair<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value).map(|ordering| {
            if ordering == Ordering::Equal {
                if (self.inclusivity == Inclusivity::Inclusive &&
                    other.inclusivity == Inclusivity::Exclusive) {
                    Ordering::Less
                } else if (self.inclusivity == Inclusivity::Exclusive &&
                           other.inclusivity == Inclusivity::Inclusive) {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            } else {
                ordering
            }
        })
    }
}

impl<T: PartialOrd> PartialOrd<MaxPair<T>> for MinPair<T> {
    fn partial_cmp(&self, other: &MaxPair<T>) -> Option<Ordering> {
        self.value.partial_cmp(&other.value).map(|ordering| {
            if ordering == Ordering::Equal {
                if (self.inclusivity == Inclusivity::Inclusive &&
                    other.inclusivity == Inclusivity::Inclusive) {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            } else {
                ordering
            }
        })
    }
}

impl<T: PartialOrd> PartialOrd<MinPair<T>> for MaxPair<T> {
    fn partial_cmp(&self, other: &MinPair<T>) -> Option<Ordering> {
        self.value.partial_cmp(&other.value).map(|ordering| {
            if ordering == Ordering::Equal {
                if (self.inclusivity == Inclusivity::Inclusive &&
                    other.inclusivity == Inclusivity::Inclusive) {
                    Ordering::Equal
                } else {
                    Ordering::Less
                }
            } else {
                ordering
            }
        })
    }
}

impl<T: PartialEq> PartialEq<MaxPair<T>> for MinPair<T> {
    fn eq(&self, other: &MaxPair<T>) -> bool {
        self.value == other.value && self.inclusivity == other.inclusivity
    }
}

impl<T: PartialEq> PartialEq<MinPair<T>> for MaxPair<T> {
    fn eq(&self, other: &MinPair<T>) -> bool {
        self.value == other.value && self.inclusivity == other.inclusivity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use self::Inclusivity::*;

    #[test]
    fn union_test_1() {
        let mut value = NumericalValue::new_value(-3, Inclusive, 3, Exclusive);
        assert_eq!(format!("{:?}", value), "[-3, 3)");

        value = value.union_value(-5, Exclusive, 0, Exclusive);
        assert_eq!(format!("{:?}", value), "(-5, 3)");


        value = value.union_value(-5, Inclusive, 3, Inclusive);
        assert_eq!(format!("{:?}", value), "[-5, 3]");

        assert_eq!(format!("{:?}", NumericalValue::new_value(-5, Exclusive, 0, Exclusive)
                           .union_value(-3, Inclusive, 3, Exclusive)), "(-5, 3)");
    }

    #[test]
    fn union_test_2() {
        let mut value = NumericalValue::new_value(-3, Inclusive, 3, Exclusive);
        assert_eq!(format!("{:?}", value), "[-3, 3)");

        value = value.union_value(-5, Exclusive, 0, Exclusive);
        assert_eq!(format!("{:?}", value), "(-5, 3)");
    }

    #[test]
    fn union_test_3() {
        let mut value = NumericalValue::new_value(-5, Inclusive, 5, Exclusive);
        assert_eq!(format!("{:?}", value), "[-5, 5)");

        value = value.union_value(5, Exclusive, 8, Inclusive);
        assert_eq!(format!("{:?}", value), "[-5, 5) U (5, 8]");

        assert_eq!(format!("{:?}", value.union_value(5, Inclusive, 5, Inclusive)), "[-5, 8]");
        assert_eq!(format!("{:?}", NumericalValue::new_value(5, Inclusive, 5, Inclusive).union(&value)), "[-5, 8]");
    }

    #[test]
    fn union_test_4() {
        assert_eq!(format!("{:?}", NumericalValue::new_value(-3, Exclusive, 10, Inclusive)
                           .union_value(-8, Exclusive, -6, Inclusive)),
                   "(-8, -6] U (-3, 10]");
    }

    #[test]
    fn intersect_test_1() {
        let mut value = NumericalValue::new_value(-5, Inclusive, 5, Exclusive);
        value = value.intersect(
            &NumericalValue::new_value(-3, Inclusive, -1, Inclusive)
                .union_value(2, Exclusive, 4, Inclusive));
        assert_eq!(format!("{:?}", value), "[-3, -1] U (2, 4]");

        value = NumericalValue::new_value(-3, Inclusive, -1, Inclusive)
            .union_value(2, Exclusive, 4, Inclusive);
        value = value.intersect(
            &NumericalValue::new_value(-5, Inclusive, 5, Exclusive));
        assert_eq!(format!("{:?}", value), "[-3, -1] U (2, 4]");
    }

    #[test]
    fn intersect_test_2() {
        let mut value = NumericalValue::new_value(-5, Inclusive, 5, Exclusive);
        value = value.intersect(
            &NumericalValue::new_value(-3, Exclusive, 10, Inclusive)
                .union_value(-8, Exclusive, -6, Inclusive));
        assert_eq!(format!("{:?}", value), "(-3, 5)");
    }

    #[test]
    fn inverse_test_1() {
        assert_eq!(format!("{:?}", NumericalValue::<i32>::new().inverse()), "[-2147483648, 2147483647]");
    }

    #[test]
    fn inverse_test_2() {
        let value: NumericalValue<i32> =
            NumericalValue::new_value(-7, Exclusive, -2, Exclusive)
            .union_value(1, Inclusive, 3, Inclusive);
        assert_eq!(format!("{:?}", value), "(-7, -2) U [1, 3]");
        assert_eq!(format!("{:?}", value.inverse()), "[-2147483648, -7] U [-2, 1) U (3, 2147483647]");
    }
}
