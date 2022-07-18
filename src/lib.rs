mod parser;
mod deflator;
pub use crate::deflator::Move;


pub enum PieceCreationError{
    ParserError(parser::ParsingError),
}
pub struct PieceDef{
}


pub struct Board{

}


pub fn check_move(p:PieceDef, b:Board, position:(usize,usize)) -> bool{
    todo!()
} 

pub fn convert_piece(s:&str) -> Result<PieceDef,PieceCreationError>{
    let converted = parser::parse_string(s);
    todo!()
}
