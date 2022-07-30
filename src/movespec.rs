use petgraph;

use crate::parser;
pub use crate::parser::Jump;
pub use crate::parser::Mod;

//TODO implement equality such that two choice nodes that have thier choices in a different order, but the same choices, are equal.
#[derive(Debug, PartialEq)]
pub enum MoveCompact {
    Jump(Jump),
    Choice(Vec<MoveCompact>),
    Sequence(Vec<MoveCompact>),
    Modded(Box<MoveCompact>, Mod),
}

impl MoveCompact {
    pub fn notation(&self) -> String {
        //TODO: not sure how efficient format!() is, or any of this function really
        match self {
            MoveCompact::Jump(j) => format!("[{},{}]", j.x, j.y),
            MoveCompact::Choice(moves) => format!(
                "{{{}}}",
                moves
                    .iter()
                    .map(|x| x.notation())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            MoveCompact::Sequence(moves) => moves
                .iter()
                .map(|x| x.notation())
                .collect::<Vec<String>>()
                .join("*"),
            MoveCompact::Modded(base, modifier) => {
                let left: String = base.notation();
                let mod_sequence = match modifier {
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
                };
                left + &mod_sequence
            }
        }
    }
}

impl From<MoveCompact> for String {
    fn from(m: MoveCompact) -> Self {
        m.notation()
    }
}

impl TryFrom<String> for MoveCompact {
    type Error = parser::ParsingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        parser::parse_string(&value)
    }
}

pub struct MoveGraphNode {
    jump: Jump,
}

//TODO couldn't Jump be the edge wieght, and nodes be Units?
pub struct MoveGraph {
    graph: petgraph::csr::Csr<MoveGraphNode, (), petgraph::Directed, petgraph::graph::DefaultIx>,
}

//TODO do I want it to consume the MoveCompact? I can't convert back, and MoveCompact is actually serializable, unless we count the string representation.
//It'll have to though, I think, that's how MoveCompact works, it owns its values...
impl From<MoveCompact> for MoveGraph {
    fn from(input: MoveCompact) -> Self {
        let mut r = MoveGraph {
            graph: petgraph::csr::Csr::new(),
        };
        r.build_from_node(&input);
        r
    }
}

impl MoveGraph {
    //TODO an optimiser for this, notice we are generating dummy nodes!
    fn build_from_node(
        &mut self,
        node: &MoveCompact,
    ) -> (petgraph::graph::DefaultIx, petgraph::graph::DefaultIx) {
        match node {
            MoveCompact::Jump(j) => {
                let idx = self.graph.add_node(MoveGraphNode { jump: j.clone() });
                (idx, idx)
            }
            MoveCompact::Choice(choices) => {
                let head_idx = self.graph.add_node(MoveGraphNode {
                    jump: Jump { x: 0, y: 0 },
                });
                let tail_idx = self.graph.add_node(MoveGraphNode {
                    jump: Jump { x: 0, y: 0 },
                });

                choices.iter().for_each(|c| {
                    let (h, t) = self.build_from_node(c);
                    self.graph.add_edge(head_idx, h, ());
                    self.graph.add_edge(t, tail_idx, ());
                });

                (head_idx, tail_idx)
            }
            MoveCompact::Sequence(seq) => {
                let head_idx = self.graph.add_node(MoveGraphNode {
                    jump: Jump { x: 0, y: 0 },
                });
                let mut tail_idx = self.graph.add_node(MoveGraphNode {
                    jump: Jump { x: 0, y: 0 },
                });
                seq.iter().for_each(|s| {
                    let (h, t) = self.build_from_node(s);
                    self.graph.add_edge(tail_idx, h, ());
                    tail_idx = t;
                });

                (head_idx, tail_idx)
            }
            MoveCompact::Modded(mov, modifier) => self.build_from_mod(mov, modifier),
        }
    }

    fn build_from_mod(
        &mut self,
        mov: &Box<MoveCompact>,
        modifier: &Mod,
    ) -> (petgraph::graph::DefaultIx, petgraph::graph::DefaultIx) {
        match modifier {
            Mod::HorizontalMirror => todo!(),
            Mod::VerticalMirror => todo!(),
            Mod::DiagonalMirror => todo!(),
            Mod::Exponentiate(_) => todo!(),
            Mod::ExponentiateRange(min, max) => {
                let head = self.graph.add_node(MoveGraphNode {
                    jump: Jump { x: 0, y: 0 },
                });
                let tail = self.graph.add_node(MoveGraphNode {
                    jump: Jump { x: 0, y: 0 },
                });
                //let mov_inner = &**mov;
                for exp in *min..=*max {
                    let (h, t) = self.build_from_mod(mov, &Mod::Exponentiate(exp));
                    self.graph.add_edge(head, h, ());
                    self.graph.add_edge(t, tail, ());
                }
                (head, tail)
            }
            Mod::ExponentiateInfinite(min) => {
                let head = self.graph.add_node(MoveGraphNode {
                    jump: Jump { x: 0, y: 0 },
                });
                if *min == 1 {
                    let (loop_back, t) = self.build_from_node(&*mov);
                    self.graph.add_edge(t, loop_back, ());
                    return (head, t);
                } else {
                    let (h, t_mid) = self.build_from_mod(mov, &Mod::Exponentiate(min - 1));
                    let (h_mid, t) = self.build_from_mod(mov, &Mod::ExponentiateInfinite(1));
                    self.graph.add_edge(t_mid, h_mid, ());
                    (h, t)
                }
            }
        }
    }
}
