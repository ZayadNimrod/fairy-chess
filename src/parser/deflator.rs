use crate::parser;
use crate::movespec::Move;

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
                let choices = moves
                    .into_iter()
                    .map(|x| x.deflate())
                    .flat_map(|x| match x {
                        Move::Choice(c) => c,
                        _ => vec![x],
                    }) //unpack choices, i.e turn {{a,b},{c,d}} into {a,b,c,d}
                    .collect();
                    //TODO perhaps also deflate obviously mirrored items in a choice node?
                Move::Choice(choices)
            }
        }
    }
}
