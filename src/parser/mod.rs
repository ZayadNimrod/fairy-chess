use std::{iter::Peekable, num::TryFromIntError, vec};

use peeking_take_while::PeekableExt;

mod deflator;
use deflator::Deflatable;
use crate::movespec;


#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct Jump {
    pub x: i32,
    pub y: i32,
}


#[derive(Debug, PartialEq)]
pub enum ParsingError {
    ExpectedCharacter(Vec<&'static str>, char, usize), //expected <str or str or str...>, got <char>, at index <usize>
    ExpectedEOF(usize), //parsing ended at character <usize>, but the string is not ended
    UnexpectedEOF(),
    NotAValidExponent(TryFromIntError, i32, usize), //Given exponent <int> at index <char> is not valid
    IntegerParsingError(<i32 as std::str::FromStr>::Err, usize),
}





pub fn parse_string(input: &str) -> Result<crate::movespec::Move, ParsingError> {
    //TODO also filter out tabs
    let mut a = input
        .chars()
        .enumerate()
        .filter(|(_, x): &(usize, char)| *x != ' ')
        .peekable();
    let r = parse_seq(&mut a);
    match a.next() {
        None => r,
        Some((idx, _)) => Err(ParsingError::ExpectedEOF(idx)), //check that the whole string was consumed.
    }
    .map(|x| x.deflate())
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
                Some((_, '|')) | Some((_, '/')) | Some((_, '-')) | Some((_, '^'))
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
        Some((idx, c)) => Err(ParsingError::ExpectedCharacter(
            vec!["|", "/", "-", "^"],
            c,
            idx,
        )),
        None => Err(ParsingError::UnexpectedEOF()),
    }
}

fn parse_exponentiation_modifier<T>(input: &mut Peekable<T>) -> Result<Mod, ParsingError>
where
    T: Iterator<Item = (usize, char)>,
{
    let (_, f) = match input.peek() {
        Some((i, f)) => (*i, *f),
        None => return Err(ParsingError::UnexpectedEOF()),
    };
    if f == '[' {
        //this is a range
        input.next();
        let lower = parse_usize(input)?;
        if !input.take(2).all(|(_, f)| f == '.') {
            let (i, c) = match input.peek() {
                Some((i, f)) => (i, f),
                None => return Err(ParsingError::UnexpectedEOF()),
            };
            return Err(ParsingError::ExpectedCharacter(vec![".."], *c, *i));
        }
        match match input.peek() {
            Some((i, c)) => (*i, *c),
            None => return Err(ParsingError::UnexpectedEOF()),
        } {
            (_, '*') => {
                //infinite range
                input.next();

                match input.next() {
                    Some((_, ']')) => Ok(Mod::ExponentiateInfinite(lower)),
                    None => Err(ParsingError::UnexpectedEOF()),
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
                match input.next() {
                    Some((_, ']')) => Ok(Mod::ExponentiateRange(lower, upper)),
                    None => Err(ParsingError::UnexpectedEOF()),
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
        None => return Err(ParsingError::UnexpectedEOF()),
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
                            None => return Err(ParsingError::UnexpectedEOF()),
                            Some((i, c)) => {
                                return Err(ParsingError::ExpectedCharacter(vec![",", "}"], c, i))
                            }
                        }
                    }
                }
                None => Err(ParsingError::UnexpectedEOF()),
            }
        }
        Some((_, '(')) => {
            input.next();
            let m = parse_seq(input);

            match input.next() {
                Some((_, ')')) => m.map(|x| PieceOption::Move(Box::new(x))),
                None => Err(ParsingError::UnexpectedEOF()),
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
        None => return Err(ParsingError::UnexpectedEOF()),
        Some((idx, c)) => return Err(ParsingError::ExpectedCharacter(vec!["["], c, idx)),
    };

    let first_int = parse_integer(input)?;
    match input.next() {
        Some((_, ',')) => (),
        None => return Err(ParsingError::UnexpectedEOF()),
        Some((idx, c)) => return Err(ParsingError::ExpectedCharacter(vec![","], c, idx)),
    };

    let second_int = parse_integer(input)?;
    match input.next() {
        Some((_, ']')) => (),
        None => return Err(ParsingError::UnexpectedEOF()),
        Some((idx, c)) => return Err(ParsingError::ExpectedCharacter(vec!["]"], c, idx)),
    };

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
        _ => return Err(ParsingError::UnexpectedEOF()),
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
            None => return Err(ParsingError::UnexpectedEOF()),
        };
        return Err(ParsingError::ExpectedCharacter(vec!["integer"], c, i));
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
}
