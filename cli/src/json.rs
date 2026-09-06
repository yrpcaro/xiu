//! Minimal JSON reader for the xiu CLI — one file, no dependencies outside
//! std, exactly enough to read the update engine's single output object and
//! the update manifest. Objects keep insertion order (the engine's key order
//! is stable, so reports print in the order it wrote them).

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// The value at `key` in an object, or None for anything else.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// A number as i64 when it is integral (counts, "behind"), else None.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Num(n) if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 => {
                Some(*n as i64)
            }
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }
}

impl fmt::Display for Json {
    /// The raw string for Str, a compact form for everything else. Only used
    /// for error messages, so the compact form can stay crude.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Json::Null => write!(f, "null"),
            Json::Bool(b) => write!(f, "{b}"),
            Json::Num(n) => write!(f, "{n}"),
            Json::Str(s) => write!(f, "{s}"),
            Json::Arr(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Json::Obj(pairs) => {
                write!(f, "{{")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

pub fn parse(text: &str) -> Result<Json, String> {
    let mut p = Parser { bytes: text.as_bytes(), pos: 0 };
    p.skip_ws();
    let value = p.value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(format!("unexpected trailing content at byte {}", p.pos));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        match self.peek() {
            Some(b) if b == byte => {
                self.pos += 1;
                Ok(())
            }
            other => Err(format!("expected '{}' at byte {}, found {:?}", byte as char, self.pos, other)),
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::Str),
            Some(b't') => self.literal("true").map(|_| Json::Bool(true)),
            Some(b'f') => self.literal("false").map(|_| Json::Bool(false)),
            Some(b'n') => self.literal("null").map(|_| Json::Null),
            Some(b'-') | Some(b'0'..=b'9') => self.number(),
            other => Err(format!("unexpected {:?} at byte {}", other.map(|b| b as char), self.pos)),
        }
    }

    fn literal(&mut self, word: &str) -> Result<(), String> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(())
        } else {
            Err(format!("invalid literal at byte {}", self.pos))
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(b'0'..=b'9') | Some(b'.') | Some(b'e') | Some(b'E') | Some(b'+') | Some(b'-')) {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| "bad number".to_string())?;
        text.parse::<f64>()
            .map(Json::Num)
            .map_err(|_| format!("invalid number '{text}'"))
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let byte = self.peek().ok_or("unterminated string")?;
            self.pos += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = self.peek().ok_or("unterminated escape")?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let cp = self.hex4()?;
                            // A surrogate pair completes a non-BMP code point;
                            // a lone surrogate is kept as its replacement char
                            // rather than failing the whole parse.
                            let ch = if (0xD800..=0xDBFF).contains(&cp) {
                                if self.bytes[self.pos..].starts_with(b"\\u") {
                                    self.pos += 2;
                                    let low = self.hex4()?;
                                    if (0xDC00..=0xDFFF).contains(&low) {
                                        let combined =
                                            0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                                        char::from_u32(combined).unwrap_or('\u{FFFD}')
                                    } else {
                                        '\u{FFFD}'
                                    }
                                } else {
                                    '\u{FFFD}'
                                }
                            } else {
                                char::from_u32(cp).unwrap_or('\u{FFFD}')
                            };
                            out.push(ch);
                        }
                        other => return Err(format!("bad escape '\\{}'", other as char)),
                    }
                }
                _ => {
                    // Collect the raw UTF-8 bytes of one char.
                    let len = utf8_len(byte);
                    let start = self.pos - 1;
                    let end = start + len;
                    if end > self.bytes.len() {
                        return Err("truncated utf-8".to_string());
                    }
                    self.pos = end;
                    match std::str::from_utf8(&self.bytes[start..end]) {
                        Ok(s) => out.push_str(s),
                        Err(_) => out.push('\u{FFFD}'),
                    }
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.bytes.len() {
            return Err("truncated \\u escape".to_string());
        }
        let text = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
            .map_err(|_| "bad \\u escape".to_string())?;
        let cp = u32::from_str_radix(text, 16).map_err(|_| format!("bad \\u escape '{text}'"))?;
        self.pos += 4;
        Ok(cp)
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect(b'{')?;
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Obj(pairs));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            let value = self.value()?;
            pairs.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Obj(pairs));
                }
                other => return Err(format!("expected ',' or '}}' at byte {}, found {:?}", self.pos, other)),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Arr(items));
                }
                other => return Err(format!("expected ',' or ']' at byte {}, found {:?}", self.pos, other)),
            }
        }
    }
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_engine_shape() {
        let text = r#"{"status":"ok","behind":3,"applied":false,"restartNeeded":true,
            "version":"a1b2c3d 2026-09-07","changelog":["one","two"],
            "missingDeps":[{"id":"cava","name":"cava"}],"depFailures":[],"conflicts":[]}"#;
        let v = parse(text).unwrap();
        assert_eq!(v.get("status").and_then(Json::as_str), Some("ok"));
        assert_eq!(v.get("behind").and_then(Json::as_i64), Some(3));
        assert_eq!(v.get("applied").and_then(Json::as_bool), Some(false));
        assert_eq!(v.get("restartNeeded").and_then(Json::as_bool), Some(true));
        assert_eq!(v.get("changelog").and_then(Json::as_arr).map(<[Json]>::len), Some(2));
        let dep = &v.get("missingDeps").and_then(Json::as_arr).unwrap()[0];
        assert_eq!(dep.get("id").and_then(Json::as_str), Some("cava"));
        assert_eq!(v.get("depFailures").and_then(Json::as_arr).map(<[Json]>::len), Some(0));
    }

    #[test]
    fn strings_with_escapes_and_unicode() {
        let v = parse(r#""a\"b\\c\ndA😀""#).unwrap();
        assert_eq!(v.as_str(), Some("a\"b\\c\ndA😀"));
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse("{} extra").is_err());
        assert!(parse("{\"a\":1,}").is_err());
    }

    #[test]
    fn manifest_shape() {
        let sha = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0";
        let v = parse(&format!(r#"{{"syncedSha":"{sha}","modules":{{"hypr/modules/binds.lua":"x"}}}}"#))
            .unwrap();
        assert_eq!(v.get("syncedSha").and_then(Json::as_str), Some(sha));
        assert!(v.get("modules").and_then(Json::as_arr).is_none());
    }
}
