pub mod parser;


pub struct Piece{

}


pub struct Board{

}


pub fn check_move(p:Piece, b:Board, position:(usize,usize)) -> bool{
    todo!()
} 

pub fn convert_piece(s:&str) -> Piece{
    let converted = parser::parse_string(s);
    todo!()
}