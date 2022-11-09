pub mod movespec;
mod parser;

use std::rc::Rc;

use movespec::MoveCompact;
use movespec::MoveGraph;
use parser::Jump;
use petgraph::graph::{DefaultIx, NodeIndex};
use petgraph::stable_graph::EdgeReference;

#[derive(Debug)]
pub enum PieceCreationError {
    ParserError(parser::ParsingError),
}

#[derive(PartialEq, Clone, Copy)]
pub enum TileState {
    Empty,
    Impassable,
}

pub trait Board {
    fn tile_at(&self, position: (i32, i32)) -> TileState; //returns the state of the board
}

#[derive(Debug)]
struct MoveTrace<Ix> {
    pub current_move: NodeIndex<Ix>,
    pub current_position: (i32, i32),
    pub trace: Rc<Trace<(i32, i32)>>,
}

#[derive(Debug, Clone)]
enum Trace<T> {
    Root,
    Node(T, Rc<Trace<T>>),
}

impl<T> From<Trace<T>> for Vec<T>
where
    T: Copy + Default,
{
    fn from(trace: Trace<T>) -> Self {
        let depth: usize = {
            let mut c: usize = 0;
            let mut cur: &Trace<T> = &trace;
            loop {
                match cur {
                    Trace::Root => break,
                    Trace::Node(_, n) => {
                        cur = n;
                        c += 1;
                    }
                }
            }
            c
        };

        let mut output = vec![T::default(); depth]; //TODO I'd rather not even set a value here; I'm overwriting them anyway in a second!

        let mut cur = &trace;
        for i in (0..=depth - 1).rev() {
            match cur {
                Trace::Root => {
                    // should never happen
                    panic!()
                }
                Trace::Node(t, n) => {
                    output[i] = *t;
                    cur = n;
                }
            }
        }

        output
    }
}

