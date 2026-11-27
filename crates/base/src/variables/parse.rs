// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2025  darktable developers (ported from src/common/variables.c)
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

//! A pattern read as darktable's `_expand_source`, `_variable_get_value` and
//! `_get_base_value` read it as they expand, into parts expanded later, refusing what
//! darktable reads past (D8): a name it does not know, a variable it cannot close.

use super::names::{self, Takes};
use super::{Operation, Params, Part, PatternError, Replace, Variable};

/// Reads `text` whole.
pub(super) fn pattern(text: &str) -> Result<Vec<Part>, PatternError> {
    let mut parser = Parser {
        text,
        at: 0,
        depth: 0,
    };
    let parts = parser.parts(None)?;
    debug_assert_eq!(parser.at, text.len());
    Ok(parts)
}

/// How deep variables nest in one another: deeper is refused (D8), far beyond any
/// pattern a person writes.
pub(super) const MAX_DEPTH: usize = 64;

struct Parser<'a> {
    text: &'a str,
    /// The byte the reader is at.
    at: usize,
    /// How many variables the reader is inside.
    depth: usize,
}

/// A byte of a name: darktable's names are letters, digits, dots and underscores.
fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'_'
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.text[self.at..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// Text and variables up to `stop` or the end, `stop` left unread: `_expand_source`.
    /// A backslash takes the character after it as it is, a `$(` starts a variable.
    fn parts(&mut self, stop: Option<char>) -> Result<Vec<Part>, PatternError> {
        let mut parts = Vec::new();
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if Some(c) == stop {
                break;
            }
            if c == '\\'
                && let Some(next) = self.rest()[1..].chars().next()
            {
                text.push(next);
                self.at += 1 + next.len_utf8();
                continue;
            }
            if self.rest().starts_with("$(") {
                if !text.is_empty() {
                    parts.push(Part::Text(std::mem::take(&mut text)));
                }
                parts.push(Part::Variable(Box::new(self.variable()?)));
                continue;
            }
            text.push(c);
            self.at += c.len_utf8();
        }
        if !text.is_empty() {
            parts.push(Part::Text(text));
        }
        Ok(parts)
    }

    /// A variable inside at most [`MAX_DEPTH`] others, so that no pattern reads, expands or
    /// drops deeper than the stack takes.
    fn variable(&mut self) -> Result<Variable, PatternError> {
        if self.depth == MAX_DEPTH {
            return Err(PatternError::TooDeep { at: self.at });
        }
        self.depth += 1;
        let variable = self.one_variable();
        self.depth -= 1;
        variable
    }

    /// A variable, `$(` read: its name, its parameters, an operation and its `)`.
    fn one_variable(&mut self) -> Result<Variable, PatternError> {
        let start = self.at;
        self.at += 2;
        let name_start = self.at;
        let run = self.rest().bytes().take_while(|b| is_name_byte(*b)).count();
        let typed = &self.text[name_start..name_start + run];
        let unknown = || PatternError::Unknown {
            name: typed.to_owned(),
            at: name_start..name_start + run,
            suggestion: names::nearest(typed),
        };
        let Some((name, key)) = names::name_at(self.rest()) else {
            return Err(unknown());
        };
        self.at += name.len();
        // The name's characters past the name: one digit for darktable's older forms of
        // SEQUENCE and CATEGORY, otherwise a longer name it does not know.
        let digit = match &typed.as_bytes()[name.len()..] {
            [] => None,
            [d @ b'0'..=b'9'] if matches!(key.takes(), Takes::Sequence | Takes::Category) => {
                self.at += 1;
                Some(d & 0b1111)
            }
            _ => return Err(unknown()),
        };
        let params = match (key.takes(), digit) {
            (Takes::Nothing, _) => Params::None,
            (Takes::Category, Some(level)) => {
                // darktable's `$(CATEGORYn(category))`: the category up to the next ")".
                if self.peek() == Some('(') {
                    let Some(length) = self.rest()[1..].find(')') else {
                        return Err(self.unclosed(start, ')'));
                    };
                    self.at += length + 2;
                }
                Params::Digit(level)
            }
            (_, Some(digit)) => Params::Digit(digit),
            (_, None) => self.brackets(start)?,
        };
        let operation = self.operation(start, name)?;
        if self.peek() != Some(')') {
            return Err(self.unclosed(start, ')'));
        }
        self.at += 1;
        Ok(Variable {
            key,
            name,
            params,
            operation,
        })
    }

    /// `[n]` or `[n,m]` after a name: `_get_parameters_n_m`. Like darktable, a `[`
    /// followed by a `,` takes nothing more, so the `,` is the operation.
    fn brackets(&mut self, start: usize) -> Result<Params, PatternError> {
        if self.peek() != Some('[') {
            return Ok(Params::None);
        }
        self.at += 1;
        if self.peek() == Some(',') {
            return Ok(Params::None);
        }
        // Closed by the first "]", which must come before the variable's ")".
        let close = self.rest().find(']');
        let paren = self.rest().find(')');
        let Some(close) = close.filter(|close| paren.is_none_or(|paren| *close < paren)) else {
            return Err(self.unclosed(start, ']'));
        };
        let inside = &self.rest()[..close];
        // `g_strsplit` of empty text gives no part, so `[]` sets nothing.
        let params = match inside.split_once(',') {
            _ if inside.is_empty() => Params::None,
            Some((first, second)) => Params::Brackets(first.to_owned(), Some(second.to_owned())),
            None => Params::Brackets(inside.to_owned(), None),
        };
        self.at += close + 1;
        Ok(params)
    }

    /// The operation after a name and its parameters, if any: `_variable_get_value`'s
    /// switch, darktable's bash-like string manipulation.
    fn operation(&mut self, start: usize, name: &str) -> Result<Option<Operation>, PatternError> {
        let Some(c) = self.peek() else {
            return Err(self.unclosed(start, ')'));
        };
        if c == ')' {
            return Ok(None);
        }
        self.at += c.len_utf8();
        let paren = Some(')');
        Ok(Some(match c {
            '-' => Operation::Default(self.parts(paren)?),
            '+' => Operation::Alternative(self.parts(paren)?),
            ':' => {
                let offset = self.strtol();
                let length = if self.peek() == Some(':') {
                    self.at += 1;
                    // darktable reads the length into an int.
                    Some(i64::from(self.strtol() as i32))
                } else {
                    None
                };
                Operation::Substring { offset, length }
            }
            '#' => Operation::RemovePrefix(self.parts(paren)?),
            '%' => Operation::RemoveSuffix(self.parts(paren)?),
            '/' => {
                let replace = match self.peek() {
                    Some('/') => Replace::All,
                    Some('#') => Replace::Prefix,
                    Some('%') => Replace::Suffix,
                    _ => Replace::First,
                };
                if replace != Replace::First {
                    self.at += 1;
                }
                let pattern = self.parts(Some('/'))?;
                if self.peek() != Some('/') {
                    return Err(self.unclosed(start, '/'));
                }
                self.at += 1;
                let replacement = self.parts(paren)?;
                Operation::Replace {
                    replace,
                    pattern,
                    replacement,
                }
            }
            '^' | ',' => {
                let all = self.peek() == Some(c);
                if all {
                    self.at += 1;
                }
                if c == '^' {
                    Operation::Upper { all }
                } else {
                    Operation::Lower { all }
                }
            }
            found => {
                return Err(PatternError::NoOperation {
                    variable: format!("$({name}"),
                    found,
                    at: self.at - found.len_utf8(),
                });
            }
        }))
    }

    /// C's `strtol` in base 10 over what follows, as darktable reads an offset or a
    /// length: spaces, a sign and digits, or 0 reading nothing; saturating.
    fn strtol(&mut self) -> i64 {
        let (value, read) = strtol(self.rest());
        self.at += read;
        value
    }

    fn unclosed(&self, start: usize, expected: char) -> PatternError {
        PatternError::Unclosed {
            variable: self.text[start..self.at].to_owned(),
            expected,
            at: self.at,
        }
    }
}

