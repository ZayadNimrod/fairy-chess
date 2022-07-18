pub mod movespec;
mod parser;
use movespec::Move;

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
    fn tile_at(position: (usize, usize)) -> TileState; //returns the state of the board
}

pub struct Piece {
    standard_move: Move,
    capture_move: Move,
}

pub fn check_move<B>(p: Piece, b: B, position: (usize, usize)) -> bool
where
    B: Board,
{
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
    let capture_move =  match parser::parse_string(capture) {
        Ok(o) => o,
        Err(e) => return Err(PieceCreationError::ParserError(e)),
    };

    Ok(Piece{
        standard_move,
        capture_move
    })
}
