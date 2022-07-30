use crate::movespec::MoveCompact;
use crate::parser;

pub trait Deflatable {
    fn deflate(self) -> MoveCompact;
}

impl Deflatable for parser::Seq {
    fn deflate(self) -> MoveCompact {
        match self {
            parser::Seq::Modded(m) => m.deflate(),

            parser::Seq::Moves(head, tail) => {
                let head = head.deflate();
                let tail = tail.deflate();
                match tail {
                    MoveCompact::Sequence(mut arr) => {
                        let mut s: Vec<MoveCompact> = vec![head];
                        s.append(&mut arr);
                        MoveCompact::Sequence(s)
                    }
                    t => MoveCompact::Sequence(vec![head, t]),
                }
            }
        }
    }
}

impl Deflatable for parser::Modded {
    fn deflate(self) -> MoveCompact {
        match self {
            parser::Modded::One(o) => o.deflate(),
            parser::Modded::Modded(o, mods) => {
                let inner = o.deflate();
                mods.into_iter()
                    .fold(inner, |acc, m| MoveCompact::Modded(Box::new(acc), m))
            }
        }
    }
}

impl Deflatable for parser::PieceOption {
    fn deflate(self) -> MoveCompact {
        match self {
            parser::PieceOption::Jump(j) => MoveCompact::Jump(j),
            parser::PieceOption::Move(m) => m.deflate(),
            parser::PieceOption::Options(moves) => {
                let choices = moves
                    .into_iter()
                    .map(|x| x.deflate())
                    .flat_map(|x| match x {
                        MoveCompact::Choice(c) => c,
                        _ => vec![x],
                    }) //unpack choices, i.e turn {{a,b},{c,d}} into {a,b,c,d}
                    .collect();
                //TODO perhaps also deflate obviously mirrored items in a choice node?
                MoveCompact::Choice(choices)
            }
        }
    }
}
