use super::RStr;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    String(RStr),
    Identifier(RStr),
    Integer(i32),
    Range(Option<i32>, Option<i32>),
    Float(f32),
    LSquirrely,
    RSquirrely,
    LParen,
    RParen,
    LSquare,
    RSquare,
    At,
    Equal,
    Plus,
    PlusPlus,
    PlusEq,
    Tack,
    TackTack,
    TackEq,
    Star,
    StarEq,
    Slash,
    SlashEq,
    Percent,
    PercEq,
    Bang,
    BangEq,
    LCaret,
    LCaretEq,
    RCaret,
    RCaretEq,
    Colon,
    SemiColon,
    Comma,
    Dot,
    Doot,
    Woogly,
    UCaret,
    Arrow,
    FatArrow,
}
