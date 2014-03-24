use std::char;
use std::str::CharRange;

#[deriving(Eq,Show)]
pub enum Piece<'a> {
    String(&'a str),
    Whitespace,
    Argument(Argument<'a>),
}

#[deriving(Eq,Show)]
pub struct Argument<'a> {
    position: Position<'a>,
    scan: ScanSpec<'a>,
}

#[deriving(Eq,Show)]
pub enum Position<'a> {
    ArgumentNext,
    ArgumentIs(uint),
    ArgumentNamed(&'a str),
    ArgumentSuppress,
}

#[deriving(Eq,Show)]
pub struct ScanSpec<'a> {
    fill: Option<char>,
    align: Alignment,
    flags: uint,
    width: Option<uint>,
    ty: &'a str,
}

#[deriving(Eq,Show)]
pub enum Flags {
    FlagSignPlus,
    FlagSignMinus,
    FlagAlternate,
}

#[deriving(Eq,Show)]
pub enum Alignment {
    AlignLeft,
    AlignRight,
    AlignCenter,
    AlignUnknown,
}

fn parse_uint<'a>(s: &'a str) -> Option<(uint, &'a str)> {
    let mut last = s.len();
    for (i, c) in s.char_indices() {
        if !('0' <= c && c <= '9') {
            last = i;
            break;
        }
    }

    if last == 0 { return None; }
    from_str::<uint>(s.slice_to(last)).map(|v| (v, s.slice_from(last)))
}

fn parse_ident<'a>(s: &'a str) -> Option<(&'a str, &'a str)> {
    if s.is_empty() { return None; }

    let CharRange { ch, next } = s.char_range_at(0);
    if !char::is_XID_start(ch) { return None; }

    let mut i = next;
    let len = s.len();
    while i < len {
        let CharRange { ch, next } = s.char_range_at(i);
        if !char::is_XID_continue(ch) { break; }
        i = next;
    }

    Some((s.slice_to(i), s.slice_from(i)))
}

// assumes that `s` does not contain the initial `{`
fn parse_argument<'a>(s: &'a str) -> Result<(Argument<'a>, &'a str), ~str> {
    let s = s.trim_left();
    if s.is_empty() { return Err(~"a premature end of argument"); }

    // <scan> ::= '{' <name>? ...
    // <name> ::= INTEGER | IDENT | '*'
    let (pos, s) = match s.char_at(0) {
        '*' => (Some(ArgumentSuppress), s.slice_from(1)),
        '0'..'9' => match parse_uint(s) {
            Some((v, s)) => (Some(ArgumentIs(v)), s),
            None => (None, s),
        },
        _ => match parse_ident(s) {
            Some((id, s)) => (Some(ArgumentNamed(id)), s),
            None => (None, s),
        },
    };

    // <scan> ::= ... (':' <spec>)? '}'
    let idx = s.find('}'); // find the matching `}` first and verify it later
    if idx.is_none() { return Err(~"a premature end of argument"); }
    let idx = idx.unwrap();
    let (spec, remaining) = (s.slice_to(idx), s.slice_from(idx + 1));

    // <scan-body> ::= ... (':' <spec>)?
    let scan;
    if spec.starts_with(":") {
        let spec = spec.slice_from(1).trim_left(); // strip `:`

        // search for the potential padding character
        let (c1, s1) = spec.slice_shift_char();
        let s1 = s1.trim_left();
        let (c2, s2) = s1.slice_shift_char();
        let s2 = s2.trim_left();
        let (fill, align, s) = match (c1, c2) {
            (Some(fill), Some('<')) => (Some(fill), AlignLeft, s2),
            (Some(fill), Some('^')) => (Some(fill), AlignCenter, s2),
            (Some(fill), Some('>')) => (Some(fill), AlignRight, s2),
            (Some('<'), _) => (None, AlignLeft, s1),
            (Some('^'), _) => (None, AlignCenter, s1),
            (Some('>'), _) => (None, AlignRight, s1),
            (_, _) => (None, AlignUnknown, spec),
        };

        // parse one-character flags
        let mut flags = 0;
        let mut s = s;
        if s.starts_with("+") {
            flags |= 1 << FlagSignPlus as uint;
            s = s.slice_from(1).trim_left();
        } else if s.starts_with("-") {
            flags |= 1 << FlagSignMinus as uint;
            s = s.slice_from(1).trim_left();
        }
        if s.starts_with("#") {
            flags |= 1 << FlagAlternate as uint;
            s = s.slice_from(1).trim_left();
        }

        // parse the optional width
        let s = s.trim_left();
        let (width, s) = match parse_uint(s) {
            Some((width, s)) => (Some(width), s.trim_left()),
            None => (None, s),
        };

        // parse the type name and verify if it is the end of argument
        let s = s.trim_left();
        let (ty, s) = match parse_ident(s) {
            Some((id, s)) => (id, s),
            None => ("", s),
        };

        let s = s.trim();
        if !s.is_empty() {
            return Err(format!("invalid scan spec: {}", spec.trim()));
        }
        scan = ScanSpec { fill: fill, align: align, flags: flags, width: width, ty: ty };
    } else {
        let spec = spec.trim();
        if !spec.is_empty() {
            return Err(format!("unexpected string after the position: {}", spec));
        }
        scan = ScanSpec { fill: None, align: AlignUnknown, flags: 0, width: None, ty: "" };
    }
    Ok((Argument { position: pos.unwrap_or(ArgumentNext), scan: scan }, remaining))
}

