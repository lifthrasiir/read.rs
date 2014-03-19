use std::{str, from_str};
use std::io::{Buffer, IoResult};
use buffer::LookaheadBuffer;

pub enum Flags {
    FlagSignPlus,
    FlagSignMinus,
    FlagAlternate,
    FlagSignAwareZeroPad,
}

pub enum Alignment {
    AlignLeft,
    AlignRight,
    AlignCenter,
    AlignUnknown,
}

pub struct Scanner<'a> {
    flags: uint, // packed
    fill: Option<char>, // None for every whitespace
    align: Alignment,
    width: Option<uint>,

    buf: LookaheadBuffer<'a>,
}

impl<'a> Scanner<'a> {
    fn skip_pad(&mut self) -> IoResult<uint> {
        match self.fill {
            Some(ch) => self.buf.read_pad_char(ch),
            None => self.buf.read_pad_byte_if(|ch| ch == ' ' as u8 ||
                                                   ch == '\t' as u8 ||
                                                   ch == '\r' as u8 ||
                                                   ch == '\n' as u8),
        }
    }

    pub fn skip_prepad(&mut self) -> IoResult<uint> {
        if self.flags & ((1 << AlignLeft as uint) | (1 << AlignCenter as uint)) != 0 {
            self.skip_pad()
        } else {
            Ok(0)
        }
    }

    pub fn skip_postpad(&mut self) -> IoResult<uint> {
        if self.flags & ((1 << AlignRight as uint) | (1 << AlignCenter as uint)) != 0 {
            self.skip_pad()
        } else {
            Ok(0)
        }
    }

    pub fn trim_postpad<'a>(&self, buf: &'a str) -> &'a str {
        if self.flags & ((1 << AlignRight as uint) | (1 << AlignCenter as uint)) != 0 {
            match self.fill {
                Some(ch) => buf.trim_right_chars(&ch),
                None => buf.trim_right(),
            }
        } else {
            buf
        }
    }
}

pub trait Read<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub trait Integer<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub trait Signed<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub trait Unsigned<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub trait Char<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub trait Octal<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub trait Hex<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub trait String<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub trait Binary<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub trait Float<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub trait Exp<'a> {
    fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<Self>>;
}

pub fn scan_signed_digits<'a, T: from_str::FromStr>(s: &'a mut Scanner) -> IoResult<Option<T>> {
    try!(s.skip_prepad());

    let mut i = 0;
    let result = {
        enum State {
            ExpectSignOrDigit = 0,   // @ ('+' | '-')?   ('0'..'9')+
            ExpectDigit       = 2,   //   ('+' | '-')? @ ('0'..'9')+
            ExpectMoreDigits  = 4+1, //   ('+' | '-')?   ('0'..'9') @ ('0'..'9')*
        }

        let mut state = ExpectSignOrDigit;
        'reading: loop {
            let buf = try!(s.buf.fill_request(i + 1));
            if buf.len() <= i { break; }
            for (j, &ch) in buf.slice_from(i).iter().enumerate() {
                state = match (state, ch as char) {
                    (ExpectSignOrDigit, '+')      => ExpectDigit,
                    (ExpectSignOrDigit, '-')      => ExpectDigit,
                    (ExpectSignOrDigit, '0'..'9') => ExpectMoreDigits,

                    (ExpectDigit,       '0'..'9') => ExpectMoreDigits,

                    (ExpectMoreDigits,  '0'..'9') => ExpectMoreDigits,

                    (_, _) => { i += j; break 'reading; }
                };
            }
            i = buf.len();
        }
        if (state as uint & 1) == 0 { return Ok(None); }

        let buf = try!(s.buf.fill_request(i));
        assert!(buf.len() >= i);
        let buf = buf.slice_to(i);
        let buf = str::from_utf8(buf).unwrap();
        from_str(buf)
    };

    s.buf.consume(i);
    try!(s.skip_postpad());
    Ok(result)
}

macro_rules! delegate_impls {
    ($($trait_:ident for $ty:ty => $f:expr;)*) => (
        $(
            impl<'a> $trait_<'a> for $ty {
                fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<$ty>> { $f(s) }
            }
        )*
    )
}

delegate_impls! {
    Signed   for int  => scan_signed_digits;
    Signed   for i8   => scan_signed_digits;
    Signed   for i16  => scan_signed_digits;
    Signed   for i32  => scan_signed_digits;
    Signed   for i64  => scan_signed_digits;

    Unsigned for uint => scan_signed_digits;
    Unsigned for u8   => scan_signed_digits;
    Unsigned for u16  => scan_signed_digits;
    Unsigned for u32  => scan_signed_digits;
    Unsigned for u64  => scan_signed_digits;
}

