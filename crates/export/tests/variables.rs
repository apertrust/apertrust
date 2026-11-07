// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2025  darktable developers (ported from src/tests/variables.c)
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

//! darktable's own variable tests (`src/tests/variables.c`) restated case by case,
//! each with darktable's file name, job code and sequence number, and D8's refusals
//! where darktable expands an unknown variable to nothing or leaves one it cannot close
//! as it is (`docs/ports/variables.md`).

use apertrust_export::variables::{NotHere, Pattern, PatternError, Unavailable, Values};

/// A darktable test: `filename`, `jobcode` and `sequence`.
fn values(path: &str, jobcode: &str, sequence: u32) -> Values {
    Values {
        path: path.to_owned(),
        jobcode: jobcode.to_owned(),
        sequence,
        ..Values::default()
    }
}

/// Every case expands as darktable's test expects.
fn expands(values: &Values, cases: &[(&str, &str)]) {
    for (input, expected) in cases {
        let pattern =
            Pattern::parse(input).unwrap_or_else(|error| panic!("{input}: refused: {error}"));
        assert_eq!(pattern.expand(values), *expected, "{input}");
    }
}

/// Every case is refused, with the error darktable's result would not show.
fn refuses(cases: &[(&str, &str, PatternError)]) {
    for (input, darktable, error) in cases {
        assert_eq!(
            Pattern::parse(input).as_ref(),
            Err(error),
            "{input}, which darktable expands to {darktable:?}"
        );
    }
}

fn unknown(name: &str, at: usize, suggestion: Option<&'static str>) -> PatternError {
    PatternError::Unknown {
        name: name.to_owned(),
        at: at..at + name.len(),
        suggestion,
    }
}

/// A variable starting `start` bytes in that stops at the end, not closed by `expected`.
fn unclosed_at(input: &str, start: usize, expected: char) -> PatternError {
    PatternError::Unclosed {
        variable: input[start..].to_owned(),
        expected,
        at: input.len(),
    }
}

const NAME: &str = "abcdef12345abcdef";
const JOB: &str = "ABCDEF12345ABCDEF";

#[test]
fn test_variables() {
    let values = values(NAME, JOB, 23);
    expands(
        &values,
        &[
            ("$(FILE_NAME)", "abcdef12345abcdef"),
            ("foo-$(FILE_NAME)-bar", "foo-abcdef12345abcdef-bar"),
            ("äöü-$(FILE_NAME)-äöü", "äöü-abcdef12345abcdef-äöü"),
            ("$(FILE_NAME).$(SEQUENCE)", "abcdef12345abcdef.0023"),
        ],
    );
    refuses(&[
        ("$(NONEXISTANT)", "", unknown("NONEXISTANT", 2, None)),
        (
            "foo-$(NONEXISTANT)-bar",
            "foo--bar",
            unknown("NONEXISTANT", 6, None),
        ),
    ]);
}

