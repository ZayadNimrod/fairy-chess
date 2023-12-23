use criterion::{criterion_group, criterion_main, Criterion};

fn knight_t_bench(c: &mut Criterion) {
    c.bench_function("knight_t", |b| b.iter(|| knight_t()));
}
fn knight_bench(c: &mut Criterion) {
    c.bench_function("knight", |b| b.iter(|| knight()));
}
fn knight_offset_bench(c: &mut Criterion) {
    c.bench_function("knight_offset", |b| b.iter(|| knight_offset()));
}
fn knightrider_bench(c: &mut Criterion) {
    c.bench_function("knightrider", |b| b.iter(|| knightrider()));
}
fn infinte_king_bench(c: &mut Criterion) {
    c.bench_function("infinte_king", |b| b.iter(|| infinte_king()));
}
fn skirmisher_bench(c: &mut Criterion) {
    c.bench_function("skirmisher", |b| b.iter(|| skirmisher()));
}
fn blocked_knightrider_bench(c: &mut Criterion) {
    c.bench_function("blocked_knightrider", |b| b.iter(|| blocked_knightrider()));
}
fn convoluted_bench(c: &mut Criterion) {
    c.bench_function("convoluted", |b| b.iter(|| convoluted()));
}

criterion_group!(
    benches,
    knight_t_bench,
    knight_bench,
    knight_offset_bench,
    knightrider_bench,
    infinte_king_bench,
    skirmisher_bench,
    blocked_knightrider_bench,
    convoluted_bench
);
criterion_main!(benches);

//copied from the tests in the main crate
use std::vec;

use fairy_chess::{check_move, create_piece, movespec::MoveGraph};

struct TestBoard {
    x_max: i32,
    y_max: i32,
}

impl fairy_chess::Board for TestBoard {
    fn tile_at(&self, position: (i32, i32)) -> fairy_chess::TileState {
        if position.0 > self.x_max || position.0 < 0 || position.1 > self.y_max || position.1 < 0 {
            return fairy_chess::TileState::Impassable;
        }
        fairy_chess::TileState::Empty
    }
}

fn knight_t() {
    let board = &TestBoard { x_max: 7, y_max: 7 };
    let k = &MoveGraph::<u32>::from(create_piece("[1,2]|-/").unwrap());
    assert!(check_move(k, board, (4, 4), (5, 6)).is_some());
}

fn knight() {
    let board = &TestBoard { x_max: 7, y_max: 7 };
    let k = &MoveGraph::<u32>::from(create_piece("[1,2]|-/").unwrap());
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

fn knight_offset() {
    let board = &TestBoard { x_max: 7, y_max: 7 };
    let k = &MoveGraph::<u32>::from(create_piece("[1,2]|-/").unwrap());
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

fn knightrider() {
    let board = &TestBoard { x_max: 8, y_max: 8 };
    let k = &MoveGraph::<u32>::from(create_piece("[1,2]^*|-/").unwrap());
    let start_position = (2, 2);

    let points_r = (0..=8).collect::<Vec<i32>>();

    let points = points_r
        .iter()
        .flat_map(|x| points_r.iter().map(|y| (*x, *y)));

    let valids: Vec<(i32, i32)> = points
        .filter(|p| check_move(k, board, start_position, *p).is_some())
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

impl fairy_chess::Board for DetailedTestBoard {
    fn tile_at(&self, position: (i32, i32)) -> fairy_chess::TileState {
        if self.grid.contains(&position) {
            return fairy_chess::TileState::Empty;
        } else {
            return fairy_chess::TileState::Impassable;
        }
    }
}

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
    let piece = &MoveGraph::<u32>::from(create_piece("{[1,0]/,[1,1]}|-^*").unwrap());
    let start_position = (1, 1);
    let points = points_r
        .iter()
        .flat_map(|x| points_r.iter().map(|y| (*x, *y)));

    //piece should not be able to reach into the island due to blockages
    let invalids: Vec<(i32, i32)> = points
        .filter(|p| !check_move(piece, board, start_position, *p).is_some())
        .collect();

    assert_eq!(invalids, vec![(4, 4)])
}

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
        let piece = &MoveGraph::<u32>::from(k);
        let start_position = (1, 1);

        let points = points_r
            .iter()
            .flat_map(|x| points_r.iter().map(|y| (*x, *y)));
        let valids: Vec<(i32, i32)> = points
            .filter(|p| check_move(piece, board, start_position, *p).is_some())
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
        )
    }
}

fn blocked_knightrider() {
    let piece = &MoveGraph::<u32>::from(create_piece("[1,2]^*/|-").unwrap());

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
        .filter(|p| check_move(piece, board, start_position, *p).is_some())
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

fn convoluted() {
    let piece = &MoveGraph::<u32>::from(create_piece("([2,2]^[2..*]-|/*[0,-4])^*").unwrap());

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
        .filter(|p| check_move(piece, board, start_position, *p).is_some())
        .collect();

    assert_eq!(valids, vec![(-1, 3), (3, 3), (7, 3), (9, 5), (11, 7)])
}
