use collections::HashMap;

use syntax::ast::{Name, SpannedIdent, TokenTree, Expr, Ty};
use syntax::codemap::{Span, Spanned};
use syntax::ext::base::*;
use syntax::parse;
use syntax::parse::token;

use parse::parse_fmt;

struct Args {
    extra: @Expr,
    fmtstr: @Expr,
    ignore_case: bool,
    named: HashMap<~str,(SpannedIdent,@Ty)>,
    named_order: Vec<SpannedIdent>,
}

fn parse_args(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Option<Args> {
    let mut names = HashMap::<~str,(SpannedIdent,@Ty)>::new();
    let mut order = Vec::new();

    let mut p = parse::new_parser_from_tts(
        cx.parse_sess(), cx.cfg(), tts.iter().map(|x| (*x).clone()).collect());

    // <macro-args> ::= <expr> ',' ...
    let extra = p.parse_expr();
    if !p.eat(&token::COMMA) {
        cx.span_err(sp, "expected token: `,`");
        return None;
    }

    // ... <expr> ...
    let fmtstr = p.parse_expr();

    // ... (<ident>)? ...
    let mut ignore_case = false;
    match p.token {
        token::IDENT(ident, false) => {
            let interned_name = token::get_ident(ident);
            let name = interned_name.get();
            for ch in name.chars() {
                let mut dup = false;
                match ch {
                    'i' => {
                        if ignore_case { dup = true; }
                        ignore_case = true;
                    }
                    _ => {
                        cx.span_err(p.span, format!("unrecognized modifier `{}`", ch));
                        return None;
                    }
                }
                if dup {
                    cx.span_err(p.span, format!("duplicated modifier `{}`", ch));
                    return None;
                }
            }

            p.bump();
        }
        _ => {}
    }

    // ... <types>
    // <types> ::= (empty)
    while p.token != token::EOF {
        // <types> ::= ','
        if !p.eat(&token::COMMA) {
            cx.span_err(sp, "expected token: `,`");
            return None;
        }
        if p.token == token::EOF { break } // accept trailing commas

        // <types> ::= ',' <ident> ':' 
        let (ident, identsp) = match p.token {
            token::IDENT(i, _) => {
                p.bump();
                (i, p.last_span)
            }
            _ => {
                cx.span_err(p.span, format!("expected ident for argument, \
                                             but found `{}`", p.this_token_to_str()));
                return None;
            }
        };
        let interned_name = token::get_ident(ident);
        let name = interned_name.get();
        p.expect(&token::COLON);
        let ty = p.parse_ty(false);
        match names.find_equiv(&name) {
            None => {}
            Some(&(previd, _)) => {
                cx.span_err(identsp, format!("duplicate argument named `{}`", name));
                cx.parse_sess.span_diagnostic.span_note(previd.span, "previously here");
                continue;
            }
        }
        let spanned = Spanned { node: ident, span: identsp };
        order.push(spanned);
        names.insert(name.to_str(), (spanned, ty));
    }

    Some(Args { extra: extra, fmtstr: fmtstr, ignore_case: ignore_case,
                named: names, named_order: order })
}

fn expand(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> MacResult {
    let args = match parse_args(cx, sp, tts) {
        Some(args) => args,
        None => return MRExpr(MacResult::raw_dummy_expr(sp))
    };

    let fmt = match expr_to_str(cx, args.fmtstr,
                                "format argument must be a string literal.") {
        Some((fmt, _)) => fmt,
        None => return MRExpr(MacResult::raw_dummy_expr(sp))
    };

    let pieces = match parse_fmt(fmt.get()) {
        Ok(pieces) => pieces,
        Err(err) => {
            cx.span_err(args.fmtstr.span, err);
            return MRExpr(MacResult::raw_dummy_expr(sp));
        }
    };

    /*
    // start with the final result.
    quote_expr!(cx, ::std::result::Ok(
    */

    println!("{:?}", pieces.as_slice());

    MRExpr(quote_expr!(cx, 1i))
}

#[macro_registrar]
pub fn macro_registrar(register: |Name, SyntaxExtension|) {
    register(token::intern(&"lex"),
             NormalTT(~BasicMacroExpander { expander: expand, span: None }, None));
}