#[test]
fn test_simple_substitutions() {
    let values = values(NAME, JOB, 23);
    expands(
        &values,
        &[
            ("$(FILE_NAME-invälid)", "abcdef12345abcdef"),
            ("$(FILE_NAME+exißts)", "exißts"),
            ("$(FILE_NAME:0)", "abcdef12345abcdef"),
            ("$(FILE_NAME:5)", "f12345abcdef"),
            ("$(FILE_NAME:42)", ""),
            ("$(FILE_NAME:-5)", "bcdef"),
            ("$(FILE_NAME:-42)", "abcdef12345abcdef"),
            ("$(FILE_NAME:0:5)", "abcde"),
            ("$(FILE_NAME:5:3)", "f12"),
            ("$(FILE_NAME:5:42)", "f12345abcdef"),
            ("$(FILE_NAME:-5:3)", "bcd"),
            ("$(FILE_NAME:-7:-2)", "5abcd"),
            ("$(FILE_NAME:)", "abcdef12345abcdef"),
            ("$(FILE_NAME:5:)", ""),
            ("$(FILE_NAME#abc)", "def12345abcdef"),
            ("$(FILE_NAME#def)", "abcdef12345abcdef"),
            ("$(FILE_NAME%abc)", "abcdef12345abcdef"),
            ("$(FILE_NAME%def)", "abcdef12345abc"),
            ("$(FILE_NAME/abc/foobar)", "foobardef12345abcdef"),
            ("$(FILE_NAME/def/foobar)", "abcfoobar12345abcdef"),
            ("$(FILE_NAME//abc/foobar)", "foobardef12345foobardef"),
            ("$(FILE_NAME//def/foobar)", "abcfoobar12345abcfoobar"),
            ("$(FILE_NAME/#abc/foobar)", "foobardef12345abcdef"),
            ("$(FILE_NAME/#def/foobar)", "abcdef12345abcdef"),
            ("$(FILE_NAME/%abc/foobar)", "abcdef12345abcdef"),
            ("$(FILE_NAME/%def/foobar)", "abcdef12345abcfoobar"),
            ("$(FILE_NAME^)", "Abcdef12345abcdef"),
            ("$(FILE_NAME^^)", "ABCDEF12345ABCDEF"),
            ("$(JOBCODE,)", "aBCDEF12345ABCDEF"),
            ("$(JOBCODE,,)", "abcdef12345abcdef"),
        ],
    );
    let none = |name| unknown(name, 2, None);
    refuses(&[
        ("$(NONEXISTANT-invälid)", "invälid", none("NONEXISTANT")),
        ("$(NONEXISTANT+exißts)", "", none("NONEXISTANT")),
        ("$(NONEXISTANT:0)", "", none("NONEXISTANT")),
        ("$(NONEXISTANT#abc)", "", none("NONEXISTANT")),
        ("$(NONEXISTANT%abc)", "", none("NONEXISTANT")),
        ("$(NONEXISTANT/abc/def)", "", none("NONEXISTANT")),
        ("$(NONEXISTANT^)", "", none("NONEXISTANT")),
        ("$(NONEXISTANT^^)", "", none("NONEXISTANT")),
        ("$(NONEXISTANT,)", "", none("NONEXISTANT")),
        ("$(NONEXISTANT,,)", "", none("NONEXISTANT")),
    ]);
}

#[test]
fn test_recursive_substitutions() {
    // TITLE is darktable's legacy name of the title, which Apertrust has not yet.
    let values = values(NAME, JOB, 23);
    expands(
        &values,
        &[
            ("x$(TITLE-$(FILE_NAME))y", "xabcdef12345abcdefy"),
            ("x$(TITLE-a-$(FILE_NAME)-b)y", "xa-abcdef12345abcdef-by"),
            ("x$(SEQUENCE-$(FILE_NAME))y", "x0023y"),
            ("x$(FILE_NAME/12345/$(SEQUENCE))y", "xabcdef0023abcdefy"),
            ("x$(FILE_NAME/12345/.$(SEQUENCE).)y", "xabcdef.0023.abcdefy"),
        ],
    );
}

#[test]
fn test_broken_variables() {
    let values = values(NAME, JOB, 23);
    expands(&values, &[("x(NONEXISTANT23", "x(NONEXISTANT23")]);
    refuses(&[
        (
            "$(NONEXISTANT",
            "$(NONEXISTANT",
            unknown("NONEXISTANT", 2, None),
        ),
        (
            "$(FILE_NAME",
            "$(FILE_NAME",
            unclosed_at("$(FILE_NAME", 0, ')'),
        ),
        (
            "x$(FILE_NAME",
            "x$(FILE_NAME",
            unclosed_at("x$(FILE_NAME", 1, ')'),
        ),
        (
            "x$(TITLE-$(FILE_NAME)",
            "x$(TITLE-abcdef12345abcdef",
            unclosed_at("x$(TITLE-$(FILE_NAME)", 1, ')'),
        ),
    ]);
}

#[test]
fn test_escapes() {
    let values = values("/home/test/Images/IMG_0123.CR2", "/home/test/", 23);
    expands(
        &values,
        &[
            ("foobarbaz", "foobarbaz"),
            ("foo/bar/baz", "foo/bar/baz"),
            ("foo\\bar\\baz", "foobarbaz"),
            ("foo\\\\bar\\\\baz", "foo\\bar\\baz"),
            ("foo\\$(bar", "foo$(bar"),
            ("foo$\\(bar", "foo$(bar"),
            ("foo\\$\\(bar", "foo$(bar"),
            ("foo\\$(bar$(SEQUENCE)baz", "foo$(bar0023baz"),
            (
                "$(FILE_FOLDER)/darktable_exported/img_$(SEQUENCE)",
                "/home/test/Images/darktable_exported/img_0023",
            ),
            (
                "$(FILE_FOLDER)/darktable_exported/$(FILE_NAME)",
                "/home/test/Images/darktable_exported/IMG_0123",
            ),
        ],
    );
    refuses(&[(
        "foo$(bar$(SEQUENCE)baz",
        "foo$(bar0023baz",
        unknown("bar", 5, None),
    )]);
}

