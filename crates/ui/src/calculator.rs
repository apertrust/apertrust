// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2013-2023  darktable developers (ported from src/common/calculator.c)
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

//! darktable's calculator (`src/common/calculator.c`, `dt_calculator_solve`): the
//! expressions a slider's value field and its popup take, evaluated by the client so that
//! the session gets numbers (decision 42; `docs/gui.md`, The calculator).
//!
//! [`solve`] is darktable's recursive descent over doubles, its arithmetic restated
//! operation for operation, with two decided differences: numbers are decimal only, and a
//! character darktable's popup refuses makes the text no value where darktable's tokenizer
//! skips it. [`without_unit`] drops the unit a value is shown with, so "25%" and "+1 EV"
//! are values (deviation 4).
//!
//! Like [`crate::i18n`], this module has no egui in it.

/// The closed list of units a slider's value can show, in English: what
/// [`without_unit`] is given with the language's own words, its `unit-<unit>` messages.
pub const UNITS: [&str; 8] = ["EV", "%", "°", "K", "ISO", "px", "mired", "mm"];

/// Brackets and unary signs nested deeper than this make no value, rather than overflow
/// the stack on a pasted text. It is darktable's `DT_BAUHAUS_MAX_TEXT`: its popup holds
/// fewer characters, so no text it evaluates nests this deep.
const MAX_DEPTH: usize = 180;

/// `text` evaluated with `x` as the current value, as darktable's `dt_calculator_solve`
/// evaluates it, or `None` where the text is no value or the result is not finite.
///
/// From the loosest binding to the tightest, each binary level left-associative:
/// - `+` and `-`;
/// - `*`, `/`, `%` (C's `fmod`, the dividend's sign) and `:` (the larger over the
///   smaller);
/// - `^` (`pow`), so `2^3^2` is 64;
/// - unary `+` and `-`, so `-2^2` is 4 and `2^-1` is 0.5;
/// - a number, `x` or `X`, or an expression in brackets.
///
/// `++` or `--` as the first token is x + 1 or x − 1, whatever follows; elsewhere it has
/// no place, so `5--3` is no value where `5 - -3` is 8. Spaces and tabs are skipped, and
/// `~`, which darktable's popup takes and its tokenizer skips. A comma is the decimal
/// point in every language, as darktable's `g_strdelimit` makes it, and U+2212 a minus.
///
/// An empty text, a token left over and an unclosed bracket are no value. Otherwise a
/// missing operand is darktable's NaN, which its arithmetic carries: through `pow` and
/// the ratio's comparisons it can vanish, so `1^` and `:5` are 1, as in darktable.
pub fn solve(x: f64, text: &str) -> Option<f64> {
    let tokens = scan(&normalised(text)?, x);
    let result = match tokens.first() {
        Some(Token::Increment) => x + 1.0,
        Some(Token::Decrement) => x - 1.0,
        _ => {
            let mut parser = Parser {
                tokens: &tokens,
                at: 0,
                depth: 0,
                too_deep: false,
            };
            let result = parser.expression();
            if parser.at < tokens.len() || parser.too_deep {
                return None;
            }
            result
        }
    };
    result.is_finite().then_some(result)
}

/// `text` without one trailing unit of `units`, matched case-insensitively, with any
/// spaces before and after it: "25%" is "25", "+1 EV " is "+1" (deviation 4). Where two
/// units end the text, the longer goes. A text that ends with none is returned as it is.
pub fn without_unit<'a>(text: &'a str, units: &[impl AsRef<str>]) -> &'a str {
    let rest = text.trim_end();
    let cut = units
        .iter()
        .filter_map(|unit| {
            let unit = unit.as_ref();
            let length = unit.chars().count();
            let (cut, _) = rest.char_indices().nth_back(length.checked_sub(1)?)?;
            (rest[cut..].to_lowercase() == unit.to_lowercase()).then_some(cut)
        })
        .min();
    cut.map_or(text, |cut| rest[..cut].trim_end())
}

/// The characters darktable's popup takes for a slider (`_popup_key_press`), and a tab,
/// which its tokenizer skips as it does a space.
const CHARACTERS: &str = "0123456789.,%+-*Xx/:^~ ()\t";

