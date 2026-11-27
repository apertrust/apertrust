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

//! Translation: the interface's words in British and American English, Ukrainian and
//! Polish, with their plurals, and numbers in each language's format (`docs/gui.md`,
//! Translation; decision 36).
//!
//! Messages are Fluent, in `locales/<locale>/*.ftl`, compiled in. A message missing from
//! a translation falls back to British English, and one missing from British English
//! shows its id. American English is an overlay: its files hold only the messages whose
//! words it spells differently, over British English's.
//!
//! This module has no egui in it, so it can move unchanged into a crate of its own,
//! `apertrust-i18n`, once the CLI or the server needs translated text. Its egui side is
//! [`crate::set_locale`] and [`crate::tr`].

use std::borrow::Cow;
use std::sync::OnceLock;

use apertrust_base::settings::Choice;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentResource, FluentValue};
use icu_calendar::Gregorian;
use icu_datetime::FixedCalendarDateTimeFormatter;
use icu_datetime::fieldsets;
use icu_datetime::input::{Date, DateTime, Time};
use icu_datetime::options::YearStyle;
use icu_decimal::DecimalFormatter;
use icu_decimal::input::Decimal;
use icu_decimal::options::{DecimalFormatterOptions, GroupingStrategy};
use unic_langid::LanguageIdentifier;

pub use fluent_bundle::FluentArgs as Args;

/// `value` for a message's variable, written in the language's numbers with at least
/// `at_least` decimals, as a size's "1.0 MB" and "1,5 МБ".
pub fn decimals(value: f64, at_least: usize) -> FluentValue<'static> {
    let options = fluent_bundle::types::FluentNumberOptions {
        minimum_fraction_digits: Some(at_least),
        ..Default::default()
    };
    FluentValue::Number(fluent_bundle::types::FluentNumber::new(value, options))
}

/// The languages of the interface, and the pseudo-locale that tests layouts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Locale {
    /// British English, the source.
    #[default]
    EnGb,
    /// American English: British English's messages with American spelling, its files an
    /// overlay of the messages whose words differ.
    EnUs,
    Uk,
    Pl,
    /// English made longer and accented, in brackets: `[Çöpý éďíť~~~~]`. It shows clipped
    /// text and strings that escaped translation.
    Pseudo,
}

impl Locale {
    pub const ALL: [Locale; 5] = [
        Locale::EnGb,
        Locale::EnUs,
        Locale::Uk,
        Locale::Pl,
        Locale::Pseudo,
    ];

    /// The locale's identifier, as settings store it ([`Choice`]).
    pub const fn id(self) -> &'static str {
        match self {
            Locale::EnGb => "en-GB",
            Locale::EnUs => "en-US",
            Locale::Uk => "uk",
            Locale::Pl => "pl",
            Locale::Pseudo => "en-XA",
        }
    }

    /// The language's name in itself, for a language menu; never translated.
    pub fn native_name(self) -> &'static str {
        match self {
            Locale::EnGb => "English (UK)",
            Locale::EnUs => "English (US)",
            Locale::Uk => "Українська",
            Locale::Pl => "Polski",
            Locale::Pseudo => "Pseudo-locale",
        }
    }

    pub fn from_id(id: &str) -> Option<Locale> {
        Locale::ALL
            .into_iter()
            .find(|l| l.id().eq_ignore_ascii_case(id))
    }

    /// The best locale for the user's languages, most wanted first (`["uk-UA", "en-US"]`):
    /// the first whose language Apertrust has, or British English. English in the United
    /// States (`en-US`, `en_US`) becomes American English, every other English British
    /// English. The pseudo-locale is chosen only by its own id.
    pub fn negotiate<'a>(wanted: impl IntoIterator<Item = &'a str>) -> Locale {
        for tag in wanted {
            if let Some(exact) = Locale::from_id(tag) {
                return exact;
            }
            let mut subtags = tag.split(['-', '_']);
            let language = subtags.next().unwrap_or_default();
            let american = subtags.any(|subtag| subtag.eq_ignore_ascii_case("us"));
            match language.to_ascii_lowercase().as_str() {
                "en" if american => return Locale::EnUs,
                "en" => return Locale::EnGb,
                "uk" => return Locale::Uk,
                "pl" => return Locale::Pl,
                _ => {}
            }
        }
        Locale::EnGb
    }

    /// The language whose messages, plurals and numbers this locale uses.
    fn language(self) -> &'static str {
        match self {
            Locale::Pseudo => "en-GB",
            other => other.id(),
        }
    }

    fn sources(self) -> &'static [&'static str] {
        match self {
            // The interface's own words, then the tools' (decision 28).
            Locale::EnGb | Locale::Pseudo => &[
                include_str!("../locales/en-GB/ui.ftl"),
                include_str!("../locales/en-GB/tools.ftl"),
            ],
            // Only the messages American English spells differently; `bundle` adds British
            // English's for the rest.
            Locale::EnUs => &[
                include_str!("../locales/en-US/ui.ftl"),
                include_str!("../locales/en-US/tools.ftl"),
            ],
            Locale::Uk => &[
                include_str!("../locales/uk/ui.ftl"),
                include_str!("../locales/uk/tools.ftl"),
            ],
            Locale::Pl => &[
                include_str!("../locales/pl/ui.ftl"),
                include_str!("../locales/pl/tools.ftl"),
            ],
        }
    }

    /// The position of the locale's number formatters in [`FORMATTERS`].
    fn numbers(self) -> usize {
        match self {
            Locale::EnGb | Locale::Pseudo => 0,
            Locale::Uk => 1,
            Locale::Pl => 2,
            Locale::EnUs => 3,
        }
    }
}

/// The language setting's options (decision 43), by their BCP 47 tags.
impl Choice for Locale {
    const OPTIONS: &'static [(Locale, &'static str)] = &[
        (Locale::EnGb, Locale::EnGb.id()),
        (Locale::EnUs, Locale::EnUs.id()),
        (Locale::Uk, Locale::Uk.id()),
        (Locale::Pl, Locale::Pl.id()),
        (Locale::Pseudo, Locale::Pseudo.id()),
    ];

    /// A tag in any case, as BCP 47 has them: `UK` is `uk`.
    fn from_id(id: &str) -> Option<Locale> {
        Locale::from_id(id)
    }
}

/// Messages and numbers in one locale.
pub struct Translator {
    locale: Locale,
    bundle: FluentBundle<FluentResource>,
    /// British English, for messages a translation lacks. American English has none: its
    /// bundle holds British English's messages under its own.
    english: Option<FluentBundle<FluentResource>>,
}

impl Translator {
    pub fn new(locale: Locale) -> Translator {
        Translator {
            locale,
            bundle: bundle(locale),
            english: matches!(locale, Locale::Uk | Locale::Pl).then(|| bundle(Locale::EnGb)),
        }
    }

    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// Whether this locale's own messages have `id`, without falling back to English.
    /// American English's own are British English's with its overlay.
    pub fn has(&self, id: &str) -> bool {
        self.bundle.has_message(id)
    }

    /// Whether this locale's own message `id` has the attribute `name`, without falling
    /// back to English, as [`has`](Self::has) does for messages.
    pub fn has_attribute(&self, id: &str, name: &str) -> bool {
        (self.bundle.get_message(id)).is_some_and(|message| message.get_attribute(name).is_some())
    }

    /// The message `id`, translated.
    pub fn text(&self, id: &str) -> String {
        self.text_with(id, None)
    }

