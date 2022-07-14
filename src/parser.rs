use std::iter::Peekable;

use peeking_take_while::PeekableExt;

use crate::deflator::{Deflatable, Move};

#[derive(Debug, PartialEq)]
pub enum Mod {
    HorizontalMirror,
    VerticalMirror,
    DiagonalMirror,
    Exponentiate(usize),
    ExponentiateRange(usize, usize), //bounds of the range of exponents
    ExponentiateInfinite(usize),     //lower bound of exponent
}

/*
#[derive(Debug, PartialEq)]
pub enum Move {
    Seq(Box<Seq>),
}*/

#[derive(Debug, PartialEq)]
pub enum Modded {
    Modded(PieceOption, Vec<Mod>),
    One(PieceOption),
}

#[derive(Debug, PartialEq)]
pub enum Seq {
    Moves(Modded, Box<Seq>),
    Modded(Modded),
}

#[derive(Debug, PartialEq)]
pub enum PieceOption {
    Options(Vec<Seq>),
    Move(Box<Seq>),
    Jump(Jump),
}

#[derive(Debug, PartialEq)]
pub struct Jump {
    pub x: i32,
    pub y: i32,
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
    //TODO also filter out tabs
    let mut a = input.chars().filter(|x: &char| *x != ' ').peekable();
    let r = parse_seq(&mut a);
    match a.next() {
        None => r,
        Some(_) => None, //check that the whole string was consumed.
    }
    .map(|x| x.deflate())
}

fn parse_seq<T>(input: &mut Peekable<T>) -> Option<Seq>
where
    T: Iterator<Item = char>,
{
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

fn parse_modded<T>(input: &mut Peekable<T>) -> Option<Modded>
where
    T: Iterator<Item = char>,
{
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

fn parse_mod<T>(input: &mut Peekable<T>) -> Option<Mod>
where
    T: Iterator<Item = char>,
{
    match input.next() {
        Some('|') => Some(Mod::VerticalMirror),
        Some('/') => Some(Mod::DiagonalMirror),
        Some('-') => Some(Mod::HorizontalMirror),
        Some('^') => parse_exponentiation_modifier(input),
        _ => None,
    }
}

fn parse_exponentiation_modifier<T>(input: &mut Peekable<T>) -> Option<Mod>
where
    T: Iterator<Item = char>,
{
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
    } else if *f == '*' {
        input.next();
        //this is a single asterisk, signifying [1..*]
        Some(Mod::ExponentiateInfinite(1))
    } else {
        //this is a single exponent
        let exp = parse_usize(input)?;

        Some(Mod::Exponentiate(exp))
    }
}

fn parse_usize<T>(input: &mut Peekable<T>) -> Option<usize>
where
    T: Iterator<Item = char>,
{
    match parse_integer(input)?.try_into() {
        Ok(e) => Some(e),
        Err(_) => None,
    }
}

fn parse_option<T>(input: &mut Peekable<T>) -> Option<PieceOption>
where
    T: Iterator<Item = char>,
{
    match input.peek() {
        Some('{') => {
            //parse options
            input.next();
            let mut moves: Vec<Seq> = Vec::new();
            match input.peek() {
                Some('}') => Some(PieceOption::Options(moves)), //empty option
                Some(_) => {
                    //at least 1 option
                    moves.push(parse_seq(input)?);
                    loop {
                        match input.next() {
                            Some(',') => moves.push(parse_seq(input)?),
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
            let m = parse_seq(input);
            if input.next() != Some(')') {
                return None;
            };
            m.map(|x| PieceOption::Move(Box::new(x)))
        }
        _ => {
            let jump: Option<Jump> = parse_jump(input);
            jump.map(PieceOption::Jump)
        }
    }
}

fn parse_jump<T>(input: &mut Peekable<T>) -> Option<Jump>
where
    T: Iterator<Item = char>,
{
    if input.next() != Some('[') {
        return None;
    }

    let first_int = parse_integer(input)?;
    if input.next() != Some(',') {
        return None;
    }

    let second_int = parse_integer(input)?;
    if input.next() != Some(']') {
        return None;
    }

    Some(Jump {
        x: first_int,
        y: second_int,
    })
}

fn parse_integer<T>(input: &mut Peekable<T>) -> Option<i32>
where
    T: Iterator<Item = char>,
{
    let is_negative: bool = match input.peek() {
        Some('-') => {
            input.next();
            true
        }
        _ => false,
    };
    let chars = input.peeking_take_while(|x| {
        matches!(x, '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9')
    });

    let s = chars.collect::<String>();
    if s.is_empty() {
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

    #[test]
    fn options() {
        let r1 = parse_string("{{[1,2],[2,1]},{[-1,2],[2,-1]},{[1,-2],[-2,1]},{[-1,-2],[-2,-1]}}");
        let r2 = parse_string("{[1,2],[2,1 ], [-1,2],[2, -1],[1,-2], [-2,1],[-1,-2],[-2,-1]}");
        assert_ne!(r1, None);
        assert_ne!(r2, None);
        //assert_eq(r1,r2); this is only after the deflator
    }

    #[test]
    fn sequences() {
        let result =
            parse_string("{[1,2],[2,1],[-1,2],[2,-1],[1,-2],[-2,1],[-1,-2],[-2,-1]} * [0,1]");
        assert_ne!(result, None);
    }

    #[test]
    fn exponentiation() {
        let r2 = parse_string("{[1,1]^4,[-1,1]^4,[1,-1]^4,[1,-1]^4}");
        let r3 = parse_string("{[1,1],[-1,1],[1,-1],[-1,-1]}^4");
        let r4 = parse_string("{[1,1]^[1..4],[-1,1]^[1..4],[1,-1]^[1..4],[1,-1]^[1..4]}");
        let r5 = parse_string("{[1,1],[-1,1],[1,-1],[-1,-1]}^[1..4]");
        let r1 = parse_string("{[1,1]^[1..*],[-1,1]^[1..*],[1,-1]^[1..*],[1,-1]^[1..*]}");
        let r6 = parse_string("{[1,1]^*,[-1,1]^*,[1,-1]^*,[1,-1]^*}"); //tests the syntactical sugar

        assert_ne!(r1, None);
        assert_ne!(r2, None);
        assert_ne!(r3, None);
        assert_ne!(r4, None);
        assert_ne!(r5, None);
        assert_ne!(r6, None);
    }

    #[test]
    fn mirrors() {
        let knight = parse_string("[2,1]/|-");
        assert_ne!(knight, None);
        let rook = parse_string("[1,0]^*|-");
        //println!("{:#?}",rook);
        assert_ne!(rook, None);
    }
}
