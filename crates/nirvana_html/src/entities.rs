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

            if found_semicolon {
                if let Some(decoded) = decode_named_or_numeric_entity(&entity_buf) {
                    result.push(decoded);
                    continue;
                }
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
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{00A0}'),
        "copy" => Some('\u{00A9}'),
        "reg" => Some('\u{00AE}'),
        s if s.starts_with("#x") || s.starts_with("#X") => {
            u32::from_str_radix(&s[2..], 16).ok().and_then(char::from_u32)
        }
        s if s.starts_with('#') => {
            s[1..].parse::<u32>().ok().and_then(char::from_u32)
        }
        _ => None,
    }
}
