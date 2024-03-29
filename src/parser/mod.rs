use std::{iter::Peekable, num::TryFromIntError, vec};

use peeking_take_while::PeekableExt;
use thiserror::Error;

mod deflator;

#[derive(Debug, PartialEq, Clone)]
pub enum Mod {
    HorizontalMirror,
    VerticalMirror,
    DiagonalMirror,
    Exponentiate(usize),
    ExponentiateRange(usize, usize), //bounds of the range of exponents
    ExponentiateInfinite(usize),     //lower bound of exponent
}

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

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Jump {
    pub x: i32,
    pub y: i32,
}

// TODO add positions where the error occurred to all errors
#[derive(Debug, PartialEq, Error)]
pub enum ParsingError {
    #[error("Expected one of {0:?}, found {1}, at character position {2}")]
    ExpectedCharacter(Vec<&'static str>, char, usize),
    #[error("parsing ended at character {0}, but the string is not ended ")]
    ExpectedEOF(usize),
    #[error("Unexpected EOF (unknown position)")]
    UnexpectedEOF,
    #[error("The exponent ({1}) at character position {2} is invalid: {0}")]
    NotAValidExponent(TryFromIntError, i32, usize),
    #[error("Expected an integer at character position {1} : {0}")]
    IntegerParsingError(<i32 as std::str::FromStr>::Err, usize),
    #[error("[0,0] is not a valid jump (Unknown position_")]
    NotAValidJump,
    #[error(
        "Upper bound ({1}) in exponent range is less than lower bound ({0}) (unkown position)"
    )]
    UpperExpLessThanLower(usize, usize),
}

pub(crate) fn parse_string(input: &str) -> Result<crate::movespec::MoveCompact, ParsingError> {
    //TODO also filter out tabs
    let mut a = input
        .chars()
        .enumerate()
        .filter(|(_, x): &(usize, char)| *x != ' ')
        .peekable();
    let r = parse_seq(&mut a);
    match r {
        Ok(ast) => {
            match a.next() {
                None => Ok(crate::movespec::MoveCompact::from(ast)),
                Some((idx, _)) => Err(ParsingError::ExpectedEOF(idx)), //check that the whole string was consumed.
            }
        }
        Err(e) => Err(e),
    }
}

fn parse_seq<T>(input: &mut Peekable<T>) -> Result<Seq, ParsingError>
where
    T: Iterator<Item = (usize, char)>,
{
    let lhs: Result<Modded, ParsingError> = parse_modded(input);

    match lhs {
        Err(e) => Err(e),
        Ok(ast) => match input.peek() {
            Some((_, '*')) => {
                input.next();
                let rhs = parse_seq(input);
                rhs.map(|rast| Seq::Moves(ast, Box::new(rast)))
            }
            _ => Ok(Seq::Modded(ast)),
        },
    }
}

fn parse_modded<T>(input: &mut Peekable<T>) -> Result<Modded, ParsingError>
where
    T: Iterator<Item = (usize, char)>,
{
    let lhs: Result<PieceOption, ParsingError> = parse_option(input);

    match lhs {
        Ok(ast) => {
            let mut mods: Vec<Mod> = Vec::new();
            while matches!(
                input.peek(),
                Some((_, '|')) | Some((_, '/')) | Some((_, '-')) | Some((_, '^')) | Some((_, '?'))
            ) {
                let modifier = parse_mod(input)?;
                mods.push(modifier);
            }

            if mods.is_empty() {
                Ok(Modded::One(ast))
            } else {
                Ok(Modded::Modded(ast, mods))
            }
        }
        Err(e) => Err(e),
    }
}

fn parse_mod<T>(input: &mut Peekable<T>) -> Result<Mod, ParsingError>
where
    T: Iterator<Item = (usize, char)>,
{
    match input.next() {
        Some((_, '|')) => Ok(Mod::VerticalMirror),
        Some((_, '/')) => Ok(Mod::DiagonalMirror),
        Some((_, '-')) => Ok(Mod::HorizontalMirror),
        Some((_, '^')) => parse_exponentiation_modifier(input),
        Some((_, '?')) => Ok(Mod::ExponentiateRange(0, 1)), //? is syntactical sugar for ^[0..1]
        Some((idx, c)) => Err(ParsingError::ExpectedCharacter(
            vec!["|", "/", "-", "^"],
            c,
            idx,
        )),
        None => Err(ParsingError::UnexpectedEOF),
    }
}

