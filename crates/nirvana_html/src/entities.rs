//! HTML entity decoding for character streams and attributes.

pub fn decode_entities(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity_buf = String::new();
            let mut found_semicolon = false;

            while let Some(&next_c) = chars.peek() {
                if next_c == ';' {
                    chars.next();
                    found_semicolon = true;
                    break;
                } else if next_c.is_alphanumeric() || next_c == '#' {
                    entity_buf.push(chars.next().unwrap());
                    if entity_buf.len() > 10 {
                        break;
                    }
                } else {
                    break;
                }
            }

            if found_semicolon
                && let Some(decoded) = decode_named_or_numeric_entity(&entity_buf)
            {
                result.push(decoded);
                continue;
            }

            // Not an entity or unrecognized: emit raw & and accumulated chars
            result.push('&');
            result.push_str(&entity_buf);
            if found_semicolon {
                result.push(';');
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn decode_named_or_numeric_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" | "AMP" => Some('&'),
        "lt" | "LT" => Some('<'),
        "gt" | "GT" => Some('>'),
        "quot" | "QUOT" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{00A0}'),
        "copy" | "COPY" => Some('\u{00A9}'),
        "reg" | "REG" => Some('\u{00AE}'),
        "trade" | "TRADE" => Some('\u{2122}'),
        "ndash" => Some('\u{2013}'),
        "mdash" => Some('\u{2014}'),
        "lsquo" => Some('\u{2018}'),
        "rsquo" => Some('\u{2019}'),
        "sbquo" => Some('\u{201A}'),
        "ldquo" => Some('\u{201C}'),
        "rdquo" => Some('\u{201D}'),
        "bdquo" => Some('\u{201E}'),
        "hellip" => Some('\u{2026}'),
        "bull" => Some('\u{2022}'),
        "middot" => Some('\u{00B7}'),
        "laquo" => Some('\u{00AB}'),
        "raquo" => Some('\u{00BB}'),
        "lsaquo" => Some('\u{2039}'),
        "rsaquo" => Some('\u{203A}'),
        "times" => Some('\u{00D7}'),
        "divide" => Some('\u{00F7}'),
        "plusmn" => Some('\u{00B1}'),
        "deg" => Some('\u{00B0}'),
        "euro" => Some('\u{20AC}'),
        "pound" => Some('\u{00A3}'),
        "yen" => Some('\u{00A5}'),
        "cent" => Some('\u{00A2}'),
        "sect" => Some('\u{00A7}'),
        "para" => Some('\u{00B6}'),
        "larr" => Some('\u{2190}'),
        "uarr" => Some('\u{2191}'),
        "rarr" => Some('\u{2192}'),
        "darr" => Some('\u{2193}'),
        "harr" => Some('\u{2194}'),
        "ensp" => Some('\u{2002}'),
        "emsp" => Some('\u{2003}'),
        "thinsp" => Some('\u{2009}'),
        "frac14" => Some('\u{00BC}'),
        "frac12" => Some('\u{00BD}'),
        "frac34" => Some('\u{00BE}'),
        "shy" => Some('\u{00AD}'),
        "iexcl" => Some('\u{00A1}'),
        "iquest" => Some('\u{00BF}'),
        "check" | "checkmark" => Some('\u{2713}'),
        s if s.starts_with("#x") || s.starts_with("#X") => {
            u32::from_str_radix(&s[2..], 16).ok().and_then(char::from_u32)
        }
        s if s.starts_with('#') => {
            s[1..].parse::<u32>().ok().and_then(char::from_u32)
        }
        _ => None,
    }
}