    /// The message `id`, translated, with its variables. Numbers among them are written
    /// in the locale's format and choose its plural form.
    pub fn text_with(&self, id: &str, args: Option<&Args>) -> String {
        let text = format(&self.bundle, id, args)
            .or_else(|| self.english.as_ref().and_then(|b| format(b, id, args)))
            .unwrap_or_else(|| id.to_owned());
        if self.locale == Locale::Pseudo {
            lengthen(&text)
        } else {
            text
        }
    }

    /// The attribute `name` of the message `id`, translated, as `tool-exposure-title`'s
    /// `.search`; a translation lacking it falls back to British English. None where
    /// neither has it. American English's overlay keeps an attribute only where it writes
    /// it again with the message.
    pub fn attribute(&self, id: &str, name: &str) -> Option<String> {
        let text = format_attribute(&self.bundle, id, name)
            .or_else(|| (self.english.as_ref()).and_then(|b| format_attribute(b, id, name)))?;
        Some(if self.locale == Locale::Pseudo {
            lengthen(&text)
        } else {
            text
        })
    }

    /// `value` with `decimals` decimal places in the locale's format, with no digit
    /// grouping, as value fields show it: `−1.25` in English, `−1,25` in Ukrainian and
    /// Polish. A negative value takes the minus sign (U+2212), which has the digits'
    /// width. With `signed`, as C's `%+` writes a slider whose range spans zero in
    /// darktable, a positive one takes `+`, and so does zero, a value that rounds to it
    /// included: `+0.00`, where darktable prints a tiny negative as `−0.00` (decision 42,
    /// its deviation 13). Without it, zero takes no sign.
    pub fn format_number(&self, value: f64, decimals: usize, signed: bool) -> String {
        signed_number(self.locale, value, decimals, signed, false)
    }

    /// `at`, milliseconds since 1970 in UTC, on the local clock as the locale writes a
    /// date and a time to the minute, through CLDR's short date with its year whole:
    /// "21/10/2026, 17:52" in British English, "10/21/2026, 5:52 PM" in American,
    /// "21.10.2026, 17:52" in Ukrainian and Polish. Empty for a time the local clock
    /// cannot place.
    pub fn format_date_time(&self, at: u64) -> String {
        date_time(self.locale, at).unwrap_or_default()
    }

    /// `at`, a date and a time as a clock showed it, the camera's, in the locale's medium
    /// date with its weekday and the time to the second, 24- or 12-hour as the language has
    /// it, through CLDR as [`Self::format_date_time`] is, then the clock's `offset` from UTC
    /// in minutes where it is known: "Sat, 4 Jul 2026, 08:10:01 (+01:00)" in British English.
    /// Empty for a time the calendar cannot place.
    pub fn format_long_date_time(&self, at: chrono::NaiveDateTime, offset: Option<i16>) -> String {
        let Some(text) = to_second(self.locale, at, true) else {
            return String::new();
        };
        match offset {
            Some(minutes) => {
                let sign = if minutes < 0 { '−' } else { '+' };
                let minutes = minutes.unsigned_abs();
                format!("{text} ({sign}{:02}:{:02})", minutes / 60, minutes % 60)
            }
            None => text,
        }
    }

    /// `at` as [`Self::format_long_date_time`] writes it, without its weekday or an offset:
    /// "4 Jul 2026, 08:10:01".
    pub fn format_date_time_to_second(&self, at: chrono::NaiveDateTime) -> String {
        to_second(self.locale, at, false).unwrap_or_default()
    }

    /// `date` in the locale's medium date, with its year or without: "4 Jul 2026", "4 Jul"
    /// in British English. Empty for a date the calendar cannot place.
    pub fn format_date(&self, date: chrono::NaiveDate, year: bool) -> String {
        medium_date(self.locale, date, year).unwrap_or_default()
    }

    /// `date` in the locale's long date with its weekday, as screen readers say it:
    /// "Saturday, 4 July 2026" in British English. Empty for a date the calendar cannot place.
    pub fn format_spoken_date(&self, date: chrono::NaiveDate) -> String {
        long_date(self.locale, date).unwrap_or_default()
    }

    /// `at`'s time to the minute, as the locale writes it after a medium date: "08:10" in
    /// British English, "8:10 AM" in American. Empty for a time the calendar cannot place.
    pub fn format_time(&self, at: chrono::NaiveDateTime) -> String {
        to_minute(self.locale, at).unwrap_or_default()
    }

    /// A size on disk, `size` with `decimals` decimals in `unit`, `kb`, `mb` or `gb` of 10³,
    /// 10⁶ and 10⁹ bytes, as the engine's `size_in_unit` gives it: "32 MB", "4.5 MB", in the
    /// language's numbers, the words Settings › Storage and a cache's Clear write a size in.
    pub fn format_bytes(&self, size: f64, decimals: usize, unit: &str) -> String {
        let mut args = Args::new();
        args.set("size", self::decimals(size, decimals));
        args.set("unit", unit);
        self.text_with("job-cache-size", Some(&args))
    }

    /// The local calendar day `day` ([`local_day`]) as the locale writes a date with no
    /// time, CLDR's long month and day: "12 November" in English, "12 листопада" in
    /// Ukrainian, "12 listopada" in Polish; with its year where it is not `today`'s year's:
    /// "12 November 2025". Empty for a day the calendar cannot place.
    pub fn format_day(&self, day: i32, today: i32) -> String {
        calendar_day(self.locale, day, today).unwrap_or_default()
    }

    /// The locale's decimal separator: `.` in English, `,` in Ukrainian and Polish.
    pub fn decimal_separator(&self) -> char {
        decimal_separator(self.locale)
    }

    /// A number typed in a value field. It accepts the locale's decimal separator and the
    /// point, ignores digit grouping, takes `−` or `-` for minus, and stops at the first
    /// character that belongs to no number, so a unit may follow (`1,5 EV`).
    pub fn parse_number(&self, text: &str) -> Option<f64> {
        let separator = decimal_separator(self.locale);
        let mut number = String::new();
        for c in text.trim().chars() {
            match c {
                '0'..='9' | '.' => number.push(c),
                '−' | '-' => number.push('-'),
                '+' | ' ' | '\u{a0}' | '\u{202f}' | '\'' => {}
                c if c == separator => number.push('.'),
                // A comma that is not the decimal separator groups digits.
                ',' => {}
                _ => break,
            }
        }
        number.parse::<f64>().ok().filter(|v| v.is_finite())
    }
}

fn bundle(locale: Locale) -> FluentBundle<FluentResource> {
    let language: LanguageIdentifier = locale
        .language()
        .parse()
        .expect("the locales' ids are valid; tested");
    let mut bundle = FluentBundle::new_concurrent(vec![language]);
    // egui lays text out left to right only, and would draw the invisible marks Fluent
    // puts around variables for right-to-left text.
    bundle.set_use_isolating(false);
    bundle.set_formatter(Some(match locale.numbers() {
        1 => number_in::<1, _>,
        2 => number_in::<2, _>,
        3 => number_in::<3, _>,
        _ => number_in::<0, _>,
    }));
    if locale == Locale::Pseudo {
        bundle.set_transform(Some(accent));
    }
    // American English's overlay comes first, then British English's messages for the words
    // it spells alike: a duplicate keeps the first.
    let under: &[&str] = match locale {
        Locale::EnUs => Locale::EnGb.sources(),
        _ => &[],
    };
    for source in locale.sources().iter().chain(under) {
        let resource =
            FluentResource::try_new((*source).to_owned()).unwrap_or_else(|(resource, _)| resource);
        // A duplicate id keeps the first message; the tests reject duplicates.
        let _ = bundle.add_resource(resource);
    }
    bundle
}

