# Fairy Chess notation

## Informal introduction

### Jumps
The basis of the system shall be a *jump*. This forms an atomic move. This is an (x,y) displacement of the piece relative to it's current position. If said position is outside of board or otherwise unreachable (for example, a friendly piece occupies that space) then the peice cannot make this move. If making this move would result in the piece occupying the same space  as an enemy piece, the enemy piece is captured, and the moving piece takes its position.

The syntax of a jump shall be `[x,y]`, where `x` and `y` represent the x- and y- diplacement of the jump accordingly.


```
[-3,2]
```
![A piece that has one possible move - a jump 3 squares to the left and 2 forwards.](TODO)


### Options
Real-world pieces can make more than one move. To allow this, we group these moves together into a set (yes, this will be a set in the mathematical sense too). The piece can then make any one of the moves in the set.

The syntax of an option is `{a,b,...,c}` where the letter identifiers reprsent some further optionals. Note that an atomic jump can be treated as the option containing only itself.

```
{{[1,2],[2,1]},{[-1,2],[2,-1]},{[1,-2],[-2,1]},{[-1,-2],[-2,-1]}}
```

Is the same as:

```
{[1,2],[2,1],[-1,2],[2,-1],[1,-2],[-2,1],[-1,-2],[-2,-1]}
```
![The moves of a knight.](TODO)


### Sequences
Real pieces can make a chain of moves. To allow this, we must allow the concept of a sequence of jumps. This is a series of options. The piece must select a move from each option and make it in sequence, with the position it has reached in the previous jump being the start position for the next. If one of the jumps in the sequence cannot be made, then the entire complete sequence is illegal. In addition, if an enemy piece occupies a space that the piece would occupy during any of the intermediary jumps (but not after the last jump in sequence), then the move is, too, illegal. However, if all spaces the peice would occupy during the full move are free, except for the last, which is occupied by an enemy, the move is legal, becuause it is a capture.

As mentioned before, a sequence can be over a series of options. If an option is a set, this makes the sequence a multiplication of the two sets, where the result is the set of all possible combinations of one move (which can be iself a sequence!) from the first set, followed by one move from the second set (can also be a sequence). Therefore, the syntax is ` a*b` where `a` is the set of preceding moves, and `b` is the set of proceeding moves. 

TODO perhaps multiplcation could be `.` rather than  `*` to avoid ambiguity in syntax?

```
{[1,2],[2,1],[-1,2],[2,-1],[1,-2],[-2,1],[-1,-2],[-2,-1]} * [0,1]
```

![The moves of a piece that makes a knight move, then a single forward step like a pawn. Note that it cannot choose to make *only* the knight move, it *must* make the pawn move too: if this would be illegal, then it cannot make the preceding knight move! Similairly, it cannot make the pawn move without first making the knight move, which too must be a legal move, and furthermore, not a capture!](TODO)


### Repeated Sequences
What about pieces that make the same repeated move, like a bishop? This would be equivalent to `{move,move * move}`: The piece can either make the move, or make the move then follow it up with the move again. This is set exponentiation, so we use the syntax `^`.The second operatd of exponentiation is an integer, sych that `a^1` is equivalent to `a`, and `a^n` is equivalent to `{a*a^(n-1)}`.  However, this is not enough to create moves like bishops, so we will have to create a further construct on this.



```
{[1,1]^4,[-1,1]^4,[1,-1]^4,[1-,-1]^4}
```
![A pseudo-bishop, that can move 4 spaces in one diagonal, but can be blocked, and can only capture at distance 4](TODO)

Note that this is *not* the same as `{[1,1],[-1,1],[1,-1],[-1,-1]}^4`:
![The piece defined by `{[1,1],[-1,1],[1,-1],[-1,-1]}^4`; it makes 4 individual bishop hops of independent direction.](TODO)

#### Advanced exponentiation
We should also be able to input a range of values for eexponentiation. This would be done as `[x..y]`, where `x` and `y` are the upper and lower (inclusive) bounds. Exponentiating a move `m` in this way gives us the set of possible moves `{m^x,m^x+1,...m^y}`.

```
{[1,1]^[1..4],[-1,1]^[1..4],[1,-1]^[1..4],[1,-1]^[1..4]}
```
![A second pseudo-bishop, that can move up 4 spaces in one diagonal. Note that it can capture at any distance within range, unlike the previous pseudo-bishop.](TODO)

Note that this is *not* the same as `{[1,1],[-1,1],[1,-1],[-1,-1]}^[1..4]`:

There is also syntactical sugar for the `^[0..1]` exponent range; this is the question mark.

#### Exponentiation wildcards
What must the degree of exponentiation be for a piece that can make moves indevinately? Infinitely, of course! (Of course, we could use the size of the board to inform the degree, but this would preclude making use of the piece definition in a game with a larger board.) Therfore, we should allow the upper bound of the exponentoiation range be `*`, which means that the expression `m^[x..*]` will caluclate `{m^x,m^x+1,m^x+2.....}`. Note this set has infinite items, which must be accounted for by the evalutor.

As syntactical sugar, we can allow using the asterisk as an exponent outside of range syntax as sugar for `[1..*]`.


```
{[1,1]^*,[-1,1]^*,[1,-1]^*,[1,-1]^*}
```

![Finally, a true bishop!](TODO)

Note the above makes use of the syntactical sugar; without it, it would be `{[1,1]^[1..*],[-1,1]^[1..*],[1,-1]^[1..*],[1,-1]^[1..*]}`


### Mirrors
As a form of syntacal sugar, we will introduce mirror syntax. The first, `-`, the horizontal mirror, returns the move it was applied to, with the option of another move derived by inverted the y-compnent of the jump, i.e `[x,y]-` results in `{[x,y],[x,-y]}`. Similairly, `|` is the vertical mirror, such that `[x,y]|` results in `{[x,y],[-x,y]}`. Note that applying one of these modifiers to a set result in a set containing the elements of applying the modifier to each element in the original set. If they are applied to a sequence, there are two items in the output set; the original sequence, and the sequence frormed by mirroring each subsequence element. 

```
[1,0]^*|-
```
![A rook, in a more compact syntax.](TODO)


Next, we have the diagonal mirror, `/`. This swaps the x and y elements of the jump it is applied to. `[x,y]/` results in`{[x,y][y,x]}`. Otherwise, this follows the same rules as the other mirrors regarding sequences and options.

## Formal Syntax
```
Jump    ::= [Int,Int]

Option  ::= {OptionC}
            | Move
            | Jump

Seq     ::= Move * Move
            | Repeat

Repeat  ::= Option ^ Int
            | Option ^ [Int..Int]
            | Option ^ [Int..*]
            | Option ^ *
            | Option

Move    ::= (Move)
            | Seq
            | Move Mod

Mod     ::= -
            | |
            | /

OptionC ::= Move
            | Move , OptionC
```