fn parse_exponentiation_modifier<T>(input: &mut Peekable<T>) -> Result<Mod, ParsingError>
where
    T: Iterator<Item = (usize, char)>,
{
    let (_, f) = match input.peek() {
        Some((i, f)) => (*i, *f),
        None => return Err(ParsingError::UnexpectedEOF),
    };
    if f == '[' {
        //this is a range
        input.next();
        let lower = parse_usize(input)?;
        if !input.take(2).all(|(_, f)| f == '.') {
            let (i, c) = match input.peek() {
                Some((i, f)) => (i, f),
                None => return Err(ParsingError::UnexpectedEOF),
            };
            return Err(ParsingError::ExpectedCharacter(vec![".."], *c, *i));
        }
        match match input.peek() {
            Some((i, c)) => (*i, *c),
            None => return Err(ParsingError::UnexpectedEOF),
        } {
            (_, '*') => {
                //infinite range
                input.next();

                match input.next() {
                    Some((_, ']')) => Ok(Mod::ExponentiateInfinite(lower)),
                    None => Err(ParsingError::UnexpectedEOF),
                    Some((idx, c)) => Err(ParsingError::ExpectedCharacter(vec!["]"], c, idx)),
                }
            }

            (_, '0')
            | (_, '1')
            | (_, '2')
            | (_, '3')
            | (_, '4')
            | (_, '5')
            | (_, '6')
            | (_, '7')
            | (_, '8')
            | (_, '9') => {
                //finite range

                let upper = parse_usize(input)?;
                if upper <= lower {
                    return Err(ParsingError::UpperExpLessThanLower(lower, upper));
                }
                match input.next() {
                    Some((_, ']')) => Ok(Mod::ExponentiateRange(lower, upper)),
                    None => Err(ParsingError::UnexpectedEOF),
                    Some((idx, c)) => Err(ParsingError::ExpectedCharacter(vec!["]"], c, idx)),
                }
            }
            (i, c) => Err(ParsingError::ExpectedCharacter(
                vec!["*", "non-negative integer"],
                c,
                i,
            )),
        }
    } else if f == '*' {
        input.next();
        //this is a single asterisk, signifying [1..*]
        Ok(Mod::ExponentiateInfinite(1))
    } else {
        //this is a single exponent
        let exp = parse_usize(input)?;

        Ok(Mod::Exponentiate(exp))
    }
}

fn parse_usize<T>(input: &mut Peekable<T>) -> Result<usize, ParsingError>
where
    T: Iterator<Item = (usize, char)>,
{
    let idx = match input.peek() {
        Some((idx, _)) => *idx,
        None => return Err(ParsingError::UnexpectedEOF),
    };

    let int: i32 = parse_integer(input)?;

    match int.try_into() {
        Ok(e) => Ok(e),
        Err(e) => Err(ParsingError::NotAValidExponent(e, int, idx)),
    }
}

fn parse_option<T>(input: &mut Peekable<T>) -> Result<PieceOption, ParsingError>
where
    T: Iterator<Item = (usize, char)>,
{
    match input.peek() {
        Some((_, '{')) => {
            //parse options
            input.next();
            let mut moves: Vec<Seq> = Vec::new();
            match input.peek() {
                Some((_, '}')) => Ok(PieceOption::Options(moves)), //empty option
                Some(_) => {
                    //at least 1 option
                    moves.push(parse_seq(input)?);
                    loop {
                        match input.next() {
                            Some((_, ',')) => moves.push(parse_seq(input)?),
                            Some((_, '}')) => return Ok(PieceOption::Options(moves)),
                            None => return Err(ParsingError::UnexpectedEOF),
                            Some((i, c)) => {
                                return Err(ParsingError::ExpectedCharacter(vec![",", "}"], c, i))
                            }
                        }
                    }
                }
                None => Err(ParsingError::UnexpectedEOF),
            }
        }
        Some((_, '(')) => {
            input.next();
            let m = parse_seq(input);

            match input.next() {
                Some((_, ')')) => m.map(|x| PieceOption::Move(Box::new(x))),
                None => Err(ParsingError::UnexpectedEOF),
                Some((idx, c)) => Err(ParsingError::ExpectedCharacter(vec![")"], c, idx)),
            }
        }
        _ => {
            let jump: Result<Jump, ParsingError> = parse_jump(input);
            jump.map(PieceOption::Jump)
        }
    }
}

fn parse_jump<T>(input: &mut Peekable<T>) -> Result<Jump, ParsingError>
where
    T: Iterator<Item = (usize, char)>,
{
    match input.next() {
        Some((_, '[')) => (),
        None => return Err(ParsingError::UnexpectedEOF),
        Some((idx, c)) => return Err(ParsingError::ExpectedCharacter(vec!["["], c, idx)),
    };

    let first_int = parse_integer(input)?;
    match input.next() {
        Some((_, ',')) => (),
        None => return Err(ParsingError::UnexpectedEOF),
        Some((idx, c)) => return Err(ParsingError::ExpectedCharacter(vec![","], c, idx)),
    };

    let second_int = parse_integer(input)?;
    match input.next() {
        Some((_, ']')) => (),
        None => return Err(ParsingError::UnexpectedEOF),
        Some((idx, c)) => return Err(ParsingError::ExpectedCharacter(vec!["]"], c, idx)),
    };

    if first_int == 0 && second_int == 0 {
        return Err(ParsingError::NotAValidJump);
    }

    Ok(Jump {
        x: first_int,
        y: second_int,
    })
}