fn format(bundle: &FluentBundle<FluentResource>, id: &str, args: Option<&Args>) -> Option<String> {
    let pattern = bundle.get_message(id)?.value()?;
    let mut errors = Vec::new();
    Some(
        bundle
            .format_pattern(pattern, args, &mut errors)
            .into_owned(),
    )
}

fn format_attribute(bundle: &FluentBundle<FluentResource>, id: &str, name: &str) -> Option<String> {
    let pattern = bundle.get_message(id)?.get_attribute(name)?.value();
    let mut errors = Vec::new();
    Some(
        bundle
            .format_pattern(pattern, None, &mut errors)
            .into_owned(),
    )
}

/// `text` as a search compares it, in any of the interface's languages: lower case, the
/// accents taken off Latin letters, `ł` read as `l`, so "Ł", "ł" and "l", "É" and "e" are
/// one, and an apostrophe however it is typed, `ʼ` and `’` read as `'`. Cyrillic letters
/// are kept whole: `й`, `ї` and `ё` are letters of their own, not `и`, `і` and `е` with a
/// mark, however they were typed. Unicode's canonical decomposition splits a Latin letter
/// from its marks, which are then dropped.
pub fn fold(text: &str) -> String {
    let nfd = icu_normalizer::DecomposingNormalizerBorrowed::new_nfd();
    // Composed first, so an и typed with its breve apart is й.
    let text = icu_normalizer::ComposingNormalizerBorrowed::new_nfc().normalize(text);
    let mut folded = String::with_capacity(text.len());
    let mut one = [0; 4];
    for c in text.chars().flat_map(char::to_lowercase) {
        match c {
            'ł' => folded.push('l'),
            '\u{2bc}' | '\u{2019}' => folded.push('\''),
            '\u{400}'..='\u{4ff}' => folded.push(c),
            _ => folded.extend(
                nfd.normalize(c.encode_utf8(&mut one))
                    .chars()
                    // The combining marks Latin letters carry: accents, diaeresis,
                    // cedillas and ogoneks.
                    .filter(|c| !('\u{0300}'..='\u{036f}').contains(c)),
            ),
        }
    }
    folded
}

/// Fluent's formatter for numbers in messages, for the locale whose formatters sit at
/// `L` in [`FORMATTERS`]. Messages group digits unless the number asks otherwise.
fn number_in<const L: usize, M>(value: &FluentValue, _: &M) -> Option<String> {
    let FluentValue::Number(number) = value else {
        return None;
    };
    let locale = [Locale::EnGb, Locale::Uk, Locale::Pl, Locale::EnUs][L];
    let decimals = number
        .options
        .minimum_fraction_digits
        .unwrap_or(0)
        .max(natural_decimals(number.value));
    Some(signed_number(
        locale,
        number.value,
        decimals,
        false,
        number.options.use_grouping,
    ))
}

/// The decimal places a value needs, up to six: 1.5 needs one, 2 none.
fn natural_decimals(value: f64) -> usize {
    let text = format!("{:.6}", value.abs());
    let trimmed = text.trim_end_matches('0');
    trimmed.split('.').nth(1).map_or(0, str::len)
}

fn signed_number(
    locale: Locale,
    value: f64,
    decimals: usize,
    signed: bool,
    grouping: bool,
) -> String {
    let digits = format!("{:.*}", decimals, value.abs());
    let zero = !digits.bytes().any(|b| matches!(b, b'1'..=b'9'));
    let body = Decimal::try_from_str(&digits)
        .map(|d| formatter(locale, grouping).format_to_string(&d))
        .unwrap_or(digits);
    let sign = if signed && (zero || value > 0.0) {
        "+"
    } else if !zero && value < 0.0 {
        "−"
    } else {
        ""
    };
    format!("{sign}{body}")
}

/// The CLDR number formatters for British English, Ukrainian, Polish and American English,
/// without and with digit grouping, made when first used.
static FORMATTERS: [[OnceLock<DecimalFormatter>; 2]; 4] = [
    [OnceLock::new(), OnceLock::new()],
    [OnceLock::new(), OnceLock::new()],
    [OnceLock::new(), OnceLock::new()],
    [OnceLock::new(), OnceLock::new()],
];

fn formatter(locale: Locale, grouping: bool) -> &'static DecimalFormatter {
    FORMATTERS[locale.numbers()][usize::from(grouping)].get_or_init(|| {
        let language: icu_locale_core::Locale = locale
            .language()
            .parse()
            .expect("the locales' ids are valid; tested");
        let mut options = DecimalFormatterOptions::default();
        options.grouping_strategy = Some(if grouping {
            GroupingStrategy::Auto
        } else {
            GroupingStrategy::Never
        });
        DecimalFormatter::try_new((&language).into(), options)
            .expect("CLDR data for every locale's language is compiled in")
    })
}

/// The CLDR date and time formatters for British English, Ukrainian, Polish and American
/// English, made when first used.
static DATE_TIMES: [OnceLock<DateTimes>; 4] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

type DateTimes = FixedCalendarDateTimeFormatter<Gregorian, fieldsets::YMDT>;

/// `at`, milliseconds since 1970 in UTC, on the local clock in `locale`'s short date, its
/// year whole, and its time to the minute; `None` where the local clock cannot place it.
fn date_time(locale: Locale, at: u64) -> Option<String> {
    use chrono::{Datelike as _, TimeZone as _, Timelike as _};
    let local = chrono::Local
        .timestamp_millis_opt(i64::try_from(at).ok()?)
        .single()?;
    let date = Date::try_new_gregorian(local.year(), local.month() as u8, local.day() as u8);
    let time = Time::try_new(local.hour() as u8, local.minute() as u8, 0, 0);
    let at = DateTime {
        date: date.ok()?,
        time: time.ok()?,
    };
    let formatter = DATE_TIMES[locale.numbers()].get_or_init(|| {
        let language: icu_locale_core::Locale = locale
            .language()
            .parse()
            .expect("the locales' ids are valid; tested");
        let fields = fieldsets::YMD::short()
            .with_year_style(YearStyle::Full)
            .with_time_hm();
        FixedCalendarDateTimeFormatter::try_new((&language).into(), fields)
            .expect("CLDR data for every locale's language is compiled in")
    });
    Some(formatter.format(&at).to_string())
}

/// The CLDR formatters of a medium date with its weekday and a time to the second, and of
/// one without its weekday, for British English, Ukrainian, Polish and American English,
/// made when first used.
static WEEKDAYS_TO_SECOND: [OnceLock<FixedCalendarDateTimeFormatter<Gregorian, fieldsets::YMDET>>;
    4] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];
static DAYS_TO_SECOND: [OnceLock<DateTimes>; 4] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

