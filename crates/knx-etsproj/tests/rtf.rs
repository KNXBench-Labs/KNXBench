//! Bounded external-text interpretation; no IO or mutation of original content.
use knx_etsproj::rtf::{
    inspect_text, write_supported_rtf, TextError, TextFormat, TextRun, TextStyle,
};

fn text(input: &str) -> String {
    inspect_text(input).unwrap().plain_text()
}

#[test]
fn plain_text_is_literal_and_never_interpreted_as_html() {
    let input = "<img src=\"https://invalid.example\">\nGrüße {engineer}";
    let view = inspect_text(input).unwrap();
    assert_eq!(view.format, TextFormat::Plain);
    assert_eq!(view.plain_text(), input);
    assert!(view.unsupported.is_empty());
}

#[test]
fn rtf_header_and_escaped_syntax_are_decoded_without_changing_source() {
    let input = String::from(r"{\rtf1\ansi One \\ two \{three\} and \~four}");
    let original = input.clone();
    let view = inspect_text(&input).unwrap();
    assert_eq!(view.format, TextFormat::Rtf);
    assert_eq!(view.plain_text(), "One \\ two {three} and \u{a0}four");
    assert_eq!(input, original);
}

#[test]
fn paragraph_line_tab_and_ignored_source_newlines_are_distinct() {
    assert_eq!(text("{\\rtf1 a\r\nb\\par c\\line d\\tab e}"), "ab\nc\nd\te");
}

#[test]
fn group_closure_restores_basic_character_formatting() {
    let view = inspect_text(r"{\rtf1 a{\b b{\i c}d}e\ul f\ulnone g}").unwrap();
    assert_eq!(view.plain_text(), "abcdefg");
    assert_eq!(view.runs.len(), 7);
    assert_eq!(view.runs[0].style, TextStyle::default());
    assert!(view.runs[1].style.bold);
    assert!(view.runs[2].style.bold && view.runs[2].style.italic);
    assert!(view.runs[3].style.bold && !view.runs[3].style.italic);
    assert_eq!(view.runs[4].style, TextStyle::default());
    assert!(view.runs[5].style.underline);
    assert_eq!(view.runs[6].style, TextStyle::default());
}

#[test]
fn toggles_accept_nonzero_and_plain_resets_the_supported_style() {
    let view = inspect_text(r"{\rtf1\b2\i\ul x\b0 y\plain z}").unwrap();
    assert!(view.runs[0].style.bold);
    assert!(!view.runs[1].style.bold && view.runs[1].style.italic);
    assert_eq!(view.runs[2].style, TextStyle::default());
}

#[test]
fn unicode_fallback_is_scoped_and_counts_escaped_characters() {
    assert_eq!(text(r"{\rtf1\uc1 \u252?{\uc2\u228\'61\'62}\u246?}"), "üäö");
    assert_eq!(text(r"{\rtf1\uc0 \u252X}"), "üX");
    assert_eq!(text(r"{\rtf1\u65\~B}"), "AB");
}

#[test]
fn signed_utf16_surrogates_form_one_non_bmp_scalar() {
    assert_eq!(text(r"{\rtf1\u-10179?\u-8704?}"), "😀");
    for input in [r"{\rtf1\u-10179?x}", r"{\rtf1\u-8704?}", r"{\rtf1\u65536?}"] {
        assert_eq!(
            inspect_text(input),
            Err(TextError::InvalidUnicode),
            "{input}"
        );
    }
}

#[test]
fn explicit_windows_1252_escapes_are_not_utf8_bytes() {
    assert_eq!(text(r"{\rtf1\ansi\ansicpg1252 \'fc\'80}"), "ü€");
    assert_eq!(
        inspect_text(r"{\rtf1\ansicpg932 \'82\'a0}"),
        Err(TextError::UnsupportedEncoding)
    );
    assert_eq!(
        inspect_text(r"{\rtf1 \'fc}"),
        Err(TextError::UnsupportedEncoding)
    );
    assert_eq!(text(r"{\rtf1 \'41}"), "A");
}

