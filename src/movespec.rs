use std::iter::Zip;

use petgraph::graph::{DefaultIx, NodeIndex};
use petgraph::stable_graph::EdgeReference;
use petgraph::visit::{EdgeRef, IntoEdges, IntoNeighbors};
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
    pub(crate) fn notation(&self) -> String {
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

#[cfg(feature = "serde")]
impl serde::Serialize for MoveCompact {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.notation())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for MoveCompact {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.try_into().map_err(serde::de::Error::custom)
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

#[derive(Debug)]
pub struct MoveGraph<Ix: petgraph::adj::IndexType = DefaultIx> {
    pub graph: petgraph::stable_graph::StableDiGraph<(), EdgeType, Ix>,
    head: NodeIndex<DefaultIx>,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum EdgeType {
    Jump(Jump),
    DummyOptional,
    DummyRequired,
}

impl From<&MoveCompact> for MoveGraph {
    fn from(input: &MoveCompact) -> Self {
        let mut r = MoveGraph {
            graph: petgraph::stable_graph::StableDiGraph::<(), EdgeType, DefaultIx>::with_capacity(
                0, 0,
            ),
            head: NodeIndex::<DefaultIx>::default(),
        };
        let (h, _) = r.build_from_node(input);
        r.head = h;
        r.deflate();
        r
    }
}

// Is this actually necessary? Would have thought there'd be a blanket impl From<T> when you have From<&T>
impl From<MoveCompact> for MoveGraph {
    fn from(input: MoveCompact) -> Self {
        (&input).into()
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

impl MoveGraph {
    fn build_from_node(
        &mut self,
        node: &MoveCompact,
    ) -> (NodeIndex<DefaultIx>, NodeIndex<DefaultIx>) {
        match node {
            MoveCompact::Jump(j) => {
                let h = self.graph.add_node(());
                let t = self.graph.add_node(());
                self.graph.add_edge(h, t, EdgeType::Jump(*j));
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
                    .collect::<Vec<NodeIndex<DefaultIx>>>()[0];

                (head_idx, tail_idx)
            }
            MoveCompact::Modded(mov, modifier) => self.build_from_mod(mov, modifier),
        }
    }

    fn build_from_mod(
        &mut self,
        mov: &MoveCompact,
        modifier: &Mod,
    ) -> (NodeIndex<DefaultIx>, NodeIndex<DefaultIx>) {
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
                let (h, t_mid) = self.build_from_mod(mov, &Mod::Exponentiate(*min - 1));
                let (h_mid, t) = self.build_from_node(mov);
                self.graph.add_edge(t, h_mid, EdgeType::DummyOptional);
                self.merge(h_mid, t_mid);
                (h, t)
            }
        }
    }

    fn merge(&mut self, to_keep: NodeIndex<DefaultIx>, to_drop: NodeIndex<DefaultIx>) {
        let drop_outgoing: Vec<(NodeIndex<DefaultIx>, EdgeType)> = self
            .graph
            .edges_directed(to_drop, EdgeDirection::Outgoing)
            .map(|r| (r.target(), *r.weight()))
            .collect();

        for (target, weight) in drop_outgoing {
            self.graph.add_edge(to_keep, target, weight);
        }

        let drop_incoming: Vec<(NodeIndex<DefaultIx>, EdgeType)> = self
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
        idx: NodeIndex<DefaultIx>,
    ) -> <&petgraph::stable_graph::StableDiGraph<(), EdgeType, DefaultIx> as IntoNeighbors>::Neighbors
    {
        self.graph.neighbors(idx)
    }

    pub fn outgoing_edges(
        &self,
        idx: NodeIndex<DefaultIx>,
    ) -> <&petgraph::stable_graph::StableDiGraph<(), EdgeType, DefaultIx> as IntoEdges>::Edges {
        self.graph.edges(idx)
    }

    pub fn all_outgoing(
        &self,
        idx: NodeIndex<DefaultIx>,
    ) -> Zip<
        <&petgraph::stable_graph::StableDiGraph<(), EdgeType, DefaultIx> as IntoNeighbors>::Neighbors,
        <&petgraph::stable_graph::StableDiGraph<(), EdgeType, DefaultIx> as IntoEdges>::Edges,
    >{
        self.successors(idx).zip(self.outgoing_edges(idx))
    }

    pub fn head(&self) -> NodeIndex<DefaultIx> {
        self.head
    }

    //TODO consider deflating by combining identical subgraphs

    ///deflate the graph by removing superfluous nodes
    pub fn deflate(&mut self) {
        //TODO this loop is probably not the most efficient way to solve this problem...
        loop {
            //Find a reason to merge nodes

            let mut edge = None;

            //Reason 2: If there are outgoing required dummy edges, where  the nodes these lead to have no extra incoming edges (beyond the ones to the current node)
            //then we can merge said children nodes the the current node
            //i.e merge multi-layer choice nodes

            edge = edge.or_else(|| {
                self.graph.node_indices().find_map(|n| {
                    let mergable = self
                        .outgoing_edges(n)
                        .filter(
                            //filter out non-dummy edges
                            |e| match e.weight() {
                                EdgeType::Jump(_) => false,
                                EdgeType::DummyOptional => false,
                                EdgeType::DummyRequired => true,
                            },
                        )
                        .filter(
                            //filter out child nodes that have >1 incoming edge
                            |e| {
                                self.graph
                                    .edges_directed(e.target(), EdgeDirection::Incoming)
                                    .count()
                                    == 1
                            },
                        )
                        .filter(
                            //filter out child nodes that have an edge to the parent
                            |e| {
                                self.graph
                                    .edges_directed(e.target(), EdgeDirection::Outgoing)
                                    .all(|ed| ed.target() != e.source())
                            },
                        );

                    //return the edges to the mergable nodes
                    let es = mergable
                        .map(|e| (e.source(), e.target()))
                        .collect::<Vec<(NodeIndex<DefaultIx>, NodeIndex<DefaultIx>)>>();
                    match es.len() {
                        0 => None,
                        _ => Some(es),
                    }
                })
                //can only merge into one node at a time, to prevent mergeing into a node that has been merged away
            });
            //the above could return several edges to remove at once; they should be mutually removable

            //Reason 1: if there is only one outgoing edge, and it is a dummy type, we can merge the nodes
            edge = edge.or_else(|| {
                let a = self
                    .graph
                    .node_indices()
                    .filter_map(|n| {
                        let es: Vec<EdgeReference<EdgeType, DefaultIx>> =
                            self.outgoing_edges(n).collect();
                        if es.len() == 1 {
                            let e = es[0];
                            return match e.weight() {
                                EdgeType::Jump(_) => None,
                                EdgeType::DummyOptional => None,
                                EdgeType::DummyRequired => Some((e.source(), e.target())),
                            };
                        }
                        None
                    })
                    .take(1)
                    //this sort of removal can only allow for one removal at a time, otherwise we may try to merge into a node that has already been deleted
                    .collect::<Vec<(NodeIndex<DefaultIx>, NodeIndex<DefaultIx>)>>();

                match a.len() {
                    0 => None,
                    _ => Some(a),
                }
            });

            //remove an edge
            match edge {
                Some(vec) => {
                    vec.into_iter().for_each(|(s, t)| {
                        //there is an edge between the two nodes, this is kept during merging, so remove it first
                        self.graph.remove_edge(self.graph.find_edge(s, t).unwrap());
                        self.merge(s, t);
                    });
                    continue;
                }
                None => break,
            }
        }

        //println!("{:?}", petgraph::dot::Dot::with_config(&self.graph, &[]))
    }
}
