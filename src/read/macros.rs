use std::vec_ng::Vec;
use collections::HashMap;

use syntax::ast::{Name, TokenTree, Expr, Ty};
use syntax::codemap::Span;
use syntax::ext::base::*;
use syntax::parse;
use syntax::parse::token;

struct Args {
    extra: @Expr,
    fmtstr: @Expr,
    ignore_case: bool,
    unnamed: Vec<@Ty>,
    named: HashMap<~str,@Ty>,
    named_order: Vec<~str>,
}

fn parse_args(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Option<Args> {
    let mut args = Vec::new();
    let mut names = HashMap::<~str, @Ty>::new();
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
    let mut named = false;
    // <types> ::= (empty)
    while p.token != token::EOF {
        // <types> ::= ','
        if !p.eat(&token::COMMA) {
            cx.span_err(sp, "expected token: `,`");
            return None;
        }
        if p.token == token::EOF { break } // accept trailing commas

        // <types> ::= ',' <ident> ':' 
        if named || (token::is_ident(&p.token) && p.look_ahead(1, |t| *t == token::COLON)) {
            named = true;
            let ident = match p.token {
                token::IDENT(i, _) => {
                    p.bump();
                    i
                }
                _ if named => {
                    cx.span_err(p.span, "expected ident, positional arguments \
                                         cannot follow named arguments");
                    return None;
                }
                _ => {
                    cx.span_err(p.span, format!("expected ident for named argument, but found `{}`",
                                                p.this_token_to_str()));
                    return None;
                }
            };
            let interned_name = token::get_ident(ident);
            let name = interned_name.get();
            p.expect(&token::COLON);
            let e = p.parse_ty(false);
            match names.find_equiv(&name) {
                None => {}
                Some(prev) => {
                    cx.span_err(e.span, format!("duplicate argument named `{}`", name));
                    cx.parse_sess.span_diagnostic.span_note(prev.span, "previously here");
                    continue;
                }
            }
            order.push(name.to_str());
            names.insert(name.to_str(), e);
        } else {
            args.push(p.parse_ty(false));
        }
    }

    Some(Args { extra: extra, fmtstr: fmtstr, ignore_case: ignore_case,
                unnamed: args, named: names, named_order: order })
}

fn expand(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> MacResult {
    let args = match parse_args(cx, sp, tts) {
        Some(args) => args,
        None => return MRExpr(MacResult::raw_dummy_expr(sp))
    };

    println!("{:?}", args);

    MRExpr(quote_expr!(cx, 1i))
}

#[macro_registrar]
pub fn macro_registrar(register: |Name, SyntaxExtension|) {
    register(token::intern(&"lex"),
             NormalTT(~BasicMacroExpander { expander: expand, span: None }, None));
}