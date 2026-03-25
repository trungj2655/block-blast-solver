use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
use clap::{Parser, ValueEnum};
use ndarray::prelude::*;
use scan_rules::*;
use std::collections::BTreeSet;
use std::io::{BufRead as _, IsTerminal as _, stdin};
use tracing::*;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::time::Uptime;
/// right, up, left, down
const LATERAL: [(isize, isize); 4] = [(0, 1), (-1, 0), (0, -1), (1, 0)];
/// up right, up left, down left, down right
const DIAGONAL: [(isize, isize); 4] = [(-1, 1), (-1, -1), (1, -1), (1, 1)];
/// counterclockwise starting from right
const KNIGHT: [(isize, isize); 8] = [
    (-1, 2),
    (-2, 1),
    (-2, -1),
    (-1, -2),
    (1, -2),
    (2, -1),
    (2, 1),
    (1, 2),
];
/// Lookup table use to determine which box-drawing character to print
/// 0: ─
/// 1: │
/// 2: ┌
/// 3: ┐
/// 4: └
/// 5: ┘
const LOOKUP: [[usize; 4]; 4] = [[0, 5, 0, 3], [2, 1, 3, 1], [0, 4, 0, 2], [4, 1, 5, 1]];
const DIRECTIONS: [char; 8] = ['.', '#', 'S', 'E', '→', '↑', '←', '↓'];
const CONNECTED: [char; 10] = ['.', '#', 'S', 'E', '─', '│', '┌', '┐', '└', '┘'];
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Moves {
    /// horizontal or vertical (right, up, left, down)
    Lateral,
    /// up right, up left, down left, down right
    Diagonal,
    /// counterclockwise starting from right
    Knight,
}
#[derive(Parser, Debug, Clone)]
#[command(version, about = "Hamiltonian path solver", long_about = None)]
struct Args {
    #[arg(short, long, value_enum, ignore_case = true, num_args = 1.., default_values_t = [Moves::Lateral], help = "Specify which moves the path can make")]
    moves: Vec<Moves>,
}
impl Args {
    /// Returns a unique, sorted set of selected moves
    pub fn get_moves(&self) -> BTreeSet<Moves> {
        self.moves.iter().copied().collect()
    }
}
#[derive(Copy, Clone, Debug)]
struct State {
    /// row position
    r: usize,
    /// column position
    c: usize,
    /// directions index
    dir: usize,
}
#[instrument(skip(grid))]
fn find_hamiltonian_path(
    rows: usize,
    cols: usize,
    grid: &mut Array2<usize>,
    start_r: usize,
    start_c: usize,
    total_vertices: usize,
    directions: &[(isize, isize)],
) -> Option<Vec<State>> {
    let n = directions.len();
    let mut path_length = 1_usize;
    let mut path: Vec<State> = vec![State { r: 0, c: 0, dir: 0 }; total_vertices];
    path[0] = State {
        r: start_r,
        c: start_c,
        dir: 0,
    };
    grid[[start_r, start_c]] = 2;
    loop {
        if path_length == total_vertices {
            return Some(path);
        }
        let (left, right) = path.split_at_mut(path_length);
        let cur = &mut left[path_length - 1];
        let (cr, cc, cdir) = (cur.r, cur.c, &mut cur.dir);
        loop {
            if *cdir == n {
                grid[[cr, cc]] = 0;
                path_length -= 1;
                if path_length == 0 {
                    return None;
                }
                path[path_length - 1].dir += 1;
                break;
            }
            if let Some(next_r) = cr.checked_add_signed(directions[*cdir].0)
                && let Some(next_c) = cc.checked_add_signed(directions[*cdir].1)
                && next_r < rows
                && next_c < cols
                && grid[[next_r, next_c]] == 0
            {
                right[0] = State {
                    r: next_r,
                    c: next_c,
                    dir: 0,
                };
                grid[[next_r, next_c]] = 2;
                path_length += 1;
                break;
            }
            *cdir += 1;
        }
    }
}
fn main() {
    let args = Args::parse();
    assert!(!args.moves.is_empty(), "moves shouldn't be empty!");
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_timer(Uptime::default())
        .init();
    let _main_span = info_span!("main").entered();
    let term = stdin().is_terminal();
    debug!(?args, ?term);
    let (mut start_r, mut start_c) = (0_usize, 0_usize);
    let mut start_found = false;
    let (rows, cols) = loop {
        if term {
            print!("Enter the grid dimensions (rows by columns): ");
        }
        let result = try_readln! {
            (let rows: usize, let cols: usize) => (rows, cols)
        };
        match result {
            Ok((rows, cols)) => {
                if rows == 0 || cols == 0 {
                    error!(?rows, ?cols, "Invalid input");
                    if !term {
                        return;
                    }
                } else {
                    break (rows, cols);
                }
            }
            Err(e) => {
                error!(error = %e, "Invalid input");
                if !term {
                    return;
                }
            }
        }
    };
    let mut total_vertices = rows * cols;
    debug!(?rows, ?cols, ?total_vertices);
    // 0: valid, unvisited | 1: hole | 2: visited
    let mut grid: Array2<usize> = Array::zeros((rows, cols));
    if term {
        println!(
            r"Enter the grid layout row by row.
  - Use '#' for a hole.
  - Use 'S' for the starting point.
Any other character will be interpreted as a valid path cell.
Multiple starting points after the first one will also be interpreted as a valid path cell.
Row string input with insufficient length will leave the remaining cells valid."
        );
    }
    {
        let mut iterator = stdin().lock().lines();
        for i in 0..rows {
            let row_str = iterator.next().unwrap().unwrap();
            for (j, c) in row_str.chars().enumerate() {
                if j == cols {
                    break;
                }
                match c {
                    'S' | 's' => {
                        // Start is a valid, unvisited cell
                        if !start_found {
                            (start_r, start_c) = (i, j);
                            start_found = true;
                        }
                    }
                    '#' => {
                        grid[[i, j]] = 1;
                        total_vertices -= 1;
                    } // Hole
                    _ => {} // Valid, unvisited
                }
            }
        }
    }
    debug!(?total_vertices, ?start_found, ?start_r, ?start_c);
    trace!(%grid);
    if !start_found {
        error!(?total_vertices, %grid, ?start_found, "Starting point 'S' not found in the grid");
        return;
    }
    let moves = args.get_moves();
    let mut only_lateral = true;
    let mut directions: Vec<(isize, isize)> = Vec::with_capacity(16);
    if moves.contains(&Moves::Lateral) {
        directions.extend_from_slice(&LATERAL);
    }
    if moves.contains(&Moves::Diagonal) {
        only_lateral = false;
        directions.extend_from_slice(&DIAGONAL);
    }
    if moves.contains(&Moves::Knight) {
        only_lateral = false;
        directions.extend_from_slice(&KNIGHT);
    }
    debug!(?moves, ?only_lateral, ?directions);
    if let Some(path) = find_hamiltonian_path(
        rows,
        cols,
        &mut grid,
        start_r,
        start_c,
        total_vertices,
        &directions,
    ) {
        assert_eq!(path.len(), total_vertices, "Invalid hamiltonian path!");
        info!("Hamiltonian path found:");
        for sol in &path {
            println!("{} {} {}", sol.r, sol.c, sol.dir);
        }
        if only_lateral {
            let dest = &path[total_vertices - 1];
            grid[[start_r, start_c]] = 2;
            grid[[dest.r, dest.c]] = 3;
            let n = total_vertices.saturating_sub(2);
            for i in path.iter().skip(1).take(n) {
                grid[[i.r, i.c]] = i.dir + 4;
            }
            info!("Path directions grid:");
            for i in 0..rows {
                for j in 0..cols {
                    print!("{}", DIRECTIONS[grid[[i, j]]]);
                }
                println!();
            }
            let mut prev = &path[0];
            for cur in path.iter().skip(1).take(n) {
                grid[[cur.r, cur.c]] = LOOKUP[prev.dir][cur.dir] + 4;
                prev = cur;
            }
            info!("Connected path grid:");
            for i in 0..rows {
                for j in 0..cols {
                    print!("{}", CONNECTED[grid[[i, j]]]);
                }
                println!();
            }
        }
    } else {
        warn!("No hamiltonian path exists from the starting vertex");
    }
}
