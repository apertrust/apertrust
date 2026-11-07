// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Transfer functions (decision 28): how a control's value becomes an operation
//! parameter's, each with its derivative, which the optimiser needs to fit module values
//! through the kernels' gradients (principle 7). Each is a named pure function, tested:
//! monotone where its control's promise says so, and its derivative against finite
//! differences ([`check`]). That nothing is set at the default and that the ends stay
//! inside the schema is the binding's, which [`crate::check::sweep`] checks.

use std::ops::RangeInclusive;

/// A control's value to a parameter's.
pub trait Transfer {
    fn value(&self, x: f32) -> f32;
    /// d value / d x.
    fn derivative(&self, x: f32) -> f32;
}

/// `scale × x + offset`: a percentage to a fraction, or an offset over a parameter's
/// value for the photo where the parameter is in the control's own unit, as Black level
/// correction is over the photo's black. Where it is not, the transfer function inverts
/// the operation's maths instead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Linear {
    pub scale: f32,
    pub offset: f32,
}

impl Linear {
    /// `x + offset`: an offset over a parameter's value for the photo, in the parameter's
    /// own unit.
    pub const fn over(offset: f32) -> Self {
        Self { scale: 1.0, offset }
    }

    /// `scale × x`.
    pub const fn scale(scale: f32) -> Self {
        Self { scale, offset: 0.0 }
    }
}

impl Transfer for Linear {
    fn value(&self, x: f32) -> f32 {
        self.scale * x + self.offset
    }

    fn derivative(&self, _x: f32) -> f32 {
        self.scale
    }
}

/// How many points of the range [`check`] and [`increasing`] look at.
const SAMPLES: usize = 64;

fn samples(range: &RangeInclusive<f32>) -> impl Iterator<Item = f64> {
    let (start, end) = (f64::from(*range.start()), f64::from(*range.end()));
    (0..=SAMPLES).map(move |i| start + (end - start) * i as f64 / SAMPLES as f64)
}

/// Whether `transfer`'s derivative matches central finite differences over `range`,
/// within 1% of the larger of the two and 1e-4. The differences step a thousandth of the
/// range, in f64 from the f32 values, so f32 rounding does not dominate them.
pub fn check(transfer: &dyn Transfer, range: RangeInclusive<f32>) -> Result<(), String> {
    let h = (f64::from(*range.end()) - f64::from(*range.start())) / 1000.0;
    for x in samples(&range) {
        let at = |x: f64| f64::from(transfer.value(x as f32));
        let estimate = (at(x + h) - at(x - h)) / (2.0 * h);
        let derivative = f64::from(transfer.derivative(x as f32));
        let tolerance = 1e-2 * estimate.abs().max(derivative.abs()).max(1e-2);
        if (estimate - derivative).abs() > tolerance {
            return Err(format!(
                "at {x}: the derivative is {derivative}, finite differences give {estimate}"
            ));
        }
    }
    Ok(())
}

/// Whether `transfer` rises across `range`, never falling.
pub fn increasing(transfer: &dyn Transfer, range: RangeInclusive<f32>) -> bool {
    let values: Vec<f32> = samples(&range).map(|x| transfer.value(x as f32)).collect();
    values.windows(2).all(|pair| pair[0] <= pair[1]) && values[0] < values[SAMPLES]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2^x: a curved one, so the check sees a derivative that changes.
    struct Exp2;

    impl Transfer for Exp2 {
        fn value(&self, x: f32) -> f32 {
            x.exp2()
        }

        fn derivative(&self, x: f32) -> f32 {
            std::f32::consts::LN_2 * x.exp2()
        }
    }

    #[test]
    fn linear_has_its_derivative_and_rises() {
        for transfer in [Linear::over(0.7), Linear::scale(0.01), Linear::scale(2.5)] {
            check(&transfer, -3.0..=4.0).unwrap();
            assert!(increasing(&transfer, -3.0..=4.0));
        }
        assert_eq!(Linear::over(0.7).value(0.5), 1.2);
        assert!(!increasing(&Linear::scale(-1.0), 0.0..=1.0));
    }

    #[test]
    fn the_check_finds_a_wrong_derivative() {
        check(&Exp2, -4.0..=4.0).unwrap();
        struct Wrong;
        impl Transfer for Wrong {
            fn value(&self, x: f32) -> f32 {
                x.exp2()
            }
            fn derivative(&self, x: f32) -> f32 {
                x.exp2()
            }
        }
        assert!(check(&Wrong, -4.0..=4.0).is_err());
    }
}
