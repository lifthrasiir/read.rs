use std::io::{IoResult, standard_error, InvalidInput};
use buffer::LookaheadBuffer;
pub use parse::{Flags, FlagSignPlus, FlagSignMinus, FlagAlternate};
pub use parse::{Alignment, AlignLeft, AlignRight, AlignCenter, AlignUnknown};

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
        match self.align {
            AlignLeft | AlignCenter => self.skip_pad(),
            _ => Ok(0)
        }
    }

    pub fn skip_postpad(&mut self) -> IoResult<uint> {
        match self.align {
            AlignRight | AlignCenter => self.skip_pad(),
            _ => Ok(0)
        }
    }

    pub fn trim_postpad<'a>(&self, buf: &'a str) -> &'a str {
        match self.align {
            AlignRight | AlignCenter => match self.fill {
                Some(ch) => buf.trim_right_chars(&ch),
                None => buf.trim_right(),
            },
            _ => buf
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

// XXX these should really be `Trait::<for T>::scan(s)` once it gets supported
macro_rules! define_function_aliases {
    ($($name:ident for $Trait:ident;)*) => {
        pub struct Scan;
        impl<'a> Scan {
            $(
                pub fn $name<T:$Trait<'a>>(s: &'a mut Scanner<'a>) -> IoResult<T> {
                    match try!($Trait::scan(s)) {
                        Some(v) => Ok(v),
                        None => Err(standard_error(InvalidInput))
                    }
                }
            )*
        }
    }
}

define_function_aliases! {
    for_read     for Read;
    for_integer  for Integer;
    for_signed   for Signed;
    for_unsigned for Unsigned;
    for_char     for Char;
    for_octal    for Octal;
    for_hex      for Hex;
    for_string   for String;
    for_binary   for Binary;
    for_float    for Float;
    for_exp      for Exp;
}

mod impls {
    use super::*;
    use std::{char, str};
    use std::from_str::FromStr;
    use std::io::IoResult;

    pub fn scan_signed_digits<'a, T: FromStr>(s: &'a mut Scanner) -> IoResult<Option<T>> {
        fn scan<'a>(s: &'a mut Scanner, mandatory_sign: bool) -> IoResult<Option<&'a [u8]>> {
            enum State {
                ExpectSignOrDigit = 0,   // @ ('+' | '-')?   ('0'..'9')+
                ExpectSign        = 2,   // @ ('+' | '-')    ('0'..'9')+
                ExpectDigit       = 4,   //   ('+' | '-')? @ ('0'..'9')+
                ExpectMoreDigits  = 6+1, //   ('+' | '-')?   ('0'..'9') @ ('0'..'9')*
            }

            let mut i = 0;
            let mut state = if mandatory_sign {ExpectSign} else {ExpectSignOrDigit};
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
            Ok(Some(buf.slice_to(i)))
        }

        try!(s.skip_prepad());

        let mandatory_sign = ((s.flags >> FlagSignPlus as uint) & 1) == 1;
        let (result, i) = match try!(scan(s, mandatory_sign)) {
            Some(buf) => (from_str(str::from_utf8(buf).unwrap()), buf.len()),
            None => { return Ok(None); }
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

    impl<'a> String<'a> for ~str {
        fn scan(s: &'a mut Scanner<'a>) -> IoResult<Option<~str>> {
            fn drop_incomplete_utf8_suffix(buf: &[u8]) -> (uint, uint) {
                let mut i = buf.len();
                while i > 0 {
                    i -= 1;
                    let width = str::utf8_char_width(buf[i]);
                    if width > 1 { // exclude this byte
                        return (i, i + width);
                    } else if width == 1 { // include this byte
                        return (i + 1, i + 2);
                    }
                }
                (0, 1)
            }

            try!(s.skip_prepad());

            let non_empty = ((s.flags >> FlagSignPlus as uint) & 1) == 1;
            let end_at_newline = ((s.flags >> FlagAlternate as uint) & 1) == 1;

            let mut i = 0;
            let mut request = 1;
            'reading: loop {
                let buf = try!(s.buf.fill_request(request));
                if buf.len() < request { break; }
                let (i_, request_) = drop_incomplete_utf8_suffix(buf);
                assert!(request_ > buf.len());
                let new = match str::from_utf8(buf.slice(i, i_)) {
                    Some(buf) => buf,
                    None => { return Ok(None); } // XXX may raise a premature error
                };
                if end_at_newline {
                    for (j, ch) in new.char_indices() {
                        if ch == '\r' || ch == '\n' { i += j; break 'reading; }
                    }
                } else {
                    for (j, ch) in new.char_indices() {
                        if char::is_whitespace(ch) { i += j; break 'reading; }
                    }
                }
                i = i_;
                request = request_;
            }

            if non_empty && i == 0 { return Ok(None); }

            let ret;
            {
                let buf = try!(s.buf.fill_request(i));
                assert!(buf.len() >= i);
                ret = str::from_utf8(buf.slice_to(i)).unwrap().to_owned();
            }
            s.buf.consume(i);

            // XXX slow
            let ret = s.trim_postpad(ret).to_owned();
            Ok(Some(ret))
        }
    }
}

