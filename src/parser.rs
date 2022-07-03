//use std::collections::Vec;
use std::iter::Peekable;
use std::str::Chars;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while1};
use nom::character::complete::char;
use nom::multi::separated_list1;
use nom::sequence::tuple;
use nom::Parser;

pub enum Mod {
    HorizontalMirror,
    VerticalMirror,
    DiagonalMirror,
    Exponentiate(usize),
    ExponentiateRange(usize, usize), //bounds of the range of exponents
    ExponentiateInfinite(usize),     //lower bound of exponent
}

pub enum Move {
    Seq(Box<Seq>),
}

pub enum Modded {
    Modded(PieceOption, Vec<Mod>),
    One(PieceOption),
}

pub enum Seq {
    Moves(Modded, Box<Seq>),
    Modded(Modded),
}

pub enum PieceOption {
    Options(Vec<Move>),
    Move(Move),
    Jump(Jump),
}

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

pub fn parse_string(input: String) -> Option<Move> {
    //TODO whitespace handling!
    /*
    //TODO these are constant, break them outwards
    
    let int = take_while1( |c| {c >= b'0' && c <= b'9'} ); //TODO get negatives
    let piece_jump = tuple((char('['), int, int, char(']')));

    let piece_move_mod = alt();
    let piece_move_contents = alt((piece_seq,piece_move_mod));
    let piece_move = alt((tuple((char('('),piece_move_contents,char(')'))),piece_move_contents ));


    let parse = piece_move(input.as_bytes());
    */

    let mut a = input.chars().peekable();
    let r = parse_move(&mut a);
    match a.next() {
        None => r,
        Some(_) => None, //check that the whole string was consumed.
    }
}

fn parse_move(input: &mut Peekable<Chars>) -> Option<Move> {
    let r = parse_seq(input);
    //TODO convert this into the map I'm using in parse_seq
    match r {
        None => None,
        Some(ast) => Some(Move::Seq(Box::new(ast))),
    }
}

fn parse_seq(input: &mut Peekable<Chars>) -> Option<Seq> {
    let lhs: Option<Modded> = parse_modded(input);

    match lhs {
        None => None,
        Some(ast) => match input.peek() {
            Some('*') => {
                input.next();
                let rhs = parse_seq(input);
                match rhs {
                    Some(rast) => Some(Seq::Moves(ast, Box::new(rast))),
                    None => None,
                }
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
            while match input.peek() {
                Some('|') | Some('/') | Some('-') | Some('^') => true,
                _ => false,
            } {
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
                Some('}') => return Some(PieceOption::Options(moves)), //empty option
                Some(_) => {
                    //at least 1 option
                    moves.push(parse_move(input)?);
                    loop {
                        match input.next() {
                            Some(',') => {moves.push(parse_move(input)?)},
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
            m.map(|ast| PieceOption::Move(ast))
        }
        _ => {
            let jump: Option<Jump> = parse_jump(input);
            jump.map(|j| PieceOption::Jump(j))
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
    if input.next() != Some(']') {
        return None;
    }

    Some(Jump {
        x: first_int,
        y: second_int,
    })
}

fn parse_integer(input: &mut Peekable<Chars>) -> Option<i32> {
    //TODO there has to be a better way than this to parse integers!

    let negative = match input.peek() {
        Some('-') => {
            input.next();
            true
        }
        _ => false,
    };

    /*
        input.take_while(|x| match x {
            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '0' => true,
            _ => false,
        })
        .fold(None::<i32>, |acc:Option<i32> , c:char| {Some(char_to_int(c) +10*acc.unwrap_or(0) )});
    */

    let mut chars = input.take_while(|x| match x {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => true,
        _ => false,
    }); //TODO: I'm doing a mutable JUST for the sake of the next line. That seems dumb to me, I should chnage that.

    if !chars.any(|_x| true) {
        return None;
    } //empty iterator returns false, this is checking if the iterator is empty.

    let abs = chars.fold(0, |acc, c| char_to_int(c) + 10 * acc);

    if negative {
        Some(-abs)
    } else {
        Some(abs)
    }
}

fn char_to_int(c: char) -> i32 {
    match c {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        _ => panic!("char_to_int was called on a non-digit character"),
    }
}
