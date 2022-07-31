use std::iter::Zip;
use std::ops::Index;

use petgraph;
use petgraph::visit::IntoNeighbors;

use crate::parser;
pub use crate::parser::Jump;
pub use crate::parser::Mod;

//TODO implement equality such that two choice nodes that have thier choices in a different order, but the same choices, are equal.
#[derive(Debug, PartialEq,Clone)]
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

//TODO couldn't Jump be the edge wieght, and nodes be Units? We can test whether that works better later
pub struct MoveGraph {
    graph: petgraph::csr::Csr<MoveGraphNode, EdgeType, petgraph::Directed, petgraph::graph::DefaultIx>,
    head : petgraph::graph::DefaultIx
}


#[derive(Clone,PartialEq)]
pub enum EdgeType{
    Optional,
    Required
}


//TODO do I want it to consume the MoveCompact? I can't convert back, and MoveCompact is actually serializable, unless we count the string representation- which has to be genrated from MoveCompact anyway!
//It'll have to though, I think, that's how MoveCompact works, it owns its values...
impl From<MoveCompact> for MoveGraph {
    fn from(input: MoveCompact) -> Self {
        let mut r = MoveGraph {
            graph: petgraph::csr::Csr::new(),
            head:0
        };
        let (h,_)=r.build_from_node(&input);
        r.head=h;
        r
    }
}


impl MoveCompact{
    pub fn map<F>(&self, f:F) -> MoveCompact
    where F:Fn(&Jump)->Jump,
    F:Copy
    {
        match self {
            MoveCompact::Jump(j) => MoveCompact::Jump(f(j)),
            MoveCompact::Choice(c) => MoveCompact::Choice(c.iter().map(|x|x.map(f)).collect()),
            MoveCompact::Sequence(s) => MoveCompact::Sequence(s.iter().map(|x|x.map(f)).collect()),
            MoveCompact::Modded(mo, md) => MoveCompact::Modded(Box::new(mo.map(f)),md.clone()),
        }
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
                let idx = self.graph.add_node(MoveGraphNode { jump: *j });
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
                    self.graph.add_edge(head_idx, h, EdgeType::Optional);
                    self.graph.add_edge(t, tail_idx, EdgeType::Required);
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
                    self.graph.add_edge(tail_idx, h, EdgeType::Required);
                    tail_idx = t;
                });

                (head_idx, tail_idx)
            }
            MoveCompact::Modded(mov, modifier) => self.build_from_mod(mov, modifier),
        }
    }

    fn build_from_mod(
        &mut self,
        mov: &MoveCompact,
        modifier: &Mod,
    ) -> (petgraph::graph::DefaultIx, petgraph::graph::DefaultIx) {
        match modifier {
            Mod::HorizontalMirror =>  {
                self.build_from_node(&MoveCompact::Choice(vec![mov.map(|j|Jump{x:j.x,y:-j.y}),(*mov).clone()]))
            },
            Mod::VerticalMirror => {
                self.build_from_node(&MoveCompact::Choice(vec![mov.map(|j|Jump{x:-j.x,y:j.y}),(*mov).clone()]))
            },
            Mod::DiagonalMirror => {
                self.build_from_node(&MoveCompact::Choice(vec![mov.map(|j|Jump{x:j.y,y:j.x}),(*mov).clone()]))
            },
            Mod::Exponentiate(exp) => {
                if *exp==1{
                    self.build_from_node(mov)
                }else{
                    let (h, t_mid) = self.build_from_mod(mov, &Mod::Exponentiate(exp - 1));
                    let (h_mid,t)=self.build_from_node(mov);
                    self.graph.add_edge(t_mid, h_mid, EdgeType::Required);
                    (h,t)
                }

            },
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
                    self.graph.add_edge(head, h, EdgeType::Optional);
                    self.graph.add_edge(t, tail, EdgeType::Required);
                }
                (head, tail)
            }
            Mod::ExponentiateInfinite(min) => {
                let head = self.graph.add_node(MoveGraphNode {
                    jump: Jump { x: 0, y: 0 },
                });
                if *min == 1 {
                    let (loop_back, t) = self.build_from_node(&*mov);
                    self.graph.add_edge(t, loop_back, EdgeType::Optional);
                    (head, t)
                } else {
                    let (h, t_mid) = self.build_from_mod(mov, &Mod::Exponentiate(min - 1));
                    let (h_mid, t) = self.build_from_mod(mov, &Mod::ExponentiateInfinite(1));
                    self.graph.add_edge(t_mid, h_mid, EdgeType::Required);
                    (h, t)
                }
            }
        }
    }

    pub fn successors(&self,idx:petgraph::graph::DefaultIx)->petgraph::csr::Neighbors<>{
        self.graph.neighbors(idx)
    }

    pub fn outgoing_edges(&self,idx:petgraph::graph::DefaultIx)->petgraph::csr::Edges<EdgeType>{
        self.graph.edges(idx)
    }

    pub fn all_outgoing(&self,idx:petgraph::graph::DefaultIx)->Zip<petgraph::csr::Neighbors<>, petgraph::csr::Edges<EdgeType>>{
        self.successors(idx).zip(
            self.outgoing_edges(idx))
    }

    pub fn jump_at(&self, idx:petgraph::graph::DefaultIx)->&Jump{
        &self.graph.index(idx).jump
    }

    pub fn head(&self) -> u32 {
        self.head
    }
}