fn parse_integer<T>(input: &mut Peekable<T>) -> Result<i32, ParsingError>
where
    T: Iterator<Item = (usize, char)>,
{
    let i: usize = match input.peek() {
        Some((i, _)) => *i,
        _ => return Err(ParsingError::UnexpectedEOF),
    };

    let is_negative: bool = match input.peek() {
        Some((_, '-')) => {
            input.next();
            true
        }
        _ => false,
    };
    let chars = input
        .peeking_take_while(|(_, x)| {
            matches!(x, '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9')
        })
        .map(|(_, x)| x);

    let s: String = chars.collect::<String>();
    if s.is_empty() {
        let (i, c) = match input.peek() {
            Some((i, c)) => (*i, *c),
            None => return Err(ParsingError::UnexpectedEOF),
        };
        return Err(ParsingError::ExpectedCharacter(
            vec!["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"],
            c,
            i,
        ));
    }

    let abs: i32 = match s.parse::<i32>() {
        Ok(i) => i,
        Err(e) => return Err(ParsingError::IntegerParsingError(e, i)),
    };

    if is_negative {
        Ok(-abs)
    } else {
        Ok(abs)
    }
}

//TODO test specific errors

#[cfg(test)]
mod tests {
    use crate::parser::parse_string;

    #[test]
    fn jumps() {
        let result = parse_string("[-3,2]");
        assert!(result.is_ok());
    }

    #[test]
    fn options() {
        let r1 = parse_string("{{[1,2],[2,1]},{[-1,2],[2,-1]},{[1,-2],[-2,1]},{[-1,-2],[-2,-1]}}");
        let r2 = parse_string("{[1,2],[2,1 ], [-1,2],[2, -1],[1,-2], [-2,1],[-1,-2],[-2,-1]}");
        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert_eq!(r1, r2);
    }

    #[test]
    fn sequences() {
        let result =
            parse_string("{[1,2],[2,1],[-1,2],[2,-1],[1,-2],[-2,1],[-1,-2],[-2,-1]} * [0,1]");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().notation(),
            "{[1,2],[2,1],[-1,2],[2,-1],[1,-2],[-2,1],[-1,-2],[-2,-1]}*[0,1]"
        )
    }

    #[test]
    fn exponentiation() {
        let r2 = parse_string("{[1,1]^4,[-1,1]^4,[1,-1]^4,[1,-1]^4}");
        let r3 = parse_string("{[1,1],[-1,1],[1,-1],[-1,-1]}^4");
        let r4 = parse_string("{[1,1]^[1..4],[-1,1]^[1..4],[1,-1]^[1..4],[1,-1]^[1..4]}");
        let r5 = parse_string("{[1,1],[-1,1],[1,-1],[-1,-1]}^[1..4]");
        let r1 = parse_string("{[1,1]^[1..*],[-1,1]^[1..*],[1,-1]^[1..*],[1,-1]^[1..*]}");
        let r6 = parse_string("{[1,1]^*,[-1,1]^*,[1,-1]^*,[1,-1]^*}"); //tests the syntactical sugar

        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert!(r3.is_ok());
        assert!(r4.is_ok());
        assert!(r5.is_ok());
        assert!(r6.is_ok());
        assert_eq!(r6, r1);

        //TODO still need to verify that, for example, r4 and r5 are equal. This cannot be done in the deflator, however, as it requires unrolling.
    }

    #[test]
    fn mirrors() {
        let knight = parse_string("[2,1]/|-");
        assert!(knight.is_ok());
        let rook = parse_string("[1,0]^*|-");
        //println!("{:#?}",rook);
        assert!(rook.is_ok());
    }

    #[test]
    fn integer_parsing() {
        let knight = parse_string("[2,1]/|-");
        assert!(knight.is_ok());
        let knight = parse_string("[e,1]/|-");
        match knight.err().unwrap() {
            crate::parser::ParsingError::ExpectedCharacter(_, _, i) => assert_eq!(i, 1),
            _ => panic!(),
        }
        let knight = parse_string("[2.2,1]/|-");
        assert!(knight.is_err());
        //println!("{:?}",knight);
        match knight.unwrap_err() {
            crate::parser::ParsingError::ExpectedCharacter(_, _, i) => assert_eq!(i, 2), //the decimal point should cause an error
            _ => panic!(),
        }
    }

    #[test]
    fn jump_parsing() {
        //0,0 shouldn't be allowed to parse!
        let i = parse_string("[0,0]");
        assert!(i.is_err());
    }

    #[test]
    fn exp_parsing() {
        //negative exp shouldn't be allowed to parse!
        let i = parse_string("[1,0]^[-1..2]");
        assert!(i.is_err());

        let i = parse_string("[1,0]^[1.2..2.4]");
        assert!(i.is_err());
    }

    #[test]
    fn allow_tabs() {
        //TODO this isn't using proper tabs, it's spaces...
        let knight = parse_string("[2,  1]/     |-   ");
        assert!(knight.is_ok());
    }
}