/// `at` in `locale`'s medium date, with its weekday where `weekday`, and its time to the
/// second; `None` where the calendar cannot place it.
fn to_second(locale: Locale, at: chrono::NaiveDateTime, weekday: bool) -> Option<String> {
    use chrono::{Datelike as _, Timelike as _};
    let date = Date::try_new_gregorian(at.year(), at.month() as u8, at.day() as u8).ok()?;
    let time = Time::try_new(at.hour() as u8, at.minute() as u8, at.second() as u8, 0).ok()?;
    let at = DateTime { date, time };
    let language = || -> icu_locale_core::Locale {
        (locale.language().parse()).expect("the locales' ids are valid; tested")
    };
    let words = if weekday {
        let formatter = WEEKDAYS_TO_SECOND[locale.numbers()].get_or_init(|| {
            let fields = fieldsets::YMDE::medium().with_time_hms();
            FixedCalendarDateTimeFormatter::try_new((&language()).into(), fields)
                .expect("CLDR data for every locale's language is compiled in")
        });
        formatter.format(&at).to_string()
    } else {
        let formatter = DAYS_TO_SECOND[locale.numbers()].get_or_init(|| {
            let fields = fieldsets::YMD::medium().with_time_hms();
            FixedCalendarDateTimeFormatter::try_new((&language()).into(), fields)
                .expect("CLDR data for every locale's language is compiled in")
        });
        formatter.format(&at).to_string()
    };
    Some(words)
}

/// The CLDR formatters of a medium date, with its year and without, and of one with its
/// time to the minute, for British English, Ukrainian, Polish and American English, made
/// when first used.
static MEDIUM_DAYS: [OnceLock<FixedCalendarDateTimeFormatter<Gregorian, fieldsets::YMD>>; 4] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];
static MEDIUM_MONTH_DAYS: [OnceLock<FixedCalendarDateTimeFormatter<Gregorian, fieldsets::MD>>; 4] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];
static DAYS_TO_MINUTE: [OnceLock<DateTimes>; 4] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

/// The CLDR formatters of a long date with its weekday, made when first used.
static LONG_DAYS: [OnceLock<FixedCalendarDateTimeFormatter<Gregorian, fieldsets::YMDE>>; 4] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

/// `date` in `locale`'s long date with its weekday.
fn long_date(locale: Locale, date: chrono::NaiveDate) -> Option<String> {
    use chrono::Datelike as _;
    let at = Date::try_new_gregorian(date.year(), date.month() as u8, date.day() as u8).ok()?;
    let formatter = LONG_DAYS[locale.numbers()].get_or_init(|| {
        FixedCalendarDateTimeFormatter::try_new(
            (&language_of(locale)).into(),
            fieldsets::YMDE::long(),
        )
        .expect("CLDR data for every locale's language is compiled in")
    });
    Some(formatter.format(&at).to_string())
}

fn language_of(locale: Locale) -> icu_locale_core::Locale {
    (locale.language().parse()).expect("the locales' ids are valid; tested")
}

/// `date` in `locale`'s medium date, its year where `year`.
fn medium_date(locale: Locale, date: chrono::NaiveDate, year: bool) -> Option<String> {
    use chrono::Datelike as _;
    let at = Date::try_new_gregorian(date.year(), date.month() as u8, date.day() as u8).ok()?;
    // The formatter in `slot` for `fields`, made once a locale, writing `at`: one per field
    // set, each its own type.
    macro_rules! written {
        ($slot:expr, $fields:expr) => {{
            let formatter = $slot[locale.numbers()].get_or_init(|| {
                FixedCalendarDateTimeFormatter::try_new((&language_of(locale)).into(), $fields)
                    .expect("CLDR data for every locale's language is compiled in")
            });
            formatter.format(&at).to_string()
        }};
    }
    Some(if year {
        written!(MEDIUM_DAYS, fieldsets::YMD::medium())
    } else {
        written!(MEDIUM_MONTH_DAYS, fieldsets::MD::medium())
    })
}

/// `at`'s time to the minute as `locale` writes it after its medium date: the medium date
/// and time's words with the date's and the separator after it taken off.
fn to_minute(locale: Locale, at: chrono::NaiveDateTime) -> Option<String> {
    use chrono::Timelike as _;
    let day = medium_date(locale, at.date(), true)?;
    let date = {
        use chrono::Datelike as _;
        Date::try_new_gregorian(at.year(), at.month() as u8, at.day() as u8).ok()?
    };
    let time = Time::try_new(at.hour() as u8, at.minute() as u8, 0, 0).ok()?;
    let formatter = DAYS_TO_MINUTE[locale.numbers()].get_or_init(|| {
        let language = language_of(locale);
        let fields = fieldsets::YMD::medium().with_time_hm();
        FixedCalendarDateTimeFormatter::try_new((&language).into(), fields)
            .expect("CLDR data for every locale's language is compiled in")
    });
    let whole = formatter.format(&DateTime { date, time }).to_string();
    let time = match whole.strip_prefix(&day) {
        Some(rest) => rest.trim_start_matches(|c: char| c == ',' || c.is_whitespace()),
        None => whole.as_str(),
    };
    Some(time.to_owned())
}

/// The local calendar day `at` falls on, milliseconds since 1970 in UTC on the local
/// clock, counted in days from 1 January of the year 1: a day's number, the one before it
/// one less; none where the local clock cannot place it.
pub fn local_day(at: u64) -> Option<i32> {
    use chrono::{Datelike as _, TimeZone as _};
    let local = chrono::Local
        .timestamp_millis_opt(i64::try_from(at).ok()?)
        .single()?;
    Some(local.date_naive().num_days_from_ce())
}

/// The CLDR month-and-day formatters, and the year-month-and-day ones, for British
/// English, Ukrainian, Polish and American English, made when first used.
static MONTH_DAYS: [OnceLock<FixedCalendarDateTimeFormatter<Gregorian, fieldsets::MD>>; 4] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];
static YEAR_DAYS: [OnceLock<FixedCalendarDateTimeFormatter<Gregorian, fieldsets::YMD>>; 4] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

/// `day` in `locale`'s long date, its year left out where it is `today`'s.
fn calendar_day(locale: Locale, day: i32, today: i32) -> Option<String> {
    use chrono::{Datelike as _, NaiveDate};
    let date = NaiveDate::from_num_days_from_ce_opt(day)?;
    let this_year = NaiveDate::from_num_days_from_ce_opt(today)?.year() == date.year();
    let at = Date::try_new_gregorian(date.year(), date.month() as u8, date.day() as u8).ok()?;
    let language = || -> icu_locale_core::Locale {
        (locale.language().parse()).expect("the locales' ids are valid; tested")
    };
    let words = if this_year {
        let formatter = MONTH_DAYS[locale.numbers()].get_or_init(|| {
            FixedCalendarDateTimeFormatter::try_new((&language()).into(), fieldsets::MD::long())
                .expect("CLDR data for every locale's language is compiled in")
        });
        formatter.format(&at).to_string()
    } else {
        let formatter = YEAR_DAYS[locale.numbers()].get_or_init(|| {
            FixedCalendarDateTimeFormatter::try_new((&language()).into(), fieldsets::YMD::long())
                .expect("CLDR data for every locale's language is compiled in")
        });
        formatter.format(&at).to_string()
    };
    Some(words)
}

/// The locale's decimal separator: the character between 1 and 5 in "1.5".
fn decimal_separator(locale: Locale) -> char {
    signed_number(locale, 1.5, 1, false, false)
        .chars()
        .find(|c| !c.is_ascii_digit())
        .unwrap_or('.')
}