#[test]
fn test_real_paths() {
    let values = values("/home/test/Images/0023/IMG_0123.CR2", "/home/test", 23);
    expands(
        &values,
        &[
            ("$(FILE_FOLDER#$(JOBCODE))", "/Images/0023"),
            ("$(FILE_FOLDER#$(JOBCODE)/Images)", "/0023"),
            ("$(FILE_FOLDER%$(SEQUENCE))", "/home/test/Images/"),
            ("$(FILE_FOLDER%/$(SEQUENCE))", "/home/test/Images"),
            ("$(FILE_FOLDER/test/$(SEQUENCE))", "/home/0023/Images/0023"),
            (
                "$(FILE_FOLDER/test/$(SEQUENCE)-$(SEQUENCE))",
                "/home/0023-0023/Images/0023",
            ),
            (
                "$(FILE_FOLDER/test/$(SEQUENCE//0/o))",
                "/home/oo23/Images/0023",
            ),
            ("$(FILE_FOLDER/$(SEQUENCE)/XXX)", "/home/test/Images/XXX"),
            ("$(FILE_FOLDER/$(JOBCODE)\\///media/)", "/media/Images/0023"),
            (
                "$(FILE_FOLDER/\\/home\\/test\\///media/exports/)/darktable_exported/img_$(SEQUENCE)",
                "/media/exports/Images/0023/darktable_exported/img_0023",
            ),
            ("$(FILE_FOLDER/home/media)", "/media/test/Images/0023"),
        ],
    );
    refuses(&[
        (
            "$(FILE_FOLDER/",
            "$(FILE_FOLDER/",
            unclosed_at("$(FILE_FOLDER/", 0, '/'),
        ),
        (
            "$(FILE_FOLDER/home",
            "$(FILE_FOLDER/home",
            unclosed_at("$(FILE_FOLDER/home", 0, '/'),
        ),
        (
            "$(FILE_FOLDER/home/media",
            "$(FILE_FOLDER/home/media",
            unclosed_at("$(FILE_FOLDER/home/media", 0, ')'),
        ),
    ]);
}

#[test]
fn an_unknown_name_is_refused_with_the_nearest() {
    let error = Pattern::parse("$(FILE_FOLDER)/x/$(FILE_NAM)").unwrap_err();
    assert_eq!(error, unknown("FILE_NAM", 19, Some("FILE_NAME")));
    assert_eq!(
        error.to_string(),
        "unknown variable $(FILE_NAM); did you mean $(FILE_NAME)?"
    );
    // Names are darktable's, in its case; a name with more after it is not the name,
    // where darktable takes the name and drops what follows.
    for (input, name, suggestion) in [
        ("$(file_name)", "file_name", Some("FILE_NAME")),
        ("$(FILE_NAMEX)", "FILE_NAMEX", Some("FILE_NAME")),
        ("$(SEQUENCE12)", "SEQUENCE12", Some("SEQUENCE")),
        ("$(EXIF.YAER)", "EXIF.YAER", Some("EXIF.YEAR")),
        ("$(DATE.FOO)", "DATE.FOO", None),
    ] {
        assert_eq!(
            Pattern::parse(input),
            Err(unknown(name, 2, suggestion)),
            "{input}"
        );
    }
    // Inside an operation too.
    assert_eq!(
        Pattern::parse("$(TITLE-$(FILE_NMAE))"),
        Err(unknown("FILE_NMAE", 10, Some("FILE_NAME")))
    );
    // No name at all.
    let error = Pattern::parse("$()").unwrap_err();
    assert_eq!(error, unknown("", 2, None));
    assert_eq!(
        error.to_string(),
        "a variable's name is missing after \"$(\""
    );
    assert_eq!(Pattern::parse("x$("), Err(unknown("", 3, None)));
}