#[test]
fn unknown_controls_are_reported_and_prevent_subset_rewriting() {
    let view = inspect_text(r"{\rtf1 a\future42 b\future0 c}").unwrap();
    assert_eq!(view.plain_text(), "abc");
    assert_eq!(view.unsupported, vec!["future"]);
    assert_eq!(
        write_supported_rtf(&view),
        Err(TextError::UnsupportedFormatting)
    );
}

#[test]
fn ignorable_destinations_and_active_content_never_become_body_text() {
    for destination in ["object", "pict", "field", "fonttbl", "colortbl", "info"] {
        let input =
            format!(r"{{\rtf1 before{{\{destination} https://invalid.example secret}}after}}");
        let view = inspect_text(&input).unwrap();
        assert_eq!(view.plain_text(), "beforeafter");
        assert!(view.unsupported.iter().any(|entry| entry == destination));
        assert_eq!(
            write_supported_rtf(&view),
            Err(TextError::UnsupportedFormatting)
        );
    }
    let view = inspect_text(r"{\rtf1 a{\*\unknown hidden{nested}}b}").unwrap();
    assert_eq!(view.plain_text(), "ab");
    assert!(view.unsupported.iter().any(|entry| entry == "unknown"));
}

#[test]
fn binary_data_refuses_preview_even_inside_an_ignored_destination() {
    assert_eq!(
        inspect_text(r"{\rtf1{\pict\bin3 abc}}"),
        Err(TextError::UnsupportedBinary)
    );
}

#[test]
fn malformed_rtf_is_refused_instead_of_returning_an_authoritative_preview() {
    for input in [
        r"{\rtf2 text}",
        r"{\rtf1 text",
        r"{\rtf1 text}}",
        r"{\rtf1} trailing",
        r"{\rtf1 \'x0}",
        "{\\rtf1 abc\\}",
        r"{\rtf1 \uc-1 x}",
        r"{\rtf1 \b- x}",
        r"{\rtf1 \bad12345678901 x}",
    ] {
        assert_eq!(inspect_text(input), Err(TextError::Malformed), "{input}");
    }
}

#[test]
fn parser_budgets_refuse_without_mutating_the_original() {
    use knx_etsproj::rtf::{MAX_DEPTH, MAX_OUTPUT_BYTES, MAX_SOURCE_BYTES};
    let input = "x".repeat(MAX_SOURCE_BYTES + 1);
    assert_eq!(inspect_text(&input), Err(TextError::InputLimit));
    assert_eq!(input.len(), MAX_SOURCE_BYTES + 1);
    let output = format!("{{\\rtf1 {}}}", "x".repeat(MAX_OUTPUT_BYTES + 1));
    assert_eq!(inspect_text(&output), Err(TextError::OutputLimit));
    let deep = format!(
        "{{\\rtf1 {}x{}}}",
        "{".repeat(MAX_DEPTH),
        "}".repeat(MAX_DEPTH)
    );
    assert_eq!(inspect_text(&deep), Err(TextError::DepthLimit));
}

#[test]
fn bounded_unsupported_inventory_refuses_adversarial_control_names() {
    // Distinct alphabetic control names; numeric arguments alone are not identities.
    let controls: String = (0..257)
        .map(|n| {
            format!(
                "\\future{}{} ",
                (b'a' + (n / 26) as u8) as char,
                (b'a' + (n % 26) as u8) as char
            )
        })
        .collect();
    assert_eq!(
        inspect_text(&format!("{{\\rtf1 {controls}}}")),
        Err(TextError::UnsupportedLimit)
    );
}

#[test]
fn supported_writer_roundtrips_text_and_styles_without_claiming_source_bytes() {
    let input = r"{\rtf1\ansi\ansicpg1252 one{\b two} \u252? \u-10179?\u-8704?\par end}";
    let first = inspect_text(input).unwrap();
    let encoded = write_supported_rtf(&first).unwrap();
    let second = inspect_text(&encoded).unwrap();
    assert_eq!(first.runs, second.runs);
    assert_eq!(second.format, TextFormat::Rtf);
    assert!(second.unsupported.is_empty());
    assert_eq!(
        input,
        r"{\rtf1\ansi\ansicpg1252 one{\b two} \u252? \u-10179?\u-8704?\par end}"
    );
}

