pub mod movespec;
mod parser;

use movespec::MoveCompact;
use movespec::MoveGraph;
pub enum PieceCreationError {
    ParserError(parser::ParsingError),
}

#[derive(PartialEq)]
pub enum TileState {
    Empty,
    Impassable,
}

pub trait Board {
    fn tile_at(&self, position: (i32, i32)) -> TileState; //returns the state of the board
}

struct MoveTrace {
    current_move: petgraph::graph::DefaultIx,
    current_position: (i32, i32),
    trace: Vec<(i32, i32, petgraph::graph::DefaultIx)>, //TODO should be an index type of MoveGraph //TODO should be a forking list, not a vec?
}

/**
Assumes that target_position is not impassable (i.e open tile with no friendly piece)
*/
pub fn check_move<B>(
    piece: MoveGraph,
    board: B,
    start_position: (i32, i32),
    target_position: (i32, i32),
) -> bool
where
    B: Board,
{
    //breadth-first search with a vector storing the points we have visited before (and therefore don't need to visit again)
    //using BFS rather than depth-first should mean we'll find the shortest route
    //(not necessarily, becuase of how choices aren't immediately evaluted, but it shouldnt be worst-case)
    //and don't have to pop off the vector of visited positions when we roll back the loop

    //We assume that board.tile_at() is cheap to call
    //TODO that might not be a good assumption, perhaps create a version of this algorithm that minimises such calls on the assumption it's expensive

    let mut traces: Vec<MoveTrace> = vec![MoveTrace {
        current_move: piece.head(),
        current_position: start_position,
        trace: Vec::new(),
    }];

    while let Some(head) = traces.pop() {
        if head.current_position == target_position {
            //we reached the target, check that we don't have further moves to make. If not, we've reached the target tile successfuly!
            if !piece
                .outgoing_edges(head.current_move)
                .any(|e| *e.weight() == movespec::EdgeType::Required)
            {
                //TODO return trace
                return true;
            }
        }

        let j = piece.jump_at(head.current_move);
        let new_position = (head.current_position.0 + j.x, head.current_position.1 + j.y);
        //if this new position is impassable, then we cannot continue on this trace
        if board.tile_at(new_position) == TileState::Impassable {
            continue;
        }

        //test that this trace isn't in a loop
        if head
            .trace
            .iter()
            .any(|(x, y, mov)| (*x, *y) == head.current_position && *mov == head.current_move)
        {
            //this trace has already been at this location at the same point in the graph!
            //this means it has looped once, so delete it
            continue;
        }

        //append the next moves to the
        //TODO maximise the amount we consume! If there is one one sucessor and it is required, don't do the bullshit of pushing it onto the trace pile, JUST PROCESS IT EAGERLY
        let mut next_moves = piece
            .successors(head.current_move)
            .map(|n| {
                //TODO this could be a lot simpler if we used a reverse linked list...
                let mut new_trace = head.trace.clone();
                new_trace.push((
                    head.current_position.0,
                    head.current_position.1,
                    head.current_move,
                ));

                MoveTrace {
                    current_move: n,
                    current_position: new_position,
                    trace: new_trace,
                }
            })
            .collect::<Vec<MoveTrace>>();

        traces.append(&mut next_moves);
    }

    //no trace found a path to the target, no path could exist!
    false
}

pub fn create_piece_simple(s: &str) -> Result<MoveCompact, PieceCreationError> {
    match parser::parse_string(s) {
        Ok(o) => Ok(o),
        Err(e) => Err(PieceCreationError::ParserError(e)),
    }
}
