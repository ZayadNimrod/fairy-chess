pub mod movespec;
mod parser;

use movespec::MoveCompact;

pub enum PieceCreationError {
    ParserError(parser::ParsingError),
}

pub enum TileState {
    Empty,
    Impassable,
    CaptureOnly,
}

//TODO does not consider team, should that be a thing we do, or leave up to the user to implement?
pub trait Board {
    fn tile_at(&self, position: (i32, i32)) -> TileState; //returns the state of the board
}

//TODO should Piece be a trait instead?
pub struct Piece {
    standard_move: MoveCompact,
    capture_move: MoveCompact,
}

struct MoveTrace {
    current_move: Option<MoveCompact>,
    current_position: (i32, i32),
    trace: Vec<(i32, i32)>,
}

pub fn check_move<B>(
    piece: Piece,
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

   
    todo!()
}

pub fn create_piece_simple(s: &str) -> Result<Piece, PieceCreationError> {
    create_piece_complex(s, s)
}

pub fn create_piece_complex(standard: &str, capture: &str) -> Result<Piece, PieceCreationError> {
    let standard_move = match parser::parse_string(standard) {
        Ok(o) => o,
        Err(e) => return Err(PieceCreationError::ParserError(e)),
    };
    let capture_move = match parser::parse_string(capture) {
        Ok(o) => o,
        Err(e) => return Err(PieceCreationError::ParserError(e)),
    };

    Ok(Piece {
        standard_move,
        capture_move,
    })
}
