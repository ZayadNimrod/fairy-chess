use crate::parser;
pub use crate::parser::Jump;
pub use crate::parser::Mod;


//TODO implement equality such that two choice nodes that have thier choices in a different order, but the same choices, are equal.
#[derive(Debug, PartialEq)]
pub enum Move {
    Jump(Jump),
    Choice(Vec<Move>),
    Sequence(Vec<Move>),
    Modded(Box<Move>, Vec<Mod>),
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
                        Mod::DiagonalMirror => String::from("/"),
                        Mod::HorizontalMirror => String::from("-"),
                        Mod::VerticalMirror => String::from("|"),
                        Mod::Exponentiate(num) => format!("^{}", num),
                        Mod::ExponentiateRange(lower, upper) => {
                            format!("^[{}..{}]", lower, upper)
                        }
                        Mod::ExponentiateInfinite(lower) => match lower {
                            1 => String::from("^*"),
                            lower => format!("^[{}..*]", lower),
                        },
                    })
                    .collect::<Vec<String>>()
                    .concat();
                left + &mod_sequence
            }
        }
    }
}


impl Into<String> for Move{
    fn into(self) -> String {
        self.notation()
    }
}


impl TryFrom<String> for Move{
    type Error = parser::ParsingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        parser::parse_string(&value)
    }

}