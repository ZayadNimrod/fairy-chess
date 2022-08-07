pub mod movespec;
mod parser;

use movespec::MoveCompact;
use movespec::MoveGraph;
use petgraph::graph::IndexType;
use petgraph::graph::NodeIndex;

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

//TODO this can be an assosciated type with MoveGraph?
#[derive(Debug)]
pub struct MoveTrace<Ix> {
    pub current_move: NodeIndex<Ix>,
    pub current_position: (i32, i32),
    pub trace: Vec<(i32, i32, NodeIndex<Ix>)>, //TODO should be a forking list, not a vec?
}

/**
Assumes that target_position is not impassable (i.e open tile with no friendly piece)
*/
pub fn check_move<B, Ix>(
    piece: &MoveGraph<Ix>,
    board: &B,
    start_position: (i32, i32),
    target_position: (i32, i32),
) -> Option<MoveTrace<Ix>>
where
    B: Board,
    Ix: IndexType,
{
    //breadth-first search with a vector storing the points we have visited before (and therefore don't need to visit again)
    //using BFS rather than depth-first should mean we'll find the shortest route
    //(not necessarily, becuase of how choices aren't immediately evaluted, but it shouldnt be worst-case)
    //and don't have to pop off the vector of visited positions when we roll back the loop

    //We assume that board.tile_at() is cheap to call
    //TODO that might not be a good assumption, perhaps create a version of this algorithm that minimises such calls on the assumption it's expensive

    let mut traces: Vec<MoveTrace<Ix>> = vec![MoveTrace::<Ix> {
        current_move: piece.head(),
        current_position: start_position,
        trace: Vec::new(),
    }];

    let mut visited: Vec<(i32, i32, NodeIndex<Ix>)> = Vec::new();

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
                return Some(head);
            }
        }

        //if the next position is impassable, then we cannot continue on this trace; this is not a valid position to be in
        //unless this position is the target position; which it isn't, otherwise we would have retarned in the previous block
        //NOTE: this check breaks if we allow [0,0] moves; the parser blocks them, and the Move Graph constructor shouldn't create any
        if board.tile_at(head.current_position) == TileState::Impassable {
            continue;
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

        let next_moves = piece
            .all_outgoing(head.current_move)
            .map(|(n, e)| {
                //TODO this could be a lot simpler if we used a reverse linked list...

                let j = match e.weight() {
                    movespec::EdgeType::Optional(j) => j,
                    movespec::EdgeType::Required(j) => j,
                    movespec::EdgeType::DummyOptional | movespec::EdgeType::DummyRequired => {
                        return MoveTrace {
                            current_move: n,
                            current_position: head.current_position,
                            trace: head.trace.clone(),
                        };
                    }
                };

                let mut new_trace = head.trace.clone();

                new_trace.push((
                    head.current_position.0,
                    head.current_position.1,
                    head.current_move,
                ));

                let new_position = (head.current_position.0 + j.x, head.current_position.1 + j.y);
                MoveTrace {
                    current_move: n,
                    current_position: new_position,
                    trace: new_trace,
                }
            })
            .collect::<Vec<MoveTrace<Ix>>>();

        //eagerly follow Dummy edges. We need to do this, otherwise, we will prematurely stop following the trace due to the impassability check.
        //Consider a choice that is the last in the move sequence, that reaches the target position, which is impassable.
        //Becuase there is still a dummy edge ahead of it, it is not considered a finished move, so we don;t return true. Instead, we drop the trace, as we are on an impassable tile!
        //So we must eagerly follow them.
        let mut follow_up: Vec<MoveTrace<Ix>> = next_moves;

        //Follow up on dummy edges until there are no dummy edges left
        loop {
            let mut followed_up_on = false;
            let follow_up_next: Vec<MoveTrace<Ix>> = follow_up
                .iter()
                .flat_map(|mt| {
                    let hd = mt.current_move;

                    //if all outgoing edges are optional (or there are no outgoing edges), stay here. Otherwise, advance!

                    if !piece.all_outgoing(hd).any(|(_, e)| match e.weight() {
                        movespec::EdgeType::Optional(_) => true,
                        movespec::EdgeType::Required(_) => false,
                        movespec::EdgeType::DummyOptional => true,
                        movespec::EdgeType::DummyRequired => false,
                    }) {
                        //there exist at least one non-optionla egde. Therefore, we cannot stay here, so follow up required dummy edges
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
                            .collect::<Vec<MoveTrace<Ix>>>()
                    } else {
                        //all outgoing edges are optional, so just return self; we don't *have* to advance in any way, so return the current node
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

pub fn create_piece_simple(s: &str) -> Result<MoveCompact, PieceCreationError> {
    match parser::parse_string(s) {
        Ok(o) => Ok(o),
        Err(e) => Err(PieceCreationError::ParserError(e)),
    }
}

#[cfg(test)]
mod tests {

    use crate::{check_move, create_piece_simple, movespec::MoveGraph};

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
        let k = &MoveGraph::<u32>::from(create_piece_simple("[1,2]|-/").unwrap());
        assert!(check_move(k, board, (4, 4), (5, 6)).is_some());
    }
    #[test]
    fn knight() {
        let board = &TestBoard { x_max: 7, y_max: 7 };
        let k = &MoveGraph::<u32>::from(create_piece_simple("[1,2]|-/").unwrap());
        println!("{:?}", petgraph::dot::Dot::with_config(&(k.graph), &[]));
        let start_position = (4, 4);

        let points_r = (-2..=9).collect::<Vec<i32>>();

        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));

        let valids: Vec<(i32, i32)> = points
            .filter(|p| check_move(k, board, start_position, *p).is_some())
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
        let k = &MoveGraph::<u32>::from(create_piece_simple("[1,2]|-/").unwrap());
        let start_position = (1, 1);

        let points_r = (-2..=9).collect::<Vec<i32>>();

        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));

        let valids: Vec<(i32, i32)> = points
            .filter(|p| check_move(k, board, start_position, *p).is_some())
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
        let k = &MoveGraph::<u32>::from(create_piece_simple("[1,2]^*|-/").unwrap());
        println!("{:?}", petgraph::dot::Dot::with_config(&(k.graph), &[]));
        let start_position = (2, 2);

        let points_r = (0..=8).collect::<Vec<i32>>();

        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));

        let valids: Vec<(i32, i32)> = points
            .filter(|p| {
                println!("Calculating: {:?}", p);
                check_move(k, board, start_position, *p).is_some()
            })
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
        let piece = &MoveGraph::<u32>::from(create_piece_simple("{[1,0]/,[1,1]}|-^*").unwrap());
        let start_position = (1, 1);
        println!("{:?}", petgraph::dot::Dot::with_config(&(piece.graph), &[]));
        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));

        //piece should not be able to reach into the island due to blockages
        let invalids: Vec<(i32, i32)> = points
            .filter(|p| {
                println!("Calculating: {:?}", p);
                !check_move(piece, board, start_position, *p).is_some()
            })
            .collect();

        assert_eq!(invalids, vec![(4, 4)])
    }

    //TODO make sure to test something convoluted like two infinite exponentiations nested

    //TODO try a knightrider that is blocked at one point and therefore can't reach subsequent positions
}
