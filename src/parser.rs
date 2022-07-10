use std::iter::Peekable;
use std::str::Chars;

use peeking_take_while::PeekableExt;

#[derive(Debug,PartialEq)]
pub enum Mod {
    HorizontalMirror,
    VerticalMirror,
    DiagonalMirror,
    Exponentiate(usize),
    ExponentiateRange(usize, usize), //bounds of the range of exponents
    ExponentiateInfinite(usize),     //lower bound of exponent
}


#[derive(Debug,PartialEq)]
pub enum Move {
    Seq(Box<Seq>),
}



#[derive(Debug,PartialEq)]
pub enum Modded {
    Modded(PieceOption, Vec<Mod>),
    One(PieceOption),
}


#[derive(Debug,PartialEq)]
pub enum Seq {
    Moves(Modded, Box<Seq>),
    Modded(Modded),
}

#[derive(Debug,PartialEq)]
pub enum PieceOption {
    Options(Vec<Move>),
    Move(Move),
    Jump(Jump),
}

#[derive(Debug,PartialEq)]
pub struct Jump {
    x: i32,
    y: i32,
}
/*
Jump    ::= [Int,Int]

Mod     ::= -
            | |
            | /
            | ^ Int
            | ^ [Int..Int]
            | ^ [Int..*]


OptionC ::= Move
            | Move , OptionC

Option  ::= {OptionC}
            | (Move)
            | Jump

Seq     ::= Modded * Seq
            | Modded

Modded  ::= | Option Mod
            | Option

Move    ::=  Seq

*/

pub fn parse_string(input: &str) -> Option<Move> {
    //TODO whitespace handling!
    let mut a = input.chars().peekable(); //TODO a .drop(whitespace?)
    let r = parse_move(&mut a);
    match a.next() {
        None => r,
        Some(_) => None, //check that the whole string was consumed.
    }
}

fn parse_move(input: &mut Peekable<Chars>) -> Option<Move> {
    let r = parse_seq(input);
    r.map(|ast| Move::Seq(Box::new(ast)))
}

fn parse_seq(input: &mut Peekable<Chars>) -> Option<Seq> {
    let lhs: Option<Modded> = parse_modded(input);

    match lhs {
        None => None,
        Some(ast) => match input.peek() {
            Some('*') => {
                input.next();
                let rhs = parse_seq(input);
                rhs.map(|rast| Seq::Moves(ast, Box::new(rast)))
            }
            _ => Some(Seq::Modded(ast)),
        },
    }
}

fn parse_modded(input: &mut Peekable<Chars>) -> Option<Modded> {
    let lhs: Option<PieceOption> = parse_option(input);

    match lhs {
        Some(ast) => {
            let mut mods: Vec<Mod> = Vec::new();
            while matches!(input.peek(), Some('|') | Some('/') | Some('-') | Some('^')) {
                let modifier = parse_mod(input)?;
                mods.push(modifier);
            }

            if mods.is_empty() {
                Some(Modded::One(ast))
            } else {
                Some(Modded::Modded(ast, mods))
            }
        }
        None => None,
    }
}

fn parse_mod(input: &mut Peekable<Chars>) -> Option<Mod> {
    match input.next() {
        Some('|') => Some(Mod::VerticalMirror),
        Some('/') => Some(Mod::DiagonalMirror),
        Some('-') => Some(Mod::HorizontalMirror),
        Some('^') => parse_exponentiation_modifier(input),
        _ => None,
    }
}

fn parse_exponentiation_modifier(input: &mut Peekable<Chars>) -> Option<Mod> {
    let f = input.peek()?;
    if *f == '[' {
        //this is a range
        input.next();
        let lower = parse_usize(input)?;
        if !input.take(2).all(|f| f == '.') {
            return None;
        }
        match input.peek() {
            Some('*') => {
                //infinite range
                input.next();
                if input.next() != Some(']') {
                    return None;
                }
                Some(Mod::ExponentiateInfinite(lower))
            }
            Some('0') | Some('1') | Some('2') | Some('3') | Some('4') | Some('5') | Some('6')
            | Some('7') | Some('8') | Some('9') => {
                //finite range
                let upper = parse_usize(input)?;
                if input.next() != Some(']') {
                    return None;
                }
                Some(Mod::ExponentiateRange(lower, upper))
            }
            _ => None,
        }
    } else {
        //this is a single exponent
        let exp = parse_usize(input)?;

        Some(Mod::Exponentiate(exp))
    }
}

fn parse_usize(input: &mut Peekable<Chars>) -> Option<usize> {
    match parse_integer(input)?.try_into() {
        Ok(e) => Some(e),
        Err(_) => None,
    }
}

fn parse_option(input: &mut Peekable<Chars>) -> Option<PieceOption> {
    match input.peek() {
        Some('{') => {
            //parse options
            input.next();
            let mut moves: Vec<Move> = Vec::new();
            match input.peek() {
                Some('}') => Some(PieceOption::Options(moves)), //empty option
                Some(_) => {
                    //at least 1 option
                    moves.push(parse_move(input)?);
                    loop {
                        match input.next() {
                            Some(',') => moves.push(parse_move(input)?),
                            Some('}') => return Some(PieceOption::Options(moves)),
                            _ => return None,
                        }
                    }
                }
                None => None,
            }
        }
        Some('(') => {
            input.next();
            let m = parse_move(input);
            if input.next() != Some(')') {
                return None;
            };
            m.map(PieceOption::Move)
        }
        _ => {
            let jump: Option<Jump> = parse_jump(input);
            jump.map(PieceOption::Jump)
        }
    }
}

fn parse_jump(input: &mut Peekable<Chars>) -> Option<Jump> {
    if input.next() != Some('[') {
        return None;
    }

    let first_int = parse_integer(input)?;    
    if input.next() != Some(',') {
        return None;
    }

    let second_int = parse_integer(input)?;

    println!("{}",second_int);
    if input.next() != Some(']') {
        return None;
    }

    Some(Jump {
        x: first_int,
        y: second_int,
    })
}

fn parse_integer(input: &mut Peekable<Chars>) -> Option<i32> {
    let is_negative: bool = match input.peek() {
        Some('-') => {
            input.next();
            true
        }
        _ => false,
    };
    let mut chars = input
        .peeking_take_while(|x| matches!(x, '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9'));
        

    let s = chars.collect::<String>();
    if s.len() == 0 {
        return None;
    }
    let abs: i32 = s.parse().ok()?;

    if is_negative {
        Some(-abs)
    } else {
        Some(abs)
    }
}


#[cfg(test)]
mod tests {
    use crate::parser::parse_string;

    #[test]
    fn jumps() {
        let result = parse_string("[-3,2]");
        assert_ne!(result, None);
    }
}