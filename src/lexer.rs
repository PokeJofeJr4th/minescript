use std::iter::Peekable;

use crate::types::{Token, SResult};

macro_rules! multi_character_pattern {
    ($chars:ident $just:expr; {$($char:expr => $eq:expr),*}) => {
        match $chars.peek() {
            $(Some($char) => {
                $chars.next();
                $eq
            })*
            _ => $just,
        }
    };
}

pub fn tokenize(source: &str) -> SResult<Vec<Token>> {
    let mut chars = source.chars().peekable();
    let mut token_stream = Vec::new();
    while chars.peek().is_some() {
        if let Some(tok) = inner_tokenize(&mut chars)? {
            token_stream.push(tok);
        }
    }
    Ok(token_stream)
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]
fn inner_tokenize<T: Iterator<Item = char>>(
    chars: &mut Peekable<T>,
) -> SResult<Option<Token>> {
    let Some(char) = chars.next() else {
        return Err("Unexpected end of file".into())
    };
    Ok(Some(match char {
        '{' => Token::LSquirrely,
        '}' => Token::RSquirrely,
        '(' => Token::LParen,
        ')' => Token::RParen,
        '[' => Token::LSquare,
        ']' => Token::RSquare,
        '@' => Token::At,
        '=' => multi_character_pattern!(chars Token::Equal; {'>' => Token::FatArrow}),
        '+' => multi_character_pattern!(chars Token::Plus; {'=' => Token::PlusEq, '+' => Token::PlusPlus}),
        '-' => {
            multi_character_pattern!(chars Token::Tack; {'=' => Token::TackEq, '-' => Token::TackTack, '>' => Token::Arrow})
        }
        '*' => multi_character_pattern!(chars Token::Star; {'=' => Token::StarEq}),
        '/' => multi_character_pattern!(chars Token::Slash; {'=' => Token::SlashEq}),
        '%' => multi_character_pattern!(chars Token::Percent; {'=' => Token::PercEq}),
        '!' => multi_character_pattern!(chars Token::Bang; {'=' => Token::BangEq}),
        '<' => multi_character_pattern!(chars Token::LCaret; {'=' => Token::LCaretEq}),
        '>' => multi_character_pattern!(chars Token::RCaret; {'=' => Token::RCaretEq}),
        '.' => multi_character_pattern!(chars Token::Dot; {'.' => Token::Doot}),
        ':' => Token::Colon,
        ';' => Token::SemiColon,
        ',' => Token::Comma,
        '^' => Token::UCaret,
        '~' => Token::Woogly,
        '"' => {
            let mut string_buf = String::new();
            while let Some(next) = chars.next() {
                if next == '"' {
                    break;
                }
                string_buf.push(next);
                if next == '\\' {
                    string_buf.push(
                        chars
                            .next()
                            .ok_or_else(|| String::from("Unexpected end of file"))?,
                    );
                }
            }
            Token::String(string_buf.into())
        }
        '#' => {
            // consume a full-line comment
            for char in chars.by_ref() {
                if char == '\n' {
                    return Ok(None);
                }
            }
            return Ok(None);
        }
        _ => {
            // ignore whitespace
            if char.is_whitespace() {
                return Ok(None);
            }
            // get an identifier / number / range
            if char.is_ascii_alphanumeric() || char == '_' {
                let mut identifier_buf = String::from(char);
                while let Some(next) = chars.peek() {
                    if next.is_ascii_alphanumeric() || *next == '_' {
                        identifier_buf.push(
                            chars
                                .next()
                                .ok_or_else(|| String::from("Unexpected end of file"))?,
                        );
                    } else {
                        break;
                    }
                }
                return Ok(Some(match identifier_buf.parse() {
                    Ok(int) => match chars.peek() {
                        Some('.') => {
                            chars.next();
                            let mut decimal_buf = String::new();
                            let doot: bool = match chars.peek() {
                                Some('.') => {
                                    chars.next();
                                    true
                                }
                                _ => false,
                            };
                            while let Some(next) = chars.peek() {
                                if next.is_ascii_digit() {
                                    decimal_buf.push(*next);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            if doot {
                                Token::Range(Some(int), decimal_buf.parse::<i32>().ok())
                            } else {
                                Token::Float(
                                    int as f32
                                        + decimal_buf.parse::<f32>().map_err(|_| {
                                            String::from("Expected number after `.`")
                                        })? / 10.0f32.powi(decimal_buf.len() as i32),
                                )
                            }
                        }
                        _ => Token::Integer(int),
                    },
                    Err(_) => Token::Identifier(identifier_buf.into()),
                }));
            }
            // unexpected character
            return Err(format!("Unexpected character: `{char}`"));
        }
    }))
}

#[cfg(test)]
mod tests {
    use crate::lexer::{tokenize, Token};

    #[test]
    fn lex_literals() {
        assert_eq!(
            tokenize("-20.02"),
            Ok(vec![Token::Tack, Token::Float(20.02),])
        );
        assert_eq!(
            tokenize("0-lol -0"),
            Ok(vec![
                Token::Integer(0),
                Token::Tack,
                Token::Identifier(String::from("lol").into()),
                Token::Tack,
                Token::Integer(0)
            ])
        );
    }

    #[test]
    fn lex_for_loop() {
        assert_eq!(tokenize("1..10"), Ok(vec![Token::Range(Some(1), Some(10))]));
        assert_eq!(
            tokenize("for x in 1..10 {@function \"tick\"}"),
            Ok(vec![
                Token::Identifier("for".into()),
                Token::Identifier("x".into()),
                Token::Identifier("in".into()),
                Token::Range(Some(1), Some(10)),
                Token::LSquirrely,
                Token::At,
                Token::Identifier("function".into()),
                Token::String("tick".into()),
                Token::RSquirrely
            ])
        );
    }
}