#[test]
fn a_variable_not_closed_is_refused() {
    let error = Pattern::parse("$(FILE_FOLDER)/$(FILE_NAME").unwrap_err();
    assert_eq!(error, unclosed_at("$(FILE_FOLDER)/$(FILE_NAME", 15, ')'));
    assert_eq!(error.to_string(), "missing \")\" after $(FILE_NAME");
    // After an offset, where darktable reads past what follows.
    assert_eq!(
        Pattern::parse("$(FILE_NAME:5x)"),
        Err(PatternError::Unclosed {
            variable: "$(FILE_NAME:5".to_owned(),
            expected: ')',
            at: 13,
        })
    );
    // A parameter's "]" before the variable's ")".
    assert_eq!(
        Pattern::parse("$(SEQUENCE[3)"),
        Err(PatternError::Unclosed {
            variable: "$(SEQUENCE[".to_owned(),
            expected: ']',
            at: 11,
        })
    );
    // darktable's own list inserts CATEGORY[,] for the person to fill; it takes the ","
    // as the operation, as darktable does, and what follows is not the ")".
    assert_eq!(
        Pattern::parse("$(CATEGORY[,])"),
        Err(PatternError::Unclosed {
            variable: "$(CATEGORY[,".to_owned(),
            expected: ')',
            at: 12,
        })
    );
    assert_eq!(
        Pattern::parse("$(CATEGORY0(places"),
        Err(PatternError::Unclosed {
            variable: "$(CATEGORY0".to_owned(),
            expected: ')',
            at: 11,
        })
    );
}

#[test]
fn a_character_that_is_no_operation_is_refused() {
    let error = Pattern::parse("$(FILE_NAME=x)").unwrap_err();
    assert_eq!(
        error,
        PatternError::NoOperation {
            variable: "$(FILE_NAME".to_owned(),
            found: '=',
            at: 11,
        }
    );
    assert_eq!(error.to_string(), "\"=\" after $(FILE_NAME is no operation");
    assert!(matches!(
        Pattern::parse("$(SEQUENCE4[2])"),
        Err(PatternError::NoOperation { found: '[', .. })
    ));
}

#[test]
fn parameters_read_as_darktable_reads_them() {
    let values = Values {
        path: "/home/test/Images/0023/IMG_0123.CR2".to_owned(),
        sequence: 23,
        ..Values::default()
    };
    expands(
        &values,
        &[
            ("$(SEQUENCE[2,5])", "27"),
            ("$(SEQUENCE[6])", "000023"),
            ("$(SEQUENCE3)", "023"),
            ("$(SEQUENCE[0,0])", "0023"),
            ("$(SEQUENCE[x])", "0023"),
            ("$(SEQUENCE[ 2 , 101 ])", "123"),
            ("$(ROLL.NAME)", "0023"),
            ("$(ROLL.NAME[])", "0023"),
            ("$(ROLL.NAME[2])", "Images/0023"),
            ("$(ROLL_NAME[9])", "/home/test/Images/0023"),
            ("$(ID[3])", ""),
            ("$(IMAGE.ID.NEXT[4])", ""),
            ("$(CATEGORY0(places))", ""),
            ("$(CATEGORY[1,places|cities])", ""),
            ("$(FILE.EXTENSION)", "CR2"),
            ("$(IMAGE.FILENAME)", "/home/test/Images/0023/IMG_0123.CR2"),
        ],
    );
    // darktable's SEQUENCE0 asks for no digit at all, so 0 has none.
    let sequence = |sequence| Values {
        sequence,
        ..Values::default()
    };
    expands(
        &sequence(0),
        &[("$(SEQUENCE0)", ""), ("$(SEQUENCE)", "0000")],
    );
    expands(
        &sequence(1),
        &[("$(SEQUENCE0)", "1"), ("$(SEQUENCE[1,0])", "1")],
    );
}

#[test]
fn darktables_names_read_as_apertrusts() {
    let values = Values {
        version: "0.4.0+12~gabcdef0".to_owned(),
        ..Values::default()
    };
    expands(
        &values,
        &[
            (
                "$(DARKTABLE.NAME) $(DARKTABLE.VERSION)",
                "Apertrust 0.4.0+12~gabcdef0",
            ),
            (
                "$(DARKTABLE_NAME)-$(DARKTABLE_VERSION)",
                "Apertrust-0.4.0+12~gabcdef0",
            ),
            (
                "$(APERTRUST.NAME)/$(APERTRUST.VERSION)",
                "Apertrust/0.4.0+12~gabcdef0",
            ),
        ],
    );
}