#[test]
fn writer_escapes_literal_markup_backslashes_braces_and_unicode() {
    let mut view = inspect_text("plain").unwrap();
    view.runs = vec![TextRun {
        text: "<script>\\{ü😀}\n\t".into(),
        style: TextStyle {
            bold: true,
            ..TextStyle::default()
        },
    }];
    let encoded = write_supported_rtf(&view).unwrap();
    assert_eq!(inspect_text(&encoded).unwrap().runs, view.runs);
}

#[test]
fn control_words_are_case_sensitive_and_unknown_styles_remain_partial() {
    let view = inspect_text(r"{\rtf1\B upper\b lower}").unwrap();
    assert!(!view.runs[0].style.bold);
    assert!(view.runs[1].style.bold);
    assert_eq!(view.unsupported, vec!["B"]);
}

#[test]
fn charset_uncertainty_and_undefined_windows_bytes_refuse_instead_of_guessing() {
    for value in [
        r"{\rtf1\ansi\ansicpg1252{\fonttbl{\f0\fcharset128 Name;}} \'fc}",
        r"{\rtf1\ansicpg1252 \'81}",
        "{\\rtf1 literal ü}",
    ] {
        assert_eq!(inspect_text(value), Err(TextError::UnsupportedEncoding));
    }
}

#[test]
fn unicode_fallback_stops_at_group_boundaries_and_skips_control_words() {
    assert_eq!(text(r"{\rtf1\uc2\u65?{b}c}"), "Abc");
    assert_eq!(text(r"{\rtf1\uc1\u65\b B}"), "AB");
    assert!(
        !inspect_text(r"{\rtf1\uc1\u65\b B}").unwrap().runs[0]
            .style
            .bold
    );
}

#[test]
fn token_and_style_run_budgets_are_explicit_preview_refusals() {
    let ignored = format!("{{\\rtf1 {{\\info {}}}}}", "x".repeat(100_001));
    assert_eq!(inspect_text(&ignored), Err(TextError::TokenLimit));
    let many = format!(
        "{{\\rtf1 {}}}",
        r"\b x\b0 y".repeat(knx_etsproj::rtf::MAX_RUNS / 2 + 1)
    );
    assert_eq!(inspect_text(&many), Err(TextError::RunLimit));
    assert_eq!(
        inspect_text(r"{\rtf1\uc17\u65?}"),
        Err(TextError::TokenLimit)
    );
}

#[test]
fn empty_values_crlf_and_all_basic_styles_have_deterministic_writer_semantics() {
    for value in ["", "ASCII", "\\{}", "é中😀", "\r\n\t\0", "a\u{a0}b"] {
        for mask in 0..8 {
            let mut view = inspect_text(value).unwrap();
            for run in &mut view.runs {
                run.style = TextStyle {
                    bold: mask & 1 != 0,
                    italic: mask & 2 != 0,
                    underline: mask & 4 != 0,
                };
            }
            let written = write_supported_rtf(&view).unwrap();
            assert_eq!(inspect_text(&written).unwrap().runs, view.runs);
        }
    }
}

#[test]
fn bounded_malformed_utf8_candidates_cannot_panic_or_execute_anything() {
    let alphabet = [
        '{', '}', '\\', '\'', '*', 'u', 'b', '-', '0', '?', 'é', '😀', '\n',
    ];
    let mut state = 1_u32;
    for _ in 0..1000 {
        let mut value = String::from("{\\rtf1 ");
        for _ in 0..64 {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            value.push(alphabet[state as usize % alphabet.len()]);
        }
        let original = value.clone();
        let result = std::panic::catch_unwind(|| inspect_text(&value));
        assert!(result.is_ok(), "bounded UTF-8 input must never panic");
        assert_eq!(value, original);
    }
}