/// `text` as darktable's grammar reads it, U+2212 a minus and a comma the point, or `None`
/// where it holds a character not in [`CHARACTERS`].
fn normalised(text: &str) -> Option<String> {
    text.chars()
        .map(|c| match c {
            '\u{2212}' => Some('-'),
            ',' => Some('.'),
            c if CHARACTERS.contains(c) => Some(c),
            _ => None,
        })
        .collect()
}

/// darktable's tokens, with its `x` already the current value.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Token {
    Number(f64),
    /// A point with no digit, which darktable's `g_ascii_strtod` reads as nothing: no rule
    /// takes it, so it is always left over.
    Point,
    Plus,
    Increment,
    Minus,
    Decrement,
    Multiply,
    Divide,
    Remainder,
    Power,
    Ratio,
    Open,
    Close,
}

/// darktable's `_get_token`, over the whole of a [`normalised`] text. A number is decimal:
/// digits with at most one point.
fn scan(text: &str, x: f64) -> Vec<Token> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        let doubled = bytes.get(at + 1) == Some(&byte);
        let token = match byte {
            b'+' if doubled => Token::Increment,
            b'+' => Token::Plus,
            b'-' if doubled => Token::Decrement,
            b'-' => Token::Minus,
            b'*' => Token::Multiply,
            b'/' => Token::Divide,
            b'%' => Token::Remainder,
            b'^' => Token::Power,
            b':' => Token::Ratio,
            b'(' => Token::Open,
            b')' => Token::Close,
            b'x' | b'X' => Token::Number(x),
            b'0'..=b'9' | b'.' => {
                let start = at;
                let digits = |at: &mut usize| {
                    while bytes.get(*at).is_some_and(u8::is_ascii_digit) {
                        *at += 1;
                    }
                };
                digits(&mut at);
                if bytes.get(at) == Some(&b'.') {
                    at += 1;
                    digits(&mut at);
                }
                tokens.push(text[start..at].parse().map_or(Token::Point, Token::Number));
                continue;
            }
            // A space, a tab or `~`.
            _ => {
                at += 1;
                continue;
            }
        };
        // `++` and `--` take two characters, every other token one.
        at += 1 + usize::from(matches!(token, Token::Increment | Token::Decrement));
        tokens.push(token);
    }
    tokens
}

/// darktable's parser: each level returns NaN for an operand it does not find, leaving
/// the token where it is, as darktable's do.
struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
    /// Brackets and unary signs open around the token at `at`.
    depth: usize,
    too_deep: bool,
}