#[test]
fn a_variable_with_no_value_here_expands_to_nothing_and_says_why() {
    let pattern = Pattern::parse(
        "$(FILE_NAME)_$(OPENCL.ACTIVATED)$(VERSION-$(IMAGE.ID))$(WORKSPACE.LABEL)$(VERSION)",
    )
    .unwrap();
    let values = values("/photos/IMG_0001.CR3", "export", 1);
    assert_eq!(pattern.expand(&values), "IMG_0001_");
    assert_eq!(
        pattern.not_here(),
        [
            Unavailable {
                name: "OPENCL.ACTIVATED",
                why: NotHere::OpenCl,
            },
            Unavailable {
                name: "VERSION",
                why: NotHere::Duplicates,
            },
            Unavailable {
                name: "IMAGE.ID",
                why: NotHere::Library,
            },
            Unavailable {
                name: "WORKSPACE.LABEL",
                why: NotHere::Workspaces,
            },
        ]
    );
    assert_eq!(NotHere::OpenCl.to_string(), "Apertrust does not use OpenCL");
    // darktable's default pattern needs nothing Apertrust lacks.
    let default = Pattern::parse("$(FILE_FOLDER)/apertrust_exported/$(FILE_NAME)").unwrap();
    assert!(default.not_here().is_empty());
    assert_eq!(
        default.expand(&values),
        "/photos/apertrust_exported/IMG_0001"
    );
}

#[test]
fn the_list_is_darktables_with_its_descriptions() {
    let list = apertrust_export::variables::list();
    let names: Vec<&str> = list.iter().map(|listed| listed.insert()).collect();
    assert_eq!(
        &names[..4],
        ["ROLL.NAME[]", "FILE.FOLDER", "FILE.NAME", "FILE.EXTENSION"]
    );
    let sequence = list.iter().find(|l| l.name() == "SEQUENCE").unwrap();
    assert_eq!(sequence.insert(), "SEQUENCE[4,1]");
    assert_eq!(
        sequence.description(),
        "$(SEQUENCE[n,m]) - sequence number, n: number of digits, m: start number"
    );
    assert_eq!(sequence.not_here(), None);
    let stars = list.iter().find(|l| l.name() == "STARS").unwrap();
    assert_eq!(stars.not_here(), Some(NotHere::Ratings));
    // Every one the list inserts is read, as the list writes it, once filled.
    for listed in list {
        let insert = listed.insert().replace("[,]", "[1,places]");
        let pattern = format!("$({insert})");
        assert!(Pattern::parse(&pattern).is_ok(), "{pattern}");
    }
}

#[test]
fn variables_nested_too_deep_are_refused() {
    use apertrust_export::variables::MAX_DEPTH;
    let nested = |depth: usize| "$(TITLE-".repeat(depth) + "x" + &")".repeat(depth);
    // As deep as allowed, read and expanded.
    let pattern = Pattern::parse(&nested(MAX_DEPTH)).unwrap();
    assert_eq!(pattern.expand(&Values::default()), "x");
    assert_eq!(
        Pattern::parse(&nested(MAX_DEPTH + 1)),
        Err(PatternError::TooDeep {
            at: MAX_DEPTH * "$(TITLE-".len(),
        })
    );
    // Far deeper than any stack takes, refused before it goes deep.
    let error = Pattern::parse(&nested(30_000)).unwrap_err();
    assert!(matches!(error, PatternError::TooDeep { .. }), "{error}");
    assert_eq!(error.to_string(), "variables are nested more than 64 deep");
}

#[test]
fn a_long_name_is_refused_quickly_and_shown_short() {
    let name = "A".repeat(1 << 20);
    let error = Pattern::parse(&format!("$({name})")).unwrap_err();
    assert_eq!(error, unknown(&name, 2, None));
    assert_eq!(
        error.to_string(),
        format!("unknown variable $({}…)", &name[..64])
    );
    let error = Pattern::parse(&format!("$(TITLE-{name}")).unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("missing \")\" after $(TITLE-{}…", &name[..56])
    );
}

#[test]
fn a_length_is_read_into_an_int_as_darktable_reads_it() {
    // darktable-cli exports a$(FILE_NAME:0:4294967296)b as ab.jpg: 2^32 is 0.
    let values = values("/photos/IMG_0001.CR3", "export", 1);
    expands(
        &values,
        &[
            ("a$(FILE_NAME:0:4294967296)b", "ab"),
            ("a$(FILE_NAME:0:4294967299)b", "aIMGb"),
        ],
    );
}