/// C's `strtol(text, &end, 10)` and glib's `g_ascii_strtoll(text, NULL, 10)`: the value
/// of the decimal number `text` starts with, after white space, with its sign, saturating
/// at the type's ends, and how many bytes it takes; `(0, 0)` where no digit comes.
pub(super) fn strtol(text: &str) -> (i64, usize) {
    let bytes = text.as_bytes();
    let mut at = bytes
        .iter()
        .take_while(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r'))
        .count();
    let negative = bytes.get(at) == Some(&b'-');
    if matches!(bytes.get(at), Some(b'+' | b'-')) {
        at += 1;
    }
    let digits = bytes[at..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if digits == 0 {
        return (0, 0);
    }
    let value = bytes[at..at + digits].iter().fold(0_i64, |value, digit| {
        let digit = i64::from(digit - b'0');
        if negative {
            value.saturating_mul(10).saturating_sub(digit)
        } else {
            value.saturating_mul(10).saturating_add(digit)
        }
    });
    (value, at + digits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_numbers_as_c_does() {
        assert_eq!(strtol("5"), (5, 1));
        assert_eq!(strtol("-7:-2"), (-7, 2));
        assert_eq!(strtol("  +3x"), (3, 4));
        assert_eq!(strtol(")"), (0, 0));
        assert_eq!(strtol(" -)"), (0, 0));
        assert_eq!(strtol("99999999999999999999"), (i64::MAX, 20));
    }
}
