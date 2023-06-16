#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    String(String),
    Identifier(String),
    Number(i32),
    LSquirrely,
    RSquirrely,
    LParen,
    RParen,
    LSquare,
    RSquare,
    At,
    Plus,
    PlusEq,
    Tack,
    TackEq,
    Star,
    StarEq,
    Slash,
    SlashEq,
    Equal,
    Colon,
    SemiColon,
    Comma,
    Dot,
    LCaret,
    RCaret,
    Woogly,
    UCaret,
    Bang,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut chars = source.chars().peekable();
    let mut token_stream = Vec::new();
    while let Some(char) = chars.next() {
        match char {
            '{' => token_stream.push(Token::LSquirrely),
            '}' => token_stream.push(Token::RSquirrely),
            '(' => token_stream.push(Token::LParen),
            ')' => token_stream.push(Token::RParen),
            '[' => token_stream.push(Token::LSquare),
            ']' => token_stream.push(Token::RSquare),
            '@' => token_stream.push(Token::At),
            '=' => token_stream.push(Token::Equal),
            '+' => token_stream.push(match chars.peek() {
                Some('=') => {
                    chars.next();
                    Token::PlusEq
                }
                _ => Token::Plus,
            }),
            '-' => token_stream.push(match chars.peek() {
                Some('=') => {
                    chars.next();
                    Token::TackEq
                }
                _ => Token::Tack,
            }),
            '*' => token_stream.push(match chars.peek() {
                Some('=') => {
                    chars.next();
                    Token::StarEq
                }
                _ => Token::Star,
            }),
            '/' => token_stream.push(match chars.peek() {
                Some('=') => {
                    chars.next();
                    Token::SlashEq
                }
                _ => Token::Slash,
            }),
            ':' => token_stream.push(Token::Colon),
            ';' => token_stream.push(Token::SemiColon),
            ',' => token_stream.push(Token::Comma),
            '.' => token_stream.push(Token::Dot),
            '<' => token_stream.push(Token::LCaret),
            '>' => token_stream.push(Token::RCaret),
            '^' => token_stream.push(Token::UCaret),
            '~' => token_stream.push(Token::Woogly),
            '!' => token_stream.push(Token::Bang),
            '"' => {
                let mut string_buf = String::new();
                while let Some(next) = chars.next() {
                    if next == '"' {
                        break;
                    }
                    string_buf.push(next);
                    if next == '\\' {
                        string_buf
                            .push(chars.next().ok_or(String::from("Unexpected end of file"))?);
                    }
                }
                token_stream.push(Token::String(string_buf));
            }
            '#' => {
                // consume a full-line comment
                for char in chars.by_ref() {
                    if char == '\n' {
                        break;
                    }
                }
            }
            _ => {
                // ignore whitespace
                if char.is_whitespace() {
                    continue;
                }
                // get an identifier / number
                if char.is_ascii_alphanumeric() || char == '_' {
                    let mut identifier_buf = String::from(char);
                    while let Some(next) = chars.peek() {
                        if next.is_ascii_alphanumeric() || *next == '_' {
                            identifier_buf
                                .push(chars.next().ok_or(String::from("Unexpected end of file"))?)
                        } else {
                            break;
                        }
                    }
                    token_stream.push(match identifier_buf.parse() {
                        Ok(int) => Token::Number(int),
                        Err(_) => Token::Identifier(identifier_buf),
                    });
                    continue;
                }
                // unexpected character
                return Err(format!("Unexpected character: `{char}`"));
            }
        }
    }
    Ok(token_stream)
}

#[cfg(test)]
mod tests {
    use crate::lexer::{tokenize, Token};

    #[test]
    fn lex_literals() {
        assert_eq!(
            tokenize("-20.0"),
            Ok(vec![
                Token::Tack,
                Token::Number(20),
                Token::Dot,
                Token::Number(0)
            ])
        );
        assert_eq!(
            tokenize("0-lol -0"),
            Ok(vec![
                Token::Number(0),
                Token::Tack,
                Token::Identifier(String::from("lol")),
                Token::Tack,
                Token::Number(0)
            ])
        )
    }
}
