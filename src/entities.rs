//! Text-context HTML character references. Attribute-context rules differ.
use std::borrow::Cow;

pub(crate) fn decode(text: &str) -> Cow<'_, str> {
    let mut output = None::<String>;
    let mut copied = 0;
    for (offset, _) in text.match_indices('&') {
        if offset < copied {
            continue;
        }
        let rest = &text[offset + 1..];
        let entity = if let Some(number) = rest.strip_prefix('#') {
            let (number, radix, prefix) = match number.as_bytes().first() {
                Some(b'x' | b'X') => (&number[1..], 16, 2),
                _ => (number, 10, 1),
            };
            let digits = number
                .bytes()
                .take_while(|c| {
                    if radix == 16 {
                        c.is_ascii_hexdigit()
                    } else {
                        c.is_ascii_digit()
                    }
                })
                .count();
            if digits == 0 {
                None
            } else {
                let value = u32::from_str_radix(&number[..digits], radix).unwrap_or(u32::MAX);
                let semicolon = usize::from(number.as_bytes().get(digits) == Some(&b';'));
                Some((prefix + digits + semicolon, numeric(value), None))
            }
        } else {
            // Entity names are ASCII; longest match includes legacy semicolonless names.
            let length = rest
                .bytes()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == b';')
                .take(33)
                .count();
            (1..=length).rev().find_map(|length| {
                let &(first, second) = web_atoms::NAMED_ENTITIES.get(&rest[..length])?;
                if first == 0 {
                    return None;
                }
                Some((
                    length,
                    char::from_u32(first)?,
                    if second == 0 {
                        None
                    } else {
                        char::from_u32(second)
                    },
                ))
            })
        };
        if let Some((consumed, first, second)) = entity {
            let output = output.get_or_insert_with(|| String::with_capacity(text.len()));
            output.push_str(&text[copied..offset]);
            output.push(first);
            if let Some(second) = second {
                output.push(second);
            }
            copied = offset + 1 + consumed;
        }
    }
    match output {
        None => Cow::Borrowed(text),
        Some(mut output) => {
            output.push_str(&text[copied..]);
            Cow::Owned(output)
        }
    }
}

fn numeric(value: u32) -> char {
    // HTML replaces legacy Windows-1252 numeric references in this range.
    const C1: [u32; 32] = [
        0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039,
        0x152, 0x8d, 0x17d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
        0x2dc, 0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
    ];
    let value = if (0x80..=0x9f).contains(&value) {
        C1[(value - 0x80) as usize]
    } else {
        value
    };
    if value == 0 {
        '\u{fffd}'
    } else {
        char::from_u32(value).unwrap_or('\u{fffd}')
    }
}

#[test]
fn references_and_invalid_numbers() {
    assert_eq!(decode("plain"), "plain");
    assert!(matches!(decode("plain"), Cow::Borrowed(_)));
    assert_eq!(
        decode("&amp;lt; &notit; &#128; &#0; &#xD800; &#999999999999; &#65 &#x41 &unknown;"),
        "&lt; ¬it; € � � � A A &unknown;"
    );
    assert_eq!(decode("é &NotEqualTilde; &amp;中文"), "é ≂\u{338} &中文");
}
