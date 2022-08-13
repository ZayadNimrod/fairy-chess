use crate::movespec::MoveCompact;
use crate::parser;

impl From<parser::Seq> for MoveCompact {
    fn from(this: parser::Seq) -> Self {
        match this {
            parser::Seq::Modded(m) => MoveCompact::from(m),

            parser::Seq::Moves(head, tail) => {
                let head = MoveCompact::from(head);
                let tail = MoveCompact::from(*tail);
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

impl From<parser::Modded> for MoveCompact {
    fn from(this: parser::Modded) -> Self {
        match this {
            parser::Modded::One(o) => MoveCompact::from(o),
            parser::Modded::Modded(o, mods) => {
                let inner = MoveCompact::from(o);
                mods.into_iter()
                    .fold(inner, |acc, m| MoveCompact::Modded(Box::new(acc), m))
            }
        }
    }
}

impl From<parser::PieceOption> for MoveCompact {   
    fn from(this: parser::PieceOption) -> Self {
        match this {
            parser::PieceOption::Jump(j) => MoveCompact::Jump(j),
            parser::PieceOption::Move(m) => MoveCompact::from(*m),
            parser::PieceOption::Options(moves) => {
                let choices = moves
                    .into_iter()
                    .map(|x| MoveCompact::from(x))
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