/// The ids of every message, from the English source.
pub fn message_ids() -> Vec<String> {
    Locale::EnGb
        .sources()
        .iter()
        .filter_map(|source| FluentResource::try_new((*source).to_owned()).ok())
        .flat_map(|resource| {
            resource
                .entries()
                .filter_map(|entry| match entry {
                    fluent_syntax::ast::Entry::Message(m) => Some(m.id.name.to_owned()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The pseudo-locale's accents, applied to the text between a message's variables.
fn accent(text: &str) -> Cow<'_, str> {
    const FROM: &str = "acdeghijklnorstuwyzACDEGHIJKLNORSTUWYZ";
    const TO: &str = "áçďéğĥíĵķĺñöŕšťüŵýžÁÇĎÉĞĤÍĴĶĹÑÖŔŠŤÜŴÝŽ";
    if !text.chars().any(|c| FROM.contains(c)) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.chars()
            .map(|c| {
                // FROM is ASCII, so its byte index is its character index.
                FROM.find(c).map_or(c, |i| TO.chars().nth(i).unwrap_or(c))
            })
            .collect(),
    )
}

/// The pseudo-locale's length: 40% longer than English, in brackets, so that clipped
/// text and the edges of a message show.
fn lengthen(text: &str) -> String {
    let extra = (text.chars().count() * 2).div_ceil(5);
    format!("[{text}{}]", "~".repeat(extra))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use fluent_syntax::ast;

    use super::*;

    fn count(t: &Translator, n: f64) -> String {
        let mut args = Args::new();
        args.set("count", n);
        t.text_with("photo-count", Some(&args))
    }

    #[test]
    fn days_follow_each_language_with_the_year_only_where_it_is_another() {
        use chrono::{Datelike as _, NaiveDate};
        let day = |y, m, d| {
            NaiveDate::from_ymd_opt(y, m, d)
                .expect("a day")
                .num_days_from_ce()
        };
        let today = day(2026, 11, 13);
        let written = |locale, at| Translator::new(locale).format_day(at, today);
        assert_eq!(written(Locale::EnGb, day(2026, 11, 12)), "12 November");
        assert_eq!(written(Locale::EnUs, day(2026, 11, 12)), "November 12");
        assert_eq!(written(Locale::Uk, day(2026, 11, 12)), "12 листопада");
        assert_eq!(written(Locale::Pl, day(2026, 11, 12)), "12 listopada");
        assert_eq!(written(Locale::EnGb, day(2025, 3, 14)), "14 March 2025");
        // Days count in the local calendar, the day before one less.
        use chrono::TimeZone as _;
        let noon = |d| {
            let local = chrono::Local
                .with_ymd_and_hms(2026, 11, d, 12, 0, 0)
                .single();
            u64::try_from(local.expect("a time").timestamp_millis()).expect("after 1970")
        };
        assert_eq!(local_day(noon(13)), Some(today));
        assert_eq!(local_day(noon(12)), Some(today - 1));
    }

    #[test]
    fn dates_and_times_follow_each_language() {
        use chrono::TimeZone as _;
        // 21 October 2026, 17:52 on the local clock, whatever the machine's zone.
        let local = chrono::Local
            .with_ymd_and_hms(2026, 10, 21, 17, 52, 30)
            .single()
            .expect("a time the local clock has");
        let at = u64::try_from(local.timestamp_millis()).expect("after 1970");
        let written = |locale| Translator::new(locale).format_date_time(at);
        assert_eq!(written(Locale::EnGb), "21/10/2026, 17:52");
        assert_eq!(written(Locale::EnUs), "10/21/2026, 5:52\u{202f}PM");
        assert_eq!(written(Locale::Uk), "21.10.2026, 17:52");
        assert_eq!(written(Locale::Pl), "21.10.2026, 17:52");
        assert_eq!(written(Locale::Pseudo), written(Locale::EnGb));
    }

    /// A camera's clock, to the second, with its weekday and its offset, and the size of a
    /// file, in each language (decision 110).
    #[test]
    fn a_clock_s_time_and_a_size_follow_each_language() {
        let at = chrono::NaiveDate::from_ymd_opt(2026, 7, 4)
            .and_then(|day| day.and_hms_opt(8, 10, 1))
            .expect("a time");
        let long = |locale, offset| Translator::new(locale).format_long_date_time(at, offset);
        assert_eq!(
            long(Locale::EnGb, Some(60)),
            "Sat, 4 Jul 2026, 08:10:01 (+01:00)"
        );
        assert_eq!(
            long(Locale::EnGb, Some(-330)),
            "Sat, 4 Jul 2026, 08:10:01 (−05:30)"
        );
        assert_eq!(
            long(Locale::EnUs, None),
            "Sat, Jul 4, 2026, 8:10:01\u{202f}AM"
        );
        assert_eq!(
            long(Locale::Uk, None),
            "сб, 4 лип. 2026\u{202f}р., 08:10:01"
        );
        assert_eq!(long(Locale::Pl, None), "sob., 4 lip 2026, 08:10:01");
        let spoken = |locale| Translator::new(locale).format_spoken_date(at.date());
        assert_eq!(spoken(Locale::EnGb), "Saturday, 4 July 2026");
        assert_eq!(spoken(Locale::EnUs), "Saturday, July 4, 2026");
        let short = |locale| Translator::new(locale).format_date_time_to_second(at);
        assert_eq!(short(Locale::EnGb), "4 Jul 2026, 08:10:01");
        let bytes = |locale, size, decimals, unit| {
            Translator::new(locale).format_bytes(size, decimals, unit)
        };
        assert_eq!(bytes(Locale::EnGb, 32.0, 0, "mb"), "32 MB");
        assert_eq!(bytes(Locale::EnGb, 1.5, 1, "gb"), "1.5 GB");
        assert_eq!(bytes(Locale::Uk, 1.5, 1, "gb"), "1,5 ГБ");
        assert_eq!(bytes(Locale::Pl, 4.0, 1, "kb"), "4,0 kB");
    }

    #[test]
    fn every_locale_is_an_option() {
        let options: Vec<Locale> = Locale::OPTIONS.iter().map(|(l, _)| *l).collect();
        assert_eq!(options, Locale::ALL);
    }

    #[test]
    fn plurals_follow_each_language() {
        let en = Translator::new(Locale::EnGb);
        assert_eq!(count(&en, 1.0), "1 photo");
        assert_eq!(count(&en, 2.0), "2 photos");
        assert_eq!(count(&en, 1.5), "1.5 photos");
        let us = Translator::new(Locale::EnUs);
        assert_eq!(count(&us, 1.0), "1 photo");
        assert_eq!(count(&us, 1.5), "1.5 photos");
        let uk = Translator::new(Locale::Uk);
        assert_eq!(count(&uk, 1.0), "1 знімок");
        assert_eq!(count(&uk, 3.0), "3 знімки");
        assert_eq!(count(&uk, 5.0), "5 знімків");
        assert_eq!(count(&uk, 21.0), "21 знімок");
        assert_eq!(count(&uk, 1.5), "1,5 знімка");
        let pl = Translator::new(Locale::Pl);
        assert_eq!(count(&pl, 1.0), "1 zdjęcie");
        assert_eq!(count(&pl, 2.0), "2 zdjęcia");
        assert_eq!(count(&pl, 5.0), "5 zdjęć");
        assert_eq!(count(&pl, 22.0), "22 zdjęcia");
        assert_eq!(count(&pl, 1.5), "1,5 zdjęcia");
    }

    /// A number given with at least a decimal keeps it, in each language's numbers.
    #[test]
    fn a_number_with_its_decimals_keeps_them() {
        let size = |locale, value| {
            let mut args = Args::new();
            args.set("size", decimals(value, 1));
            args.set("unit", "mb");
            Translator::new(locale).text_with("job-cache-size", Some(&args))
        };
        assert_eq!(size(Locale::EnGb, 1.0), "1.0 MB");
        assert_eq!(size(Locale::EnGb, 5.0), "5.0 MB");
        assert_eq!(size(Locale::EnGb, 1.25), "1.25 MB");
        assert_eq!(size(Locale::Uk, 1.5), "1,5 МБ");
        assert_eq!(size(Locale::Pl, 1.5), "1,5 MB");
    }

    #[test]
    fn numbers_in_messages_are_grouped_in_each_language_s_way() {
        assert_eq!(
            count(&Translator::new(Locale::EnGb), 12345.0),
            "12,345 photos"
        );
        assert_eq!(
            count(&Translator::new(Locale::EnUs), 12345.0),
            "12,345 photos"
        );
        assert_eq!(
            count(&Translator::new(Locale::Uk), 12345.0),
            "12\u{a0}345 знімків"
        );
        assert_eq!(
            count(&Translator::new(Locale::Pl), 12345.0),
            "12\u{a0}345 zdjęć"
        );
    }

    #[test]
    fn value_fields_format_numbers_for_the_locale() {
        let en = Translator::new(Locale::EnGb);
        let uk = Translator::new(Locale::Uk);
        let pl = Translator::new(Locale::Pl);
        assert_eq!(en.format_number(-1.25, 2, true), "−1.25");
        assert_eq!(uk.format_number(-1.25, 2, true), "−1,25");
        assert_eq!(pl.format_number(0.5, 2, true), "+0,50");
        // Signed, zero is "+0.00", as C's `%+` writes it, and so is a value that rounds to
        // it, where darktable prints "−0.00" (decision 42).
        assert_eq!(en.format_number(0.0, 2, true), "+0.00");
        assert_eq!(en.format_number(-0.0004, 2, true), "+0.00");
        assert_eq!(uk.format_number(-0.0, 1, true), "+0,0");
        assert_eq!(en.format_number(0.0, 2, false), "0.00");
        assert_eq!(en.format_number(-0.001, 2, false), "0.00");
        assert_eq!(en.format_number(-0.006, 2, true), "−0.01");
        // No grouping in a value field: a temperature stays compact.
        assert_eq!(uk.format_number(5600.0, 0, false), "5600");
        assert_eq!(en.format_number(12345.0, 0, false), "12345");
        // American English writes numbers as British English does.
        let us = Translator::new(Locale::EnUs);
        assert_eq!(us.format_number(-1.25, 2, true), "−1.25");
        assert_eq!(us.format_number(12345.0, 0, false), "12345");
        assert_eq!(us.decimal_separator(), '.');
        assert_eq!(us.parse_number("1,234.5"), Some(1234.5));
    }

    #[test]
    fn typed_numbers_are_read_in_the_locale_and_with_the_point() {
        let en = Translator::new(Locale::EnGb);
        let uk = Translator::new(Locale::Uk);
        assert_eq!(en.parse_number("1.5"), Some(1.5));
        assert_eq!(en.parse_number("−1.25 EV"), Some(-1.25));
        assert_eq!(en.parse_number("1,234.5"), Some(1234.5));
        assert_eq!(en.parse_number("+12%"), Some(12.0));
        assert_eq!(uk.parse_number("1,5"), Some(1.5));
        assert_eq!(uk.parse_number("1.5"), Some(1.5));
        assert_eq!(uk.parse_number("-0,7 EV"), Some(-0.7));
        assert_eq!(uk.parse_number("1\u{a0}234,5"), Some(1234.5));
        assert_eq!(en.parse_number("abc"), None);
        assert_eq!(en.parse_number(""), None);
    }

    #[test]
    fn the_locale_follows_the_user_s_languages() {
        assert_eq!(Locale::negotiate(["uk-UA"]), Locale::Uk);
        assert_eq!(Locale::negotiate(["pl-PL", "en"]), Locale::Pl);
        assert_eq!(Locale::negotiate(["de-DE", "uk"]), Locale::Uk);
        assert_eq!(Locale::negotiate(["en-US"]), Locale::EnUs);
        assert_eq!(Locale::negotiate(["EN-us"]), Locale::EnUs);
        // As the app passes Unix's LANG, without its encoding.
        assert_eq!(Locale::negotiate(["en_US"]), Locale::EnUs);
        assert_eq!(Locale::negotiate(["fr-FR", "en-US"]), Locale::EnUs);
        // Every other English is British English.
        assert_eq!(Locale::negotiate(["en-GB"]), Locale::EnGb);
        assert_eq!(Locale::negotiate(["en-AU"]), Locale::EnGb);
        assert_eq!(Locale::negotiate(["en_CA"]), Locale::EnGb);
        assert_eq!(Locale::negotiate(["en"]), Locale::EnGb);
        assert_eq!(Locale::negotiate(["fr"]), Locale::EnGb);
        assert_eq!(Locale::negotiate([]), Locale::EnGb);
        assert_eq!(Locale::negotiate(["en-XA"]), Locale::Pseudo);
        for locale in Locale::ALL {
            assert_eq!(Locale::from_id(locale.id()), Some(locale));
        }
    }

    #[test]
    fn the_pseudo_locale_is_longer_accented_and_bracketed() {
        let english = Translator::new(Locale::EnGb).text("action-copy-edit");
        let pseudo = Translator::new(Locale::Pseudo).text("action-copy-edit");
        assert_eq!(english, "Copy edit");
        assert!(pseudo.starts_with('[') && pseudo.ends_with(']'), "{pseudo}");
        assert!(pseudo.contains("Çöpý éďíť"), "{pseudo}");
        assert!(pseudo.chars().count() as f32 >= english.chars().count() as f32 * 1.4);
        // Numbers among the variables keep their digits.
        let mut args = Args::new();
        args.set("count", 12);
        let photos = Translator::new(Locale::Pseudo).text_with("photo-count", Some(&args));
        assert!(photos.starts_with("[12 "), "{photos}");
    }

    /// A search compares text in lower case with its accents off, in every language of
    /// the interface (decision 96).
    #[test]
    fn a_search_folds_case_and_accents() {
        assert_eq!(fold("Zmiękczenie ŁÓDŹ"), "zmiekczenie lodz");
        // Cyrillic's й, ї and ё are letters of their own, kept.
        assert_eq!(fold("Їжак Й Ёлка"), "їжак й ёлка");
        assert_ne!(fold("й"), fold("и"));
        assert_ne!(fold("ї"), fold("і"));
        assert_eq!(fold("\u{438}\u{306}"), "й");
        // An apostrophe however it is typed.
        assert_eq!(fold("Пом’якшення"), fold("Пом'якшення"));
        assert_eq!(fold("Помʼякшення"), "пом'якшення");
        assert_eq!(fold("Éclairé"), "eclaire");
        assert_eq!(fold("RGB curve"), "rgb curve");
    }

    /// A message's attribute, translated, a utility's section's too; one no language has
    /// is none.
    #[test]
    fn an_attribute_is_translated() {
        let search = |locale| Translator::new(locale).attribute("tool-exposure-title", "search");
        assert_eq!(search(Locale::EnGb).as_deref(), Some("exposure"));
        assert_eq!(search(Locale::Uk).as_deref(), Some("Експозиція"));
        let utility = Translator::new(Locale::Uk).attribute("section-scopes", "search");
        assert_eq!(utility.as_deref(), Some("Гістограми"));
        let none = Translator::new(Locale::Pl).attribute("tool-exposure-title", "nothing");
        assert_eq!(none, None);
    }

    #[test]
    fn a_missing_message_falls_back_to_english_then_its_id() {
        let uk = Translator::new(Locale::Uk);
        assert_eq!(uk.text("no-such-message"), "no-such-message");
        let us = Translator::new(Locale::EnUs);
        assert_eq!(us.text("no-such-message"), "no-such-message");
    }

    #[test]
    fn american_english_spells_its_own_words_and_takes_british_english_s_others() {
        let us = Translator::new(Locale::EnUs);
        assert_eq!(us.text("tab-color"), "Color");
        assert_eq!(us.text("tool-colorbalancergb-title"), "Color balance RGB");
        assert_eq!(us.text("action-copy-edit"), "Copy edit");
        // Its own messages are British English's too, as code that asks whether a message
        // exists sees them.
        assert!(us.has("action-copy-edit") && us.has("tab-color"));
        // A message that names a term or another message resolves in it.
        assert_eq!(us.text("menu-file"), "&File");
        assert_eq!(us.text("action-about"), "About Apertrust");
    }

    /// Each message's variables, by id, from a locale's files.
    fn variables(locale: Locale) -> BTreeMap<String, BTreeSet<String>> {
        let mut found = BTreeMap::new();
        for source in locale.sources() {
            let resource = FluentResource::try_new((*source).to_owned())
                .unwrap_or_else(|(_, errors)| panic!("{}: {errors:?}", locale.id()));
            for entry in resource.entries() {
                if let ast::Entry::Message(message) = entry {
                    let mut names = BTreeSet::new();
                    if let Some(pattern) = &message.value {
                        pattern_variables(pattern, &mut names);
                    }
                    let id = message.id.name.to_owned();
                    assert!(
                        found.insert(id.clone(), names).is_none(),
                        "{} defines {id} twice",
                        locale.id()
                    );
                }
            }
        }
        found
    }

    fn pattern_variables(pattern: &ast::Pattern<&str>, names: &mut BTreeSet<String>) {
        for element in &pattern.elements {
            if let ast::PatternElement::Placeable { expression } = element {
                expression_variables(expression, names);
            }
        }
    }

    fn expression_variables(expression: &ast::Expression<&str>, names: &mut BTreeSet<String>) {
        match expression {
            ast::Expression::Select { selector, variants } => {
                inline_variables(selector, names);
                for variant in variants {
                    pattern_variables(&variant.value, names);
                }
            }
            ast::Expression::Inline(inline) => inline_variables(inline, names),
        }
    }

    fn inline_variables(inline: &ast::InlineExpression<&str>, names: &mut BTreeSet<String>) {
        match inline {
            ast::InlineExpression::VariableReference { id } => {
                names.insert(id.name.to_owned());
            }
            ast::InlineExpression::Placeable { expression } => {
                expression_variables(expression, names)
            }
            _ => {}
        }
    }

    #[test]
    fn every_translation_is_complete_and_uses_english_s_variables() {
        let english = variables(Locale::EnGb);
        for locale in [Locale::Uk, Locale::Pl] {
            let translated = variables(locale);
            let missing: Vec<_> = english
                .keys()
                .filter(|id| !translated.contains_key(*id))
                .collect();
            let unused: Vec<_> = translated
                .keys()
                .filter(|id| !english.contains_key(*id))
                .collect();
            assert!(missing.is_empty(), "{} lacks {missing:?}", locale.id());
            assert!(
                unused.is_empty(),
                "{} has messages English lacks: {unused:?}",
                locale.id()
            );
            for (id, names) in &english {
                assert_eq!(
                    &translated[id],
                    names,
                    "{} uses other variables in {id}",
                    locale.id()
                );
            }
        }
    }

    /// Each message's text between its placeables, its variants' included, from a locale's
    /// files: what a reader sees, without its variables' names.
    fn words(locale: Locale) -> BTreeMap<String, String> {
        let mut found = BTreeMap::new();
        for source in locale.sources() {
            let resource = FluentResource::try_new((*source).to_owned())
                .unwrap_or_else(|(_, errors)| panic!("{}: {errors:?}", locale.id()));
            for entry in resource.entries() {
                if let ast::Entry::Message(message) = entry {
                    let mut text = String::new();
                    if let Some(pattern) = &message.value {
                        pattern_words(pattern, &mut text);
                    }
                    found.insert(message.id.name.to_owned(), text);
                }
            }
        }
        found
    }

    fn pattern_words(pattern: &ast::Pattern<&str>, text: &mut String) {
        for element in &pattern.elements {
            match element {
                ast::PatternElement::TextElement { value } => text.push_str(value),
                ast::PatternElement::Placeable {
                    expression: ast::Expression::Select { variants, .. },
                } => {
                    for variant in variants {
                        text.push(' ');
                        pattern_words(&variant.value, text);
                    }
                }
                ast::PatternElement::Placeable { .. } => text.push(' '),
            }
        }
    }

    #[test]
    fn american_english_is_an_overlay_of_british_english() {
        let english = variables(Locale::EnGb);
        let american = variables(Locale::EnUs);
        assert!(!american.is_empty());
        for (id, names) in &american {
            // An overlay never adds a message, and keeps the variables.
            let Some(british) = english.get(id) else {
                panic!("en-US has {id}, which en-GB lacks");
            };
            assert_eq!(names, british, "en-US uses other variables in {id}");
        }
        let english = words(Locale::EnGb);
        for (id, text) in words(Locale::EnUs) {
            assert_ne!(text, english[&id], "en-US copies en-GB's {id}");
        }
    }

    #[test]
    fn american_english_spells_every_colour_grey_and_centre_its_way() {
        let english = words(Locale::EnGb);
        let american = words(Locale::EnUs);
        for (id, british) in &english {
            let british = british.to_lowercase();
            for word in ["colour", "grey", "centre"] {
                if !british.contains(word) {
                    continue;
                }
                let Some(text) = american.get(id) else {
                    panic!("en-US lacks {id}, whose en-GB says {word}");
                };
                assert!(!text.to_lowercase().contains(word), "en-US {id}: {text}");
            }
        }
    }

    #[test]
    fn every_message_formats_without_errors() {
        for locale in Locale::ALL {
            let bundle = bundle(locale);
            for id in variables(locale).keys() {
                let pattern = bundle.get_message(id).and_then(|m| m.value()).unwrap();
                let mut args = Args::new();
                args.set("count", 3);
                args.set("section", "Exposure");
                args.set("tool", "Tone equalizer");
                args.set("group", "Margins");
                args.set("control", "Contrast");
                args.set("value", "+0.42");
                args.set("view", "Print");
                args.set("layer", "Sky");
                args.set("name", "IMG_2041.ARW");
                args.set("keys", "Ctrl+O");
                args.set("error", "No space left on device (os error 28)");
                args.set("index", 3);
                args.set("input", 42.5);
                args.set("output", 51);
                args.set("shadows", 0.12);
                args.set("highlights", 0.4);
                args.set("scope", "Histogram");
                args.set("clipping", "No clipping");
                args.set("version", "0.1.0+197~g9201df0");
                args.set("flag", "--conf");
                args.set("tab", "Photos");
                args.set("unit", "EV");
                args.set("done", 3);
                args.set("total", 12);
                args.set("failed", 1);
                args.set("photo", "IMG_2047.ARW");
                args.set("file", "IMG_2047.jpg");
                args.set("folder", "~/Pictures/apertrust_exported");
                args.set("cause", "the disk is full");
                args.set("running", 2);
                args.set("stoppable", 1);
                args.set("waiting", 1);
                args.set("value", "2 running");
                args.set("message", "Exporting 3 / 12 to file on disk");
                // A job's end on the local clock, after its destination.
                args.set("time", "14:02");
                // The export job's own.
                args.set("number", 3);
                args.set("storage", "disk");
                args.set("written", 10);
                args.set("skipped", 1);
                // The cache's Clear: what it clears, how many and in what unit.
                args.set("kind", "thumbnails");
                args.set("entries", 2104);
                args.set("unit", "mb");
                // Settings › Storage's: the cache's limit, and Show's words.
                args.set("limit", 4);
                args.set("show", "Show in Finder");
                // The Export dialog's.
                args.set("source", "IMG_2041.ARW");
                args.set("names", "$(OPENCL.ACTIVATED)");
                args.set("problem", "Enter where to put the exported photos.");
                args.set("suggestion", "FILE_NAME");
                args.set("expected", ")");
                args.set("variable", "$(FILE_NAME");
                args.set("found", "x");
                args.set("depth", 64);
                args.set("format", "JPEG");
                args.set("largest", 65535);
                args.set("why", "permission denied");
                // The folders': a library reason in plain words.
                args.set("reason", "the folder is missing");
                args.set("width", 2048);
                args.set("height", 1365);
                args.set("exist", 1);
                // The browser bar's: the sort's key and direction, the thumbnails' size.
                args.set("key", "file-name");
                args.set("descending", "yes");
                args.set("size", "large");
                // The grid cell's: its colour labels.
                args.set("labels", "Red, Blue");
                // The log window's counts of lines.
                args.set("kept", 1284);
                args.set("shown", 1107);
                // The zoom's tooltip: the pointer's modifier.
                args.set("command", "Ctrl");
                // A picker button's tooltip: the picker's own words.
                args.set("tip", "Pick colour from the photo");
                // The active picker: its tool and name, its own sentence, where it is on
                // the photo in percent, and what a pick changed.
                args.set("tool", "Exposure");
                args.set("picker", "Exposure");
                args.set("what", "Click or drag the point on a neutral grey.");
                for share in ["x", "y", "left", "top", "right", "bottom"] {
                    args.set(share, 50);
                }
                args.set("control", "Exposure");
                // Open Recent's item for a name two recents share: the folder it is in.
                args.set("parent", "Lisbon");
                // Edit › Undo and Redo: the step they would take back or put back.
                args.set("step", "Exposure");
                // History's tooltips: a control's value before and after.
                args.set("before", "+0.70 EV");
                args.set("after", "+1.20 EV");
                // History's lines: what changed, as screen readers hear it.
                args.set("change", "+0.70 EV to +1.20 EV");
                // History's groups: their steps' numbers and their tools; a day's date.
                args.set("first", 12);
                args.set("last", 16);
                args.set("tools", "Exposure, Grain");
                args.set("date", "12 November");
                // Collections' missing recent: its label, its open kind and its name.
                args.set("label", "Folder: 2026-10 Lisbon");
                // The start window's recent, to screen readers: its path.
                args.set("path", "~/Pictures/2026-10 Lisbon");
                // Settings' Hardware acceleration: a graphics API and an adapter's names.
                args.set("api", "Vulkan");
                args.set("card", "AMD Radeon 780M Graphics");
                // The start notice's: the graphics API it started on in place of another.
                args.set("other", "Direct3D 12");
                // A picker's words by a setting: the tool's number settings, as Exposure's
                // Lightness (decision 88).
                args.set("lightness", 92.0);
                args.set("reference_lightness", 62.0);
                args.set("reference_photo", "IMG_2041");
                // A reference row's photo.
                args.set("photo", "IMG_2041");
                // A short pick's warning: the L it reached and the L it was to reach.
                args.set("reached", 12);
                args.set("target", 50);
                for channel in ["red", "green", "blue", "grey"] {
                    args.set(channel, 128);
                }
                // The Export tool's end: the summary's line and list, its warnings and the
                // line under its button.
                args.set("pattern", "$(FILE_FOLDER)/$(FILE_NAME)");
                args.set("pixels_width", "6000");
                args.set("pixels_height", "4000");
                args.set("preset", "Default settings");
                args.set("format", "JPEG");
                args.set("size", "2048 px");
                args.set("limit", "2048 px");
                args.set("left", 1);
                args.set("quality", 95);
                args.set("bits", 16);
                args.set("percent", "50");
                args.set("unit", "cm");
                args.set("dpi", 300);
                args.set("option", "Create unique filename");
                // Where a moved tool went: its place among its tab's tools (decision 96).
                args.set("at", 3);
                args.set("of", 12);
                // Which of its tab's lists a tool is in, as Move Tool Up's tip says.
                args.set("list", "pinned");
                args.set("text", "colour");
                // A second start's flags for one run, which the app running refuses.
                args.set("flags", "--safe-mode");
                // The browser bar's count: the photos selected of its total, and a number
                // in thousands.
                args.set("selected", 3);
                args.set("thousands", "123");
                // The zoomed photo's scale, as its button shows it.
                args.set("scale", "100%");
                // A settings number's range, as the box says it when what is typed is out.
                args.set("min", "256");
                args.set("max", "2048");
                // The switch a dimmed settings row needs on.
                args.set("setting", "Limit the cache's size");
                // What a group's list in the cursors' well holds.
                args.set("has", "both");
                args.set("how", "Click a neutral area, or drag one");
                // The toolbar menu's Hide, naming its item, and the customise mode's lines.
                args.set("item", "Undo");
                args.set("workspace", "Default");
                // A style's history steps (decision 106).
                args.set("style", "Warm film");
                // A style on several photos: the photos it changed, and its notice's skipped
                // photo, why, and how many more (decision 109).
                args.set("changed", 11);
                args.set("said", "Warm film applied to 11 of 12 photos");
                args.set("why", "view-only");
                args.set("more", 0);
                // The caption's tooltips: what a control does, with its key.
                args.set("action", "Rate 3 stars");
                // The Metadata tool's values and tooltips (decision 110).
                args.set("code", "7");
                args.set("megapixels", "24.0");
                args.set("when", "8 Nov 2026, 10:42");
                args.set("steps", 14);
                args.set("x", "300");
                args.set("y", "300");
                args.set("time", "08:10:01");
                args.set("type", "ISO speed");
                args.set("source", "D65");
                args.set("range", "Macro");
                args.set("focal", "24–70 mm");
                args.set("aperture", "f/2.8");
                args.set("degrees", "38");
                args.set("minutes", "42.6");
                args.set("mode", "auto");
                args.set("fired", "yes");
                args.set("red_eye", "no");
                // Several photos: a range's ends, how many are read and how many have none.
                args.set("low", "1/2000");
                args.set("high", "1/60");
                args.set("read", "2,140");
                args.set("none", 3);
                // A rating's or a label's words and the person's note for it.
                args.set("words", "3 stars");
                args.set("note", "Print candidates");
                // A field's label, and a date as the language writes one.
                args.set("field", "Title");
                args.set("example", "04/07/2026");
                // How many different values several photos hold of a field.
                args.set("values", 3);
                // An import's photos copied (decision 85).
                args.set("copied", 120);
                let mut errors = Vec::new();
                bundle.format_pattern(pattern, Some(&args), &mut errors);
                assert!(errors.is_empty(), "{} {id}: {errors:?}", locale.id());
            }
        }
    }

    #[test]
    fn this_module_does_not_use_egui() {
        // So it can become the `apertrust-i18n` crate unchanged (docs/gui.md, Layers).
        let source = include_str!("i18n.rs");
        let code = source.split("#[cfg(test)]").next().unwrap_or_default();
        assert!(!code.contains("use egui") && !code.contains("egui::"));
    }
}
