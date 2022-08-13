use std::iter::Zip;

use petgraph;
use petgraph::adj::EdgeIndex;
use petgraph::graph::IndexType;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::EdgeReference;
use petgraph::visit::EdgeRef;
use petgraph::visit::IntoEdges;
use petgraph::visit::IntoNeighbors;
use petgraph::EdgeDirection;

use crate::parser;
pub use crate::parser::Jump;
pub use crate::parser::Mod;

//TODO implement equality such that two choice nodes that have thier choices in a different order, but the same choices, are equal.
#[derive(Debug, PartialEq, Clone)]
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
    //jump: Jump,
}

#[derive(Debug)]
pub struct MoveGraph<Ix>
where
    Ix: IndexType,
{
    pub graph: petgraph::stable_graph::StableDiGraph<(), EdgeType, Ix>,
    head: NodeIndex<Ix>,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum EdgeType {
    Optional(Jump),
    Required(Jump),
    DummyOptional,
    DummyRequired,
}

//TODO do I want it to consume the MoveCompact? I can't convert back, and MoveCompact is actually serializable, unless we count the string representation- which has to be genrated from MoveCompact anyway!
//It'll have to though, I think, that's how MoveCompact works, it owns its values...
impl<Ix> From<MoveCompact> for MoveGraph<Ix>
where
    Ix: IndexType,
{
    fn from(input: MoveCompact) -> Self {
        let mut r = MoveGraph::<Ix> {
            graph: petgraph::stable_graph::StableDiGraph::<(), EdgeType, Ix>::with_capacity(0, 0),
            head: NodeIndex::<Ix>::default(),
        };
        let (h, _) = r.build_from_node(&input);
        r.head = h;
        r.deflate();
        r
    }
}

impl MoveCompact {
    pub fn map<F>(&self, f: F) -> MoveCompact
    where
        F: Fn(&Jump) -> Jump,
        F: Copy,
    {
        match self {
            MoveCompact::Jump(j) => MoveCompact::Jump(f(j)),
            MoveCompact::Choice(c) => MoveCompact::Choice(c.iter().map(|x| x.map(f)).collect()),
            MoveCompact::Sequence(s) => MoveCompact::Sequence(s.iter().map(|x| x.map(f)).collect()),
            MoveCompact::Modded(mo, md) => MoveCompact::Modded(Box::new(mo.map(f)), md.clone()),
        }
    }
}

impl<Ix> MoveGraph<Ix>
where
    Ix: IndexType,
{
    //TODO we generate a bunch of dummy nodes; DELETE THEM
    fn build_from_node(&mut self, node: &MoveCompact) -> (NodeIndex<Ix>, NodeIndex<Ix>) {
        match node {
            MoveCompact::Jump(j) => {
                let h = self.graph.add_node(());
                let t = self.graph.add_node(());
                self.graph.add_edge(h, t, EdgeType::Required(*j));
                (h, t)
            }
            MoveCompact::Choice(choices) => {
                let head_idx = self.graph.add_node(());
                let tail_idx = self.graph.add_node(());

                //merge the heads and tails of all the choice graphs
                choices.iter().for_each(|c| {
                    let (h, t) = self.build_from_node(c);
                    //self.merge(head_idx, h);
                    //self.merge(tail_idx, t);
                    self.graph.add_edge(head_idx, h, EdgeType::DummyRequired);
                    self.graph.add_edge(t, tail_idx, EdgeType::DummyRequired);
                });

                (head_idx, tail_idx)
            }
            MoveCompact::Sequence(seq) => {
                let mut tail_idx = self.graph.add_node(());

                let head_idx = seq
                    .iter()
                    .map(|s| {
                        let (h, t) = self.build_from_node(s);
                        //merge tail_idx with h
                        self.merge(h, tail_idx);
                        //get new tail
                        tail_idx = t;
                        h
                    })
                    .collect::<Vec<NodeIndex<Ix>>>()[0];

                (head_idx, tail_idx)
            }
            MoveCompact::Modded(mov, modifier) => self.build_from_mod(mov, modifier),
        }
    }

    fn build_from_mod(
        &mut self,
        mov: &MoveCompact,
        modifier: &Mod,
    ) -> (NodeIndex<Ix>, NodeIndex<Ix>) {
        match modifier {
            Mod::HorizontalMirror => self.build_from_node(&MoveCompact::Choice(vec![
                mov.map(|j| Jump { x: j.x, y: -j.y }),
                (*mov).clone(),
            ])),
            Mod::VerticalMirror => self.build_from_node(&MoveCompact::Choice(vec![
                mov.map(|j| Jump { x: -j.x, y: j.y }),
                (*mov).clone(),
            ])),
            Mod::DiagonalMirror => self.build_from_node(&MoveCompact::Choice(vec![
                mov.map(|j| Jump { x: j.y, y: j.x }),
                (*mov).clone(),
            ])),
            Mod::Exponentiate(exp) => {
                if *exp == 0 {
                    let h = self.graph.add_node(());
                    let t = self.graph.add_node(());
                    self.graph.add_edge(h, t, EdgeType::DummyRequired);
                    (h, t)
                } else if *exp == 1 {
                    self.build_from_node(mov)
                } else {
                    let (h, t_mid) = self.build_from_mod(mov, &Mod::Exponentiate(exp - 1));
                    let (h_mid, t) = self.build_from_node(mov);
                    self.merge(t_mid, h_mid);
                    (h, t)
                }
            }
            Mod::ExponentiateRange(min, max) => {
                let head = self.graph.add_node(());
                let tail = self.graph.add_node(());
                for exp in *min..=*max {
                    let (h, t) = self.build_from_mod(mov, &Mod::Exponentiate(exp));

                    self.graph.add_edge(head, h, EdgeType::DummyRequired);
                    self.graph.add_edge(t, tail, EdgeType::DummyRequired);
                }
                (head, tail)
            }
            Mod::ExponentiateInfinite(min) => {
                //TODO do we have to use 1 as a guard value? Let's turn it into its own function, no?
                if *min == 1 {
                    let (loop_back, t) = self.build_from_node(&*mov);
                    self.graph.add_edge(t, loop_back, EdgeType::DummyOptional);
                    (loop_back, t)
                } else {
                    let (h, t_mid) = self.build_from_mod(mov, &Mod::Exponentiate(*min - 1));
                    let (h_mid, t) = self.build_from_mod(mov, &Mod::ExponentiateInfinite(1));
                    self.merge(h_mid, t_mid);
                    (h, t)
                }
            }
        }
    }

    fn merge(&mut self, to_keep: NodeIndex<Ix>, to_drop: NodeIndex<Ix>) {
        let drop_outgoing: Vec<(NodeIndex<Ix>, EdgeType)> = self
            .graph
            .edges_directed(to_drop, EdgeDirection::Outgoing)
            .map(|r| (r.target(), *r.weight()))
            .collect();

        for (target, weight) in drop_outgoing {
            self.graph.add_edge(to_keep, target, weight);
        }

        let drop_incoming: Vec<(NodeIndex<Ix>, EdgeType)> = self
            .graph
            .edges_directed(to_drop, EdgeDirection::Incoming)
            .map(|r| (r.source(), *r.weight()))
            .collect();

        for (source, weight) in drop_incoming {
            self.graph.add_edge(source, to_keep, weight);
        }
        self.graph.remove_node(to_drop);
    }

    pub fn successors(
        &self,
        idx: NodeIndex<Ix>,
    ) -> <&petgraph::stable_graph::StableDiGraph<(), EdgeType, Ix> as IntoNeighbors>::Neighbors
    {
        self.graph.neighbors(idx)
    }

    pub fn outgoing_edges(
        &self,
        idx: NodeIndex<Ix>,
    ) -> <&petgraph::stable_graph::StableDiGraph<(), EdgeType, Ix> as IntoEdges>::Edges {
        self.graph.edges(idx)
    }

    pub fn all_outgoing(
        &self,
        idx: NodeIndex<Ix>,
    ) -> Zip<
        <&petgraph::stable_graph::StableDiGraph<(), EdgeType, Ix> as IntoNeighbors>::Neighbors,
        <&petgraph::stable_graph::StableDiGraph<(), EdgeType, Ix> as IntoEdges>::Edges,
    > {
        self.successors(idx).zip(self.outgoing_edges(idx))
    }

    pub fn head(&self) -> NodeIndex<Ix> {
        self.head
    }

    ///deflate the graph by removing superfluous nodes
    pub fn deflate(&mut self) {
        loop {
            //Find a reason to merge nodes

            //Reason 1: if there is only one outgoing edge, and it is a dummy type, we can merge the nodes
            let mut es = self.graph.node_indices().filter_map(|n| {
                let es: Vec<EdgeReference<EdgeType, Ix>> = self.outgoing_edges(n).collect();
                if es.len() == 1 {
                    let e = es[0];
                    return match e.weight() {
                        EdgeType::Optional(_) => None,
                        EdgeType::Required(_) => None,
                        EdgeType::DummyOptional => None,
                        EdgeType::DummyRequired => Some((e.source(), e.target())),
                    };
                }
                None
            });

            match es.next() {
                Some((s, t)) => {
                    //TODO becuase there is an edge between the two nodes, this is kept during merging.#
                    self.graph.remove_edge(self.graph.find_edge(s, t).unwrap());
                    self.merge(s, t);
                    continue;
                }
                None => {}
            }

            break;
        }

        //println!("{:?}", petgraph::dot::Dot::with_config(&self.graph, &[]))
    }
}