/**
Assumes that target_position is not impassable (i.e open tile with no friendly piece)
*/
pub fn check_move<B>(
    piece: &MoveGraph,
    board: &B,
    start_position: (i32, i32),
    target_position: (i32, i32),
    invert_x: bool,
    invert_y: bool,
) -> Option<Vec<(i32, i32)>>
where
    B: Board,
{
    //breadth-first search with a vector storing the points we have visited before (and therefore don't need to visit again)
    //using BFS rather than depth-first should mean we'll find the shortest route
    //(not necessarily, becuase of how choices aren't immediately evaluted, but it shouldnt be worst-case)
    //and don't have to pop off the vector of visited positions when we roll back the loop

    //We assume that board.tile_at() is cheap to call
    //TODO that might not be a good assumption, perhaps create a version of this algorithm that minimises such calls on the assumption it's expensive

    let mut traces: Vec<MoveTrace<DefaultIx>> = vec![MoveTrace::<DefaultIx> {
        current_move: piece.head(),
        current_position: start_position,
        trace: Rc::new(Trace::Root),
    }];

    let mut visited: Vec<(i32, i32, NodeIndex<DefaultIx>)> = Vec::new();

    while let Some(head) = traces.pop() {
        //println!("testing:{:?}",head.current_position);
        if head.current_position == target_position {
            //println!("Possible target!");
            //we reached the target, check that we don't have further moves to make. If not, we've reached the target tile successfuly!
            if !piece
                .outgoing_edges(head.current_move)
                .any(|e| match e.weight() {
                    movespec::EdgeType::Optional(_) => false,
                    movespec::EdgeType::Required(_) => true,
                    movespec::EdgeType::DummyOptional => false,
                    movespec::EdgeType::DummyRequired => true,
                })
            {
                return Some(Vec::<(i32, i32)>::from(Trace::Node(
                    head.current_position,
                    head.trace,
                )));
            }
        }

        //test that this trace isn't in a loop
        if visited
            .iter()
            .any(|(x, y, mov)| (*x, *y) == head.current_position && *mov == head.current_move)
        {
            //this trace has already been at this location at the same point in the graph!
            //this means it has looped once, so delete it
            continue;
        }

        //if the next position is impassable, then we cannot continue on this trace; this is not a valid position to be in
        //unless this position is the target position; which it isn't, otherwise we would have retarned in the previous block
        //TODO don't like the fact that I have to collect the iterator halfway through
        let next_moves = {
            if board.tile_at(head.current_position) == TileState::Impassable {
                //however, it is entirely possible that we are here but there are required dummy nodes.
                //In which case, we can still continue on dummy nodes, but cannot on non-dummy nodes

                piece
                    .all_outgoing(head.current_move)
                    .filter(|(_, e)| match e.weight() {
                        movespec::EdgeType::Optional(_) => false,
                        movespec::EdgeType::Required(_) => false,
                        movespec::EdgeType::DummyOptional => true,
                        movespec::EdgeType::DummyRequired => true,
                    })
                    .collect::<Vec<(
                        NodeIndex<DefaultIx>,
                        EdgeReference<movespec::EdgeType, DefaultIx>,
                    )>>()
            } else {
                piece.all_outgoing(head.current_move).collect::<Vec<(
                    NodeIndex<DefaultIx>,
                    EdgeReference<movespec::EdgeType, DefaultIx>,
                )>>()
            }
        }
        .iter()
        .map(|(n, e)| {
            let mut j :Jump = match e.weight() {
                movespec::EdgeType::Optional(j) => Jump{x:j.x,y:j.y},
                movespec::EdgeType::Required(j) => Jump{x:j.x,y:j.y},
                movespec::EdgeType::DummyOptional | movespec::EdgeType::DummyRequired => {
                    return MoveTrace {
                        current_move: *n,
                        current_position: head.current_position,
                        trace: head.trace.clone(),
                    };
                }
            };

            if invert_x {
                j.x = -j.x;
            }

            if invert_y {
                j.y = -j.y;
            }

            let new_trace = Rc::new(Trace::Node(
                (head.current_position.0, head.current_position.1),
                head.trace.clone(),
            ));

            let new_position = (head.current_position.0 + j.x, head.current_position.1 + j.y);
            MoveTrace {
                current_move: *n,
                current_position: new_position,
                trace: new_trace,
            }
        })
        .collect();

        //eagerly follow Dummy edges. We need to do this, otherwise, we will prematurely stop following the trace due to the impassability check.
        //Consider a choice that is the last in the move sequence, that reaches the target position, which is impassable.
        //Becuase there is still a dummy edge ahead of it, it is not considered a finished move, so we don;t return true. Instead, we drop the trace, as we are on an impassable tile!
        //So we must eagerly follow them.
        let mut follow_up: Vec<MoveTrace<DefaultIx>> = next_moves;
        //Follow up on dummy edges until there are no dummy edges left
        loop {
            let mut followed_up_on = false;
            let follow_up_next: Vec<MoveTrace<DefaultIx>> = follow_up
                .iter()
                .flat_map(|mt| {
                    let hd = mt.current_move;

                    //if all outgoing edges are optional or non-dummy (or there are no outgoing edges), stay here. Otherwise, advance!

                    if !piece.all_outgoing(hd).any(|(_, e)| match e.weight() {
                        movespec::EdgeType::Optional(_) => true,
                        movespec::EdgeType::Required(_) => true,
                        movespec::EdgeType::DummyOptional => true,
                        movespec::EdgeType::DummyRequired => false,
                    }) {
                        //there exist at least one non-optional egde. Therefore, we cannot stay here, so follow up required dummy edges
                        piece
                            .all_outgoing(hd)
                            .map(|(n, e)| match e.weight() {
                                movespec::EdgeType::DummyRequired => {
                                    followed_up_on = true;
                                    MoveTrace {
                                        current_move: n,
                                        current_position: mt.current_position,
                                        trace: mt.trace.clone(),
                                    }
                                }

                                _ => MoveTrace {
                                    current_move: hd,
                                    current_position: mt.current_position,
                                    trace: mt.trace.clone(),
                                },
                            })
                            .collect::<Vec<MoveTrace<DefaultIx>>>()
                    } else {
                        //all outgoing edges are optional or non-dummy, so just return self; we don't *have* to advance in any way, so return the current node
                        vec![MoveTrace {
                            current_move: hd,
                            current_position: mt.current_position,
                            trace: mt.trace.clone(),
                        }]
                    }
                })
                .collect();

            if !followed_up_on {
                break;
            }

            follow_up = follow_up_next;
        }

        visited.push((
            head.current_position.0,
            head.current_position.1,
            head.current_move,
        ));
        traces.append(&mut follow_up);
    }

    //no trace found a path to the target, no path could exist!
    None
}