impl Parser<'_> {
    fn token(&self) -> Option<Token> {
        self.tokens.get(self.at).copied()
    }

    /// `_parse_expression` and `_parse_additive_expression`.
    fn expression(&mut self) -> f64 {
        let mut left = self.multiplicative();
        while let Some(operator @ (Token::Plus | Token::Minus)) = self.token() {
            self.at += 1;
            let right = self.multiplicative();
            if operator == Token::Plus {
                left += right;
            } else {
                left -= right;
            }
        }
        left
    }

    /// `_parse_multiplicative_expression`.
    fn multiplicative(&mut self) -> f64 {
        let mut left = self.power();
        while let Some(
            operator @ (Token::Multiply | Token::Divide | Token::Remainder | Token::Ratio),
        ) = self.token()
        {
            self.at += 1;
            let right = self.power();
            left = match operator {
                Token::Multiply => left * right,
                Token::Divide => left / right,
                // Rust's remainder of floats is C's `fmod`.
                Token::Remainder => left % right,
                // glib's `MAX(left, right) / MIN(left, right)`, comparisons and all, so a
                // NaN goes as darktable's does.
                _ => {
                    let larger = if left > right { left } else { right };
                    let smaller = if left < right { left } else { right };
                    larger / smaller
                }
            };
        }
        left
    }

    /// `_parse_power_expression`.
    fn power(&mut self) -> f64 {
        let mut left = self.unary();
        while self.token() == Some(Token::Power) {
            self.at += 1;
            let right = self.unary();
            left = left.powf(right);
        }
        left
    }

    /// `_parse_unary_expression`.
    fn unary(&mut self) -> f64 {
        match self.token() {
            Some(Token::Minus) => {
                self.at += 1;
                -self.nested(Self::unary)
            }
            Some(Token::Plus) => {
                self.at += 1;
                self.nested(Self::unary)
            }
            _ => self.primary(),
        }
    }

    /// `_parse_primary_expression`.
    fn primary(&mut self) -> f64 {
        match self.token() {
            Some(Token::Number(number)) => {
                self.at += 1;
                number
            }
            Some(Token::Open) => {
                self.at += 1;
                let result = self.nested(Self::expression);
                if self.token() != Some(Token::Close) {
                    return f64::NAN;
                }
                self.at += 1;
                result
            }
            _ => f64::NAN,
        }
    }

    /// `parse` one level deeper, or NaN with the text marked no value past [`MAX_DEPTH`].
    fn nested(&mut self, parse: fn(&mut Self) -> f64) -> f64 {
        if self.depth == MAX_DEPTH {
            self.too_deep = true;
            return f64::NAN;
        }
        self.depth += 1;
        let result = parse(self);
        self.depth -= 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `text` solved with x = 3.
    fn at_3(text: &str) -> Option<f64> {
        solve(3.0, text)
    }

    #[test]
    fn numbers_are_decimal() {
        assert_eq!(at_3("5"), Some(5.0));
        assert_eq!(at_3("2.5"), Some(2.5));
        assert_eq!(at_3(".5"), Some(0.5));
        assert_eq!(at_3("5."), Some(5.0));
        assert_eq!(at_3("007"), Some(7.0));
        assert_eq!(at_3(" \t5 "), Some(5.0));
        // darktable's `g_ascii_strtod` reads all four; here they hold a refused character
        // or leave a token over.
        assert_eq!(at_3("0x10"), None);
        assert_eq!(at_3("1e3"), None);
        assert_eq!(at_3("1E3"), None);
        assert_eq!(at_3("inf"), None);
    }

    #[test]
    fn x_is_the_current_value() {
        assert_eq!(at_3("x"), Some(3.0));
        assert_eq!(at_3("X"), Some(3.0));
        assert_eq!(at_3("x*2"), Some(6.0));
        assert_eq!(at_3("x/2+x"), Some(4.5));
        assert_eq!(solve(-0.25, "x"), Some(-0.25));
    }

    #[test]
    fn the_levels_bind_as_darktable_s() {
        assert_eq!(at_3("1+2*3"), Some(7.0));
        assert_eq!(at_3("2*3+1"), Some(7.0));
        assert_eq!(at_3("10-4/2"), Some(8.0));
        assert_eq!(at_3("1+6:3"), Some(3.0));
        assert_eq!(at_3("1+7%4"), Some(4.0));
        assert_eq!(at_3("2+3^2"), Some(11.0));
        assert_eq!(at_3("2*3^2"), Some(18.0));
        assert_eq!(at_3("2^3*2"), Some(16.0));
        assert_eq!(at_3("(1+2)*3"), Some(9.0));
        // `*`, `/`, `%` and `:` are one level, taken left to right.
        assert_eq!(at_3("2*3%4"), Some(2.0));
        assert_eq!(at_3("7%4*2"), Some(6.0));
        assert_eq!(at_3("2*3:4"), Some(1.5));
        assert_eq!(at_3("8:2*3"), Some(12.0));
    }

    #[test]
    fn every_binary_level_is_left_associative() {
        assert_eq!(at_3("8-3-2"), Some(3.0));
        assert_eq!(at_3("8/4/2"), Some(1.0));
        assert_eq!(at_3("100%7%3"), Some(2.0));
        assert_eq!(at_3("12:3:2"), Some(2.0));
        assert_eq!(at_3("2^3^2"), Some(64.0));
    }

    #[test]
    fn the_unary_signs_bind_tighter_than_the_power() {
        assert_eq!(at_3("-2^2"), Some(4.0));
        assert_eq!(at_3("2^-1"), Some(0.5));
        assert_eq!(at_3("-x"), Some(-3.0));
        assert_eq!(at_3("+5"), Some(5.0));
        assert_eq!(at_3("+-5"), Some(-5.0));
        assert_eq!(at_3("-+5"), Some(-5.0));
        assert_eq!(at_3("- -5"), Some(5.0));
        assert_eq!(at_3("2*-3"), Some(-6.0));
        assert_eq!(at_3("2- -3"), Some(5.0));
        assert_eq!(at_3("2+-3"), Some(-1.0));
        assert_eq!(at_3("-(2+3)"), Some(-5.0));
        // A cube root of a negative is NaN in `pow`.
        assert_eq!(at_3("-8^(1/3)"), None);
    }

    #[test]
    fn a_leading_double_sign_steps_by_one_whatever_follows() {
        assert_eq!(at_3("++"), Some(4.0));
        assert_eq!(at_3("--"), Some(2.0));
        assert_eq!(at_3("  ++"), Some(4.0));
        assert_eq!(at_3("++5"), Some(4.0));
        assert_eq!(at_3("--(("), Some(2.0));
        assert_eq!(at_3("++."), Some(4.0));
        assert_eq!(at_3("+++"), Some(4.0));
        // Apart, the two are signs.
        assert_eq!(at_3("+ +5"), Some(5.0));
        // Anywhere else a double sign has no place.
        assert_eq!(at_3("5--3"), None);
        assert_eq!(at_3("5++3"), None);
        assert_eq!(at_3("-(--5)"), None);
    }

    #[test]
    fn brackets_nest_and_must_close() {
        assert_eq!(at_3("(5)"), Some(5.0));
        assert_eq!(at_3("((2))"), Some(2.0));
        assert_eq!(at_3("(2+3)*(4-1)"), Some(15.0));
        assert_eq!(at_3("(x)^2"), Some(9.0));
        assert_eq!(at_3("(5"), None);
        assert_eq!(at_3("((5)"), None);
        assert_eq!(at_3("5)"), None);
        assert_eq!(at_3("()"), None);
        assert_eq!(at_3("2(3)"), None);
    }

    #[test]
    fn the_ratio_is_the_larger_over_the_smaller() {
        assert_eq!(at_3("2:4"), Some(2.0));
        assert_eq!(at_3("4:2"), Some(2.0));
        assert_eq!(at_3("x:6"), Some(2.0));
        // For negatives the larger is nearer zero, as glib's `MAX`.
        assert_eq!(at_3("-2:-4"), Some(0.5));
        assert_eq!(at_3("-4:2"), Some(-0.5));
    }

    #[test]
    fn the_remainder_takes_the_dividend_s_sign() {
        assert_eq!(at_3("7%3"), Some(1.0));
        assert_eq!(at_3("-7%3"), Some(-1.0));
        assert_eq!(at_3("7%-3"), Some(1.0));
        assert_eq!(at_3("5.5%2"), Some(1.5));
        assert_eq!(solve(2.75, "x%1"), Some(0.75));
    }

    #[test]
    fn an_empty_text_or_a_token_left_over_is_no_value() {
        assert_eq!(at_3(""), None);
        assert_eq!(at_3("   "), None);
        assert_eq!(at_3("~"), None);
        assert_eq!(at_3("5 5"), None);
        assert_eq!(at_3("2x"), None);
        assert_eq!(at_3("1.2.3"), None);
        assert_eq!(at_3("."), None);
        assert_eq!(at_3("5+."), None);
        assert_eq!(at_3("5+"), None);
        assert_eq!(at_3("*5"), None);
        assert_eq!(at_3("%5"), None);
    }

    #[test]
    fn a_missing_operand_is_darktable_s_nan_which_pow_and_the_ratio_can_lose() {
        // `pow(1, NaN)` and `pow(NaN, 0)` are 1, and glib's `MAX(NaN, 5)` and `MIN(NaN, 5)`
        // are 5: darktable gives these 1, and so does its restatement.
        assert_eq!(at_3("1^"), Some(1.0));
        assert_eq!(at_3("^0"), Some(1.0));
        assert_eq!(at_3(":5"), Some(1.0));
        assert_eq!(at_3("()^0"), Some(1.0));
        assert_eq!(at_3("(0/0)^0"), Some(1.0));
        // The other order keeps it.
        assert_eq!(at_3("5:"), None);
        assert_eq!(at_3("2^"), None);
    }

    #[test]
    fn a_comma_is_the_point_and_u2212_a_minus() {
        assert_eq!(at_3("1,5"), Some(1.5));
        assert_eq!(at_3(",5"), Some(0.5));
        assert_eq!(at_3("2,5*2"), Some(5.0));
        assert_eq!(at_3("\u{2212}1,25"), Some(-1.25));
        assert_eq!(at_3("x\u{2212}1"), Some(2.0));
        assert_eq!(at_3("2\u{2212}\u{2212}"), None);
        assert_eq!(at_3("\u{2212}\u{2212}"), Some(2.0));
        // A comma never groups digits.
        assert_eq!(at_3("1,234.5"), None);
    }

    #[test]
    fn a_character_darktable_s_popup_refuses_makes_no_value() {
        // darktable's tokenizer skips them, so these would be 5, 5, 3 and 1000.
        assert_eq!(at_3("5abc"), None);
        assert_eq!(at_3("5 EV"), None);
        assert_eq!(at_3("1+2;"), None);
        assert_eq!(at_3("1\u{a0}000"), None);
        // Before the double sign is looked at.
        assert_eq!(at_3("++abc"), None);
        // `~` is one darktable's popup takes, and its tokenizer skips.
        assert_eq!(at_3("~5"), Some(5.0));
    }

    #[test]
    fn a_result_that_is_not_finite_is_no_value() {
        assert_eq!(at_3("5/0"), None);
        assert_eq!(at_3("-5/0"), None);
        assert_eq!(at_3("0/0"), None);
        assert_eq!(at_3("x/(x-3)"), None);
        assert_eq!(at_3("5%0"), None);
        assert_eq!(at_3("0:5"), None);
        assert_eq!(at_3("0:0"), None);
        assert_eq!(at_3("10^400"), None);
        assert_eq!(solve(f64::NAN, "x"), None);
        assert_eq!(solve(f64::INFINITY, "++"), None);
    }

    #[test]
    fn nesting_past_darktable_s_popup_is_no_value() {
        let nested = |depth: usize| format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
        assert_eq!(at_3(&nested(MAX_DEPTH)), Some(1.0));
        assert_eq!(at_3(&nested(MAX_DEPTH + 1)), None);
        assert_eq!(at_3(&format!("{}^0", nested(100_000))), None);
        assert_eq!(at_3(&format!("{}1", "- ".repeat(MAX_DEPTH))), Some(1.0));
        assert_eq!(at_3(&format!("{}1", "- ".repeat(100_000))), None);
    }

    #[test]
    fn one_trailing_unit_is_dropped() {
        assert_eq!(without_unit("25%", &UNITS), "25");
        assert_eq!(without_unit("+1 EV", &UNITS), "+1");
        assert_eq!(without_unit("1 ev", &UNITS), "1");
        assert_eq!(without_unit("45°", &UNITS), "45");
        assert_eq!(without_unit("5000K", &UNITS), "5000");
        assert_eq!(without_unit("5000 k", &UNITS), "5000");
        assert_eq!(without_unit("1600 ISO", &UNITS), "1600");
        assert_eq!(without_unit("3 PX", &UNITS), "3");
        assert_eq!(without_unit("150 mired", &UNITS), "150");
        assert_eq!(without_unit("35mm", &UNITS), "35");
        assert_eq!(without_unit("x+1 EV  ", &UNITS), "x+1");
        assert_eq!(without_unit("25\u{a0}%", &UNITS), "25");
        assert_eq!(without_unit("EV", &UNITS), "");
        // One only.
        assert_eq!(without_unit("5 EV EV", &UNITS), "5 EV");
        assert_eq!(without_unit("25%%", &UNITS), "25%");
        // A text with no unit at its end is as it was.
        assert_eq!(without_unit("7 % 3 ", &UNITS), "7 % 3 ");
        assert_eq!(without_unit("2*x", &UNITS), "2*x");
    }

    #[test]
    fn the_language_s_units_are_dropped_too() {
        let units = ["пкс", "мм", "м"];
        assert_eq!(without_unit("3 пкс", &units), "3");
        assert_eq!(without_unit("3 ПКС", &units), "3");
        // The longer of two that end the text.
        assert_eq!(without_unit("35 мм", &units), "35");
        assert_eq!(without_unit("2 м", &units), "2");
        assert_eq!(without_unit("3 px", &units), "3 px");
        let owned = vec![String::from("EV")];
        assert_eq!(without_unit("1 EV", &owned), "1");
        assert_eq!(without_unit("1", &[""]), "1");
    }

    #[test]
    fn a_value_shown_with_its_unit_solves_once_it_is_dropped() {
        let typed = |text: &str| solve(3.0, without_unit(text, &UNITS));
        assert_eq!(typed("25%"), Some(25.0));
        assert_eq!(typed("+1 EV"), Some(1.0));
        assert_eq!(typed("\u{2212}0,5 EV"), Some(-0.5));
        assert_eq!(typed("x+10%"), Some(13.0));
        assert_eq!(typed("7 % 2"), Some(1.0));
        assert_eq!(typed("5 EV EV"), None);
    }
}