pub fn parse_fmt<'a>(mut s: &'a str) -> Result<Vec<Piece<'a>>, ~str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    loop {
        let next = match s.slice_from(start).find(&['\\', '{', '}', ' ', '\t', '\r', '\n']) {
            Some(next) => next + start,
            None => { break; }
        };
        if next > 0 {
            pieces.push(String(s.slice_to(next)));
        }
        s = s.slice_from(next);
        let (c, s_) = s.slice_shift_char();
        s = s_;
        start = 0;
        match c {
            Some('\\') => {
                // skip this letter and continue to the literals
                if s.is_empty() {
                    return Err(~"an unfinished escape sequence");
                }
                start = s.char_range_at(0).next;
            }
            Some('{') => {
                let (arg, s_) = try!(parse_argument(s));
                pieces.push(Argument(arg));
                s = s_;
            }
            Some('}') => {
                return Err(~"unexpected `}` in the literal");
            }
            Some(_) => { // whitespaces
                pieces.push(Whitespace);
                s = s.trim_left();
            }
            None => unreachable!()
        }
    }
    if !s.is_empty() {
        pieces.push(String(s));
    }
    Ok(pieces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_and_whitespace() {
        assert!(parse_fmt("") == Ok(vec!()));
        assert!(parse_fmt("a") == Ok(vec!(String("a"))));
        assert!(parse_fmt("asdf") == Ok(vec!(String("asdf"))));
        assert!(parse_fmt("asdf\\foo") == Ok(vec!(String("asdf"), String("foo"))));

        assert!(parse_fmt("a b c") == Ok(vec!(String("a"), Whitespace, String("b"),
                                              Whitespace, String("c"))));
        assert!(parse_fmt("a\t\tb\r\nc") == Ok(vec!(String("a"), Whitespace, String("b"),
                                                    Whitespace, String("c"))));
        assert!(parse_fmt("a\\ b\\ c") == Ok(vec!(String("a"), String(" b"), String(" c"))));
        assert!(parse_fmt(" x ") == Ok(vec!(Whitespace, String("x"), Whitespace)));

        assert!(parse_fmt("\\x") == Ok(vec!(String("x"))));
        assert!(parse_fmt("\\{\\}") == Ok(vec!(String("{"), String("}"))));
        assert!(parse_fmt("\\").is_err());
    }

    #[test]
    fn test_literal_and_spec() {
        let placeholder = Argument(Argument {
            position: ArgumentNext,
            scan: ScanSpec { fill: None, align: AlignUnknown, flags: 0, width: None, ty: "" }
        });
        assert!(parse_fmt("{}") == Ok(vec!(placeholder)));
        assert!(parse_fmt("a{}b") == Ok(vec!(String("a"), placeholder, String("b"))));
        assert!(parse_fmt(" {} ") == Ok(vec!(Whitespace, placeholder, Whitespace)));
        assert!(parse_fmt("\\\\{}\\\\") == Ok(vec!(String("\\"), placeholder, String("\\"))));
        assert!(parse_fmt("\\{}").is_err());
    }

    #[test]
    fn test_incomplete_spec() {
        assert!(parse_fmt("{").is_err());
        assert!(parse_fmt("}").is_err());
        assert!(parse_fmt("{}}").is_err());
        assert!(parse_fmt("{:}^}").is_err()); // XXX different from `format!`
    }

    #[test]
    fn test_spec_position() {
        let arg_with_pos = |pos| Argument(Argument {
            position: pos,
            scan: ScanSpec { fill: None, align: AlignUnknown, flags: 0, width: None, ty: "" }
        });
        assert!(parse_fmt("{}") == Ok(vec!(arg_with_pos(ArgumentNext))));
        assert!(parse_fmt("{a}") == Ok(vec!(arg_with_pos(ArgumentNamed("a")))));
        assert!(parse_fmt("{名前}") == Ok(vec!(arg_with_pos(ArgumentNamed("名前")))));
        assert!(parse_fmt("{  名前  }") == Ok(vec!(arg_with_pos(ArgumentNamed("名前")))));
        assert!(parse_fmt("{0}") == Ok(vec!(arg_with_pos(ArgumentIs(0)))));
        assert!(parse_fmt("{013}") == Ok(vec!(arg_with_pos(ArgumentIs(13)))));
        assert!(parse_fmt("{{}}").is_err());
        assert!(parse_fmt("{/}").is_err());
        assert!(parse_fmt("{-7}").is_err());
    }

    #[test]
    fn test_spec_with_simple_type() {
        let arg_with_ty = |ty| Argument(Argument {
            position: ArgumentNext,
            scan: ScanSpec { fill: None, align: AlignUnknown, flags: 0, width: None, ty: ty }
        });
        assert!(parse_fmt("{}") == Ok(vec!(arg_with_ty(""))));
        assert!(parse_fmt("{:}") == Ok(vec!(arg_with_ty(""))));
        assert!(parse_fmt("{:a}") == Ok(vec!(arg_with_ty("a"))));
        assert!(parse_fmt("{ : b }") == Ok(vec!(arg_with_ty("b"))));
        assert!(parse_fmt("{:いろいろ}") == Ok(vec!(arg_with_ty("いろいろ"))));
    }

    #[test]
    fn test_spec_with_flags() {
        let arg_with_flags = |flags| Argument(Argument {
            position: ArgumentNext,
            scan: ScanSpec { fill: None, align: AlignUnknown, flags: flags, width: None, ty: "foo" }
        });
        let plus_mask = 1 << FlagSignPlus as uint;
        let minus_mask = 1 << FlagSignMinus as uint;
        let alternate_mask = 1 << FlagAlternate as uint;
        assert!(parse_fmt("{:foo}") == Ok(vec!(arg_with_flags(0))));
        assert!(parse_fmt("{:+foo}") == Ok(vec!(arg_with_flags(plus_mask))));
        assert!(parse_fmt("{:-foo}") == Ok(vec!(arg_with_flags(minus_mask))));
        assert!(parse_fmt("{:#foo}") == Ok(vec!(arg_with_flags(alternate_mask))));
        assert!(parse_fmt("{:+#foo}") == Ok(vec!(arg_with_flags(plus_mask | alternate_mask))));
        assert!(parse_fmt("{:-#foo}") == Ok(vec!(arg_with_flags(minus_mask | alternate_mask))));
        assert!(parse_fmt("{:#+foo}").is_err());
        assert!(parse_fmt("{:#-foo}").is_err());
        assert!(parse_fmt("{:+-foo}").is_err());
        assert!(parse_fmt("{:-+foo}").is_err());
        assert!(parse_fmt("{:++foo}").is_err());
        assert!(parse_fmt("{:--foo}").is_err());
        assert!(parse_fmt("{:##foo}").is_err());
    }

    #[test]
    fn test_spec_with_alignment_and_fill() {
        let arg_with_pad = |align, fill| Argument(Argument {
            position: ArgumentNext,
            scan: ScanSpec { fill: fill, align: align, flags: 0, width: None, ty: "foo" }
        });
        assert!(parse_fmt("{:foo}") == Ok(vec!(arg_with_pad(AlignUnknown, None))));
        assert!(parse_fmt("{:>foo}") == Ok(vec!(arg_with_pad(AlignRight, None))));
        assert!(parse_fmt("{: > foo}") == Ok(vec!(arg_with_pad(AlignRight, None))));
        assert!(parse_fmt("{:_>foo}") == Ok(vec!(arg_with_pad(AlignRight, Some('_')))));
        assert!(parse_fmt("{:9>foo}") == Ok(vec!(arg_with_pad(AlignRight, Some('9')))));
        assert!(parse_fmt("{:>>foo}") == Ok(vec!(arg_with_pad(AlignRight, Some('>')))));
        assert!(parse_fmt("{:>>>foo}").is_err());
        assert!(parse_fmt("{:^foo}") == Ok(vec!(arg_with_pad(AlignCenter, None))));
        assert!(parse_fmt("{: ^ foo}") == Ok(vec!(arg_with_pad(AlignCenter, None))));
        assert!(parse_fmt("{:_^foo}") == Ok(vec!(arg_with_pad(AlignCenter, Some('_')))));
        assert!(parse_fmt("{:9^foo}") == Ok(vec!(arg_with_pad(AlignCenter, Some('9')))));
        assert!(parse_fmt("{:^^foo}") == Ok(vec!(arg_with_pad(AlignCenter, Some('^')))));
        assert!(parse_fmt("{:^^^foo}").is_err());
        assert!(parse_fmt("{:<foo}") == Ok(vec!(arg_with_pad(AlignLeft, None))));
        assert!(parse_fmt("{: < foo}") == Ok(vec!(arg_with_pad(AlignLeft, None))));
        assert!(parse_fmt("{:_<foo}") == Ok(vec!(arg_with_pad(AlignLeft, Some('_')))));
        assert!(parse_fmt("{:9<foo}") == Ok(vec!(arg_with_pad(AlignLeft, Some('9')))));
        assert!(parse_fmt("{:<<foo}") == Ok(vec!(arg_with_pad(AlignLeft, Some('<')))));
        assert!(parse_fmt("{:<<<foo}").is_err());
    }

    #[test]
    fn test_spec_with_width() {
        let arg_with_width = |width| Argument(Argument {
            position: ArgumentNext,
            scan: ScanSpec { fill: None, align: AlignUnknown, flags: 0, width: width, ty: "foo" }
        });
        assert!(parse_fmt("{:foo}") == Ok(vec!(arg_with_width(None))));
        assert!(parse_fmt("{:0foo}") == Ok(vec!(arg_with_width(Some(0)))));
        assert!(parse_fmt("{:042foo}") == Ok(vec!(arg_with_width(Some(42)))));
        assert!(parse_fmt("{: 42 foo}") == Ok(vec!(arg_with_width(Some(42)))));
        assert!(parse_fmt("{:99999999999999999999999foo}").is_err());
        assert!(parse_fmt("{: 4 2 foo}").is_err());
    }
}

