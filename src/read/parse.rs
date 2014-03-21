#[deriving(Eq)]
pub enum Piece<'a> {
    String(&'a str),
    Whitespace,
    Argument(Argument<'a>),
}

#[deriving(Eq)]
pub struct Argument<'a> {
    position: Position<'a>,
    scan: ScanSpec<'a>,
}

#[deriving(Eq)]
pub enum Position<'a> {
    ArgumentNext,
    ArgumentIs(uint),
    ArgumentNamed(&'a str),
}

#[deriving(Eq)]
pub struct ScanSpec<'a> {
    fill: Option<char>,
    align: Alignment,
    flags: uint,
    width: Option<uint>,
    ty: &'a str,
}

#[deriving(Eq)]
pub enum Flags {
    FlagSignPlus,
    FlagSignMinus,
    FlagAlternate,
}

#[deriving(Eq)]
pub enum Alignment {
    AlignLeft,
    AlignRight,
    AlignCenter,
    AlignUnknown,
}

