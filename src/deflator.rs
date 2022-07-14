use crate::parser;

#[derive(Debug, PartialEq)]
pub enum Move {
    Jump(parser::Jump),
    Choice(Vec<Move>),
    Sequence(Vec<Move>),
    Modded(Box<Move>, Vec<parser::Mod>),
}

impl Move {
    pub fn notation(&self) -> String {
        //TODO: not sure how efficient format!() is, or any of this function really
        match self {
            Move::Jump(j) => format!("[{},{}]", j.x, j.y),
            Move::Choice(moves) => format!(
                "{{{}}}",
                moves
                    .iter()
                    .map(|x| x.notation())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            Move::Sequence(moves) => moves
                .iter()
                .map(|x| x.notation())
                .collect::<Vec<String>>()
                .join("*"),
            Move::Modded(base, mods) => {
                let left: String = base.notation();
                let mod_sequence = mods
                    .iter()
                    .map(|m| match m {
                        parser::Mod::DiagonalMirror => String::from("/"),
                        parser::Mod::HorizontalMirror => String::from("-"),
                        parser::Mod::VerticalMirror => String::from("|"),
                        parser::Mod::Exponentiate(num) => format!("^{}", num),
                        parser::Mod::ExponentiateRange(lower, upper) => {
                            format!("^[{}..{}]", lower, upper)
                        }
                        parser::Mod::ExponentiateInfinite(lower) => match lower {
                            1 => String::from("^*"),
                            lower => format!("^[{}..*]", lower),
                        },
                    })
                    .collect::<Vec<String>>()
                    .concat();
                return left + &mod_sequence;
            }
        }
    }
}

pub trait Deflatable {
    fn deflate(self) -> Move;
}

impl Deflatable for parser::Seq {
    fn deflate(self) -> Move {
        match self {
            parser::Seq::Modded(m) => m.deflate(),

            parser::Seq::Moves(head, tail) => {
                let head = head.deflate();
                let tail = tail.deflate();
                match tail {
                    Move::Sequence(mut arr) => {
                        let mut s: Vec<Move> = vec![head];
                        s.append(&mut arr);
                        Move::Sequence(s)
                    }
                    t => Move::Sequence(vec![head, t]),
                }
            }
        }
    }
}

impl Deflatable for parser::Modded {
    fn deflate(self) -> Move {
        match self {
            parser::Modded::One(o) => o.deflate(),
            parser::Modded::Modded(o, mods) => Move::Modded(Box::new(o.deflate()), mods),
        }
    }
}

impl Deflatable for parser::PieceOption {
    fn deflate(self) -> Move {
        match self {
            parser::PieceOption::Jump(j) => Move::Jump(j),
            parser::PieceOption::Move(m) => m.deflate(),
            parser::PieceOption::Options(moves) => {
                Move::Choice(moves.into_iter().map(|x| x.deflate()).collect())
            }
        }
    }
}
