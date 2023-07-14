use std::iter::Peekable;

use crate::types::Nbt;

enum Token {
    Star(u8),
    Underscore(u8),
    Char(char),
}

fn tokenize(src: &str) -> Vec<Token> {
    let mut chars = src.chars().peekable();
    let mut toks = Vec::new();
    while chars.peek().is_some() {
        inner_tokenize(&mut chars, &mut toks);
    }
    toks
}

fn inner_tokenize<T: Iterator<Item = char>>(chars: &mut Peekable<T>, toks: &mut Vec<Token>) {
    match chars.next() {
        Some('*') => {
            let mut star_count = 1;
            while chars.peek() == Some(&'*') {
                chars.next();
                star_count += 1;
            }
            toks.push(Token::Star(star_count));
        }
        Some('_') => {
            let mut under_count = 1;
            while chars.peek() == Some(&'_') {
                chars.next();
                under_count += 1;
            }
            toks.push(Token::Underscore(under_count));
        }
        Some(c) => toks.push(Token::Char(c)),
        None => {}
    }
}

fn interpret(src: Vec<Token>) -> Nbt {
    let mut toks = src.into_iter().peekable();
    todo!()
}

fn inner_interpret<T: Iterator<Item = Token>>(toks: &mut Peekable<T>) -> Nbt {
    todo!()
}