pub fn create_piece(s: &str) -> Result<MoveCompact, PieceCreationError> {
    match parser::parse_string(s) {
        Ok(o) => Ok(o),
        Err(e) => Err(PieceCreationError::ParserError(e)),
    }
}

#[cfg(test)]
mod tests {

    use std::vec;

    use crate::{check_move, create_piece, movespec::MoveGraph};

    struct TestBoard {
        x_max: i32,
        y_max: i32,
    }

    impl crate::Board for TestBoard {
        fn tile_at(&self, position: (i32, i32)) -> crate::TileState {
            if position.0 > self.x_max
                || position.0 < 0
                || position.1 > self.y_max
                || position.1 < 0
            {
                return crate::TileState::Impassable;
            }
            crate::TileState::Empty
        }
    }

    #[test]
    fn knight_t() {
        let board = &TestBoard { x_max: 7, y_max: 7 };
        let k = &MoveGraph::from(create_piece("[1,2]|-/").unwrap());
        let result = check_move(k, board, (4, 4), (5, 6), false, false);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), [(4, 4), (5, 6)])
    }

    #[test]
    fn knight() {
        let board = &TestBoard { x_max: 7, y_max: 7 };
        let k = &MoveGraph::from(create_piece("[1,2]|-/").unwrap());
        let start_position = (4, 4);

        let points_r = (-2..=9).collect::<Vec<i32>>();

        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));

        let valids: Vec<(i32, i32)> = points
            .filter(|p| check_move(k, board, start_position, *p, false, false).is_some())
            .collect();

        assert_eq!(
            valids,
            vec![
                (2, 3),
                (2, 5),
                (3, 2),
                (3, 6),
                (5, 2),
                (5, 6),
                (6, 3),
                (6, 5)
            ]
        )
    }

    #[test]
    fn knight_offset() {
        let board = &TestBoard { x_max: 7, y_max: 7 };
        let k = &MoveGraph::from(create_piece("[1,2]|-/").unwrap());
        let start_position = (1, 1);

        let points_r = (-2..=9).collect::<Vec<i32>>();

        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));

        let valids: Vec<(i32, i32)> = points
            .filter(|p| check_move(k, board, start_position, *p, false, false).is_some())
            .collect();

        assert_eq!(
            valids,
            vec![
                (-1, 0),
                (-1, 2),
                (0, -1),
                (0, 3),
                (2, -1),
                (2, 3),
                (3, 0),
                (3, 2)
            ]
        )
    }

    #[test]
    fn knightrider() {
        let board = &TestBoard { x_max: 8, y_max: 8 };
        let k = &MoveGraph::from(create_piece("[1,2]^*|-/").unwrap());
        let start_position = (2, 2);

        let points_r = (0..=8).collect::<Vec<i32>>();

        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));

        let valids: Vec<(i32, i32)> = points
            .filter(|p| check_move(k, board, start_position, *p, false, false).is_some())
            .collect();

        assert_eq!(
            valids,
            vec![
                (0, 1),
                (0, 3),
                (0, 6),
                (1, 0),
                (1, 4),
                (3, 0),
                (3, 4),
                (4, 1),
                (4, 3),
                (4, 6),
                (5, 8),
                (6, 0),
                (6, 4),
                (8, 5)
            ]
        )
    }

    struct DetailedTestBoard {
        grid: Vec<(i32, i32)>,
    }

    impl crate::Board for DetailedTestBoard {
        fn tile_at(&self, position: (i32, i32)) -> crate::TileState {
            if self.grid.contains(&position) {
                return crate::TileState::Empty;
            } else {
                return crate::TileState::Impassable;
            }
        }
    }
    #[test]
    fn infinte_king() {
        let points_r = (-1..=9).collect::<Vec<i32>>();

        //infinite king should not be able to reach an island that is surrounded by impassable squares
        let grid_points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)))
            .filter(|x| {
                !matches!(
                    x,
                    (5, 5) | (5, 4) | (5, 3) | (4, 3) | (3, 3) | (3, 4) | (3, 5) | (4, 5)
                )
            })
            .collect::<Vec<(i32, i32)>>();

        let board = &DetailedTestBoard { grid: grid_points };
        let piece = &MoveGraph::from(create_piece("{[1,0]/,[1,1]}|-^*").unwrap());
        let start_position = (1, 1);
        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));

        //piece should not be able to reach into the island due to blockages
        let invalids: Vec<(i32, i32)> = points
            .filter(|p| !check_move(piece, board, start_position, *p, false, false).is_some())
            .collect();

        assert_eq!(invalids, vec![(4, 4)])
    }

    #[test]
    fn skirmisher() {
        let points_r = (0..=9).collect::<Vec<i32>>();
        let grid_points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)))
            .filter(|x| !matches!(x, (2, 3) | (3, 3)))
            .collect::<Vec<(i32, i32)>>();

        let board = &DetailedTestBoard { grid: grid_points };
        //a knight that can optionally make a single hop forwards
        for s in vec!["[1,2]|-/*[0,1]^[0..1]", "[1,2]|-/*[0,1]?"] {
            //thse two pieces should be the same, just syntactcial sugar
            let k = create_piece(s).unwrap();
            let piece = &MoveGraph::from(k);
            println!("{:?}", petgraph::dot::Dot::with_config(&piece.graph, &[]));
            println!("head:{:?}", piece.head());

            let start_position = (1, 1);

            let points = points_r
                .iter()
                .flat_map(|x| points_r.iter().map(|y| (*x, *y)));
            let valids: Vec<(i32, i32)> = points
                .filter(|p| {
                    println!("{:#?}", p);
                    check_move(piece, board, start_position, *p, false, false).is_some()
                })
                .collect();

            assert_eq!(
                valids,
                vec![
                    (0, 3),
                    (0, 4),
                    (2, 3),
                    (3, 0),
                    (3, 1),
                    (3, 2),
                    (3, 3) //can't go to (2,4) due to blocked (2,3)
                ]
            );
        }
    }

    //a knightrider that is blocked at some points and therefore can't reach subsequent positions
    #[test]
    fn blocked_knightrider() {
        let piece = &MoveGraph::from(create_piece("[1,2]^*/|-").unwrap());

        let points_r = (0..=13).collect::<Vec<i32>>();
        let grid_points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)))
            .filter(|x| !matches!(x, (9, 7) | (11, 12) | (3, 13) | (3, 8))) //blocking pieces on (9,7),(11,12),(3,13),(3,8)
            .collect::<Vec<(i32, i32)>>();

        let board = &DetailedTestBoard { grid: grid_points };

        let start_position = (5, 9);

        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));
        let valids: Vec<(i32, i32)> = points
            .filter(|p| check_move(piece, board, start_position, *p, false, false).is_some())
            .collect();

        assert_eq!(
            valids,
            vec![
                (1, 1),
                (1, 11),
                (2, 3),
                (3, 5),
                (3, 8),
                (3, 10),
                (3, 13),
                (4, 7),
                (4, 11),
                (6, 7),
                (6, 11),
                (7, 5),
                (7, 8),
                (7, 10),
                (7, 13),
                (8, 3),
                (9, 1),
                (9, 7),
                (9, 11),
                (11, 12) //can't reach (1,7),(11,6),(13,5),(13,13) due to blocking tiles
            ]
        )
    }

    #[test]
    fn convoluted() {
        let piece = &MoveGraph::from(create_piece("([2,2]^[2..*]-|/*[0,-4])^*").unwrap());

        println!("{:?}", petgraph::dot::Dot::with_config(&piece.graph, &[]));
        println!("head:{:?}", piece.head());
        let points_r = (-1..=11).collect::<Vec<i32>>();
        let grid_points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)))
            .filter(|x| !matches!(x, (1, 9) | (3, 11) | (5, 1) | (5, 9) | (9, 1) | (11, 7))) //blocking pieces
            .collect::<Vec<(i32, i32)>>();

        let board = &DetailedTestBoard { grid: grid_points };

        let start_position = (7, 3);

        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));
        let valids: Vec<(i32, i32)> = points
            .filter(|p| check_move(piece, board, start_position, *p, false, false).is_some())
            .collect();

        //println!("{:?}", check_move(piece, board, start_position, (1, 1)));

        assert_eq!(valids, vec![(-1, 3), (3, 3), (7, 3), (9, 5), (11, 7)])
    }
    #[test]
    fn invert() {
        let piece = &MoveGraph::from(create_piece("[1,1]").unwrap());

        let points_r = (-1..=11).collect::<Vec<i32>>();
        let grid_points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)))
            .filter(|x| !matches!(x, (1, 9) | (3, 11) | (5, 1) | (5, 9) | (9, 1) | (11, 7))) //blocking pieces
            .collect::<Vec<(i32, i32)>>();

        let board = &DetailedTestBoard { grid: grid_points };

        let start_position = (6, 3);

        assert!(check_move(piece, board, start_position, (7, 4), false, false).is_some());
        assert!(!check_move(piece, board, start_position, (6, 4), false, false).is_some());
        assert!(!check_move(piece, board, start_position, (5, 4), false, false).is_some());
        assert!(!check_move(piece, board, start_position, (7, 3), false, false).is_some());
        assert!(!check_move(piece, board, start_position, (6, 3), false, false).is_some());
        assert!(!check_move(piece, board, start_position, (5, 3), false, false).is_some());
        assert!(!check_move(piece, board, start_position, (7, 2), false, false).is_some());
        assert!(!check_move(piece, board, start_position, (6, 2), false, false).is_some());
        assert!(!check_move(piece, board, start_position, (5, 2), false, false).is_some());

        assert!(!check_move(piece, board, start_position, (7, 4), true, false).is_some());
        assert!(!check_move(piece, board, start_position, (6, 4), true, false).is_some());
        assert!(check_move(piece, board, start_position, (5, 4), true, false).is_some());
        assert!(!check_move(piece, board, start_position, (7, 3), true, false).is_some());
        assert!(!check_move(piece, board, start_position, (6, 3), true, false).is_some());
        assert!(!check_move(piece, board, start_position, (5, 3), true, false).is_some());
        assert!(!check_move(piece, board, start_position, (7, 2), true, false).is_some());
        assert!(!check_move(piece, board, start_position, (6, 2), true, false).is_some());
        assert!(!check_move(piece, board, start_position, (5, 2), true, false).is_some());

        assert!(!check_move(piece, board, start_position, (7, 4), false, true).is_some());
        assert!(!check_move(piece, board, start_position, (6, 4), false, true).is_some());
        assert!(!check_move(piece, board, start_position, (5, 4), false, true).is_some());
        assert!(!check_move(piece, board, start_position, (7, 3), false, true).is_some());
        assert!(!check_move(piece, board, start_position, (6, 3), false, true).is_some());
        assert!(!check_move(piece, board, start_position, (5, 3), false, true).is_some());
        assert!(check_move(piece, board, start_position, (7, 2), false, true).is_some());
        assert!(!check_move(piece, board, start_position, (6, 2), false, true).is_some());
        assert!(!check_move(piece, board, start_position, (5, 2), false, true).is_some());
    }
}
