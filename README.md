# fairy-chess

Yes, you can pull crates from GitHub. Put in `cargo.toml`:

```
[dependencies]
fairy-chess = { git = "https://github.com/ZayadNimrod/fairy-chess"}
```


## Interface
First, you'll want to convert your movespec strings (defined in the DSL) into an AST called `MoveCompact`. This can be freely converted back to a string if you want to serialize it that way. This is done though `fairy_chess::create_piece(string)`. After handling errors, you can then turn this into a `MoveGraph` with `fairy_chess::movespec::MoveGraph::from(MoveCompact)`. This is the data structure that needs to be passed to `check_move`. It's also deflated to be as small a graph as possible.

Next, you'll need a `fairy_chess::Board` implementation. This requires defining the `tile_at(&self, position: (i32, i32)) -> fairy_chess::TileState`. The `fairy_chess::TileState` enum represents the state of the tile at the supplied position. This can be `Empty` or `Impassable` - the latter reprsents the case where there is a piece on the tile, or the tile is out of bounds for whatever reason, or any other reason that would make the tile "non-free" - this is of course, specific to your game.

Now, we can call `check_move`! This is the meat of the library. This requires passing the `MoveGraph` of the piece that is being moved, the `Board` implementor representing the current state of the board, and the start and end positions of the desired move. Note that we assume that the target position is a legal move target; so you should make out-of-bounds checks before calling this function. There isn't a problem if there is a piece on the tile, that just means you're making a capture. You should however check that the piece of the tile is capturable by the rules of your game, or the `check_move` will say that capturing it is a legal move, even if your game does not allow for this. Finally, there exist two boolean flags, `invert_x` and `invert_y`. Passing these allow you to process the move as if the passed move's atomic jumps had thier x or y components's sign flipped. This is so that you can use the same piece spec for pieces of the same type but are on opposing sides - without this, a black pawn and a white pawn would need seperate move specs.

`check_move` returns an `Option<Vec<i32,i32>>`. If the `Option` is `None`, then the move is illegal. Each element in the `Vec` returned by a legal move is a tuple representing the (x,y) coordinates of every position visited between the chained atomic jumps of the move. For example, a rook move will return all the tiles between the rook's start and end positions, while a knightrider move will return the end tiles of each intermediate knight move. You can use this for the purposes of animation or otherwise showing the structure of the move to your users.




## Langauge
Please refer to [the language spec](specification.md) for documentation on the language itself.
