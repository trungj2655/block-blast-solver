use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
use tracing::*;
use scan_rules::*;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::Uptime;
use tracing_subscriber::fmt::format::FmtSpan;
use ndarray::prelude::*;
use std::io::{BufRead as _, stdin, IsTerminal as _};
// right, down, left, up
static DR: [isize; 4] = [0, 1, 0, -1];
static DC: [isize; 4] = [1, 0, -1, 0];
/*
Lookup table for use to determine which box-drawing character to print
2: ─
3: │
4: ┌
5: ┐
6: └
7: ┘
*/
static LOOKUP: [[usize; 4]; 4] = [
    [2, 5, 2, 7],
    [6, 3, 7, 3],
    [2, 4, 2, 6],
    [4, 3, 5, 3],
];
static DIRECTIONS: [char; 7] = ['S', '#', '→', '↓', '←', '↑', 'E'];
static CONNECTED: [char; 9] = ['S', '#', '─', '│', '┌', '┐', '└', '┘', 'E'];
#[derive(Debug, Clone)]
struct State {
    r: usize, c: usize,
    dir: usize, // 0: right | 1: down | 2: left | 3: up
}
#[instrument(skip(grid))]
fn find_hamiltonian_path(rows: usize, cols: usize, grid: &mut Array2<usize>, start_r: usize, start_c: usize, total_vertices: usize) -> Option<Vec<State>> {
    let mut path_length = 1_usize;
    let mut path: Vec<State> = vec![State {r: 0, c: 0, dir: 0}; total_vertices];
    path[0] = State {r: start_r, c: start_c, dir: 0};
    grid[[start_r, start_c]] = 2;
    loop {
        if path_length == total_vertices {
            return Some(path);
        }
        let (left, right) = path.split_at_mut(path_length);
        let cur = &mut left[path_length - 1];
        let (cr, cc, cdir) = (cur.r, cur.c, &mut cur.dir);
        while *cdir < 4 {
            if let Some(next_r) = cr.checked_add_signed(DR[*cdir]) &&
            let Some(next_c) = cc.checked_add_signed(DC[*cdir]) &&
            next_r < rows && next_c < cols && grid[[next_r, next_c]] == 0 {
                right[0] = State {r: next_r, c: next_c, dir: 0};
                grid[[next_r, next_c]] = 2;
                path_length += 1;
                break;
            }
            *cdir += 1;
        }
        if *cdir == 4 {
            grid[[cr, cc]] = 0;
            path_length -= 1;
            if path_length == 0 {
                return None;
            }
            path[path_length - 1].dir += 1;
        }
    }
}
fn main() {
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
            },
            Err(e) => {
                error!(error = %e, "Invalid input");
                if !term {
                    return;
                }
            },
        }
    };
    let mut total_vertices = rows * cols;
    trace!(?rows, ?cols, ?total_vertices);
    let mut grid: Array2<usize> = Array::zeros((rows, cols)); // 0: valid, unvisited | 1: hole | 2: visited
    if term {
        println!(r"Enter the grid layout row by row.
  - Use '#' for a hole.
  - Use 'S' for the starting point.
Any other character will be interpreted as a valid path cell.
Multiple starting points after the first one will also be interpreted as a valid path cell.
Row string input with insufficient length will leave the remaining cells valid.");
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
                    },
                    '#' => {
                        grid[[i, j]] = 1;
                        total_vertices -= 1;
                    }, // Hole
                    _ => {}, // Valid, unvisited
                }
            }
        }
    }
    trace!(?total_vertices, %grid, ?start_found, ?start_r, ?start_c);
    if !start_found {
        error!(?total_vertices, %grid, ?start_found, "Starting point 'S' not found in the grid");
        return;
    }
    if let Some(path) = find_hamiltonian_path(rows, cols, &mut grid, start_r, start_c, total_vertices) {
        info!("Hamiltonian path found:");
        for sol in &path {
            println!("{} {} {}", sol.r, sol.c, sol.dir);
        }
        let dest = &path[total_vertices - 1];
        grid[[start_r, start_c]] = 0;
        grid[[dest.r, dest.c]] = 6;
        let n = path.len().saturating_sub(2);
        for i in path.iter().skip(1).take(n) {
            grid[[i.r, i.c]] = i.dir + 2;
        }
        info!("Path directions grid:");
        for i in 0..rows {
            for j in 0..cols {
                print!("{}", DIRECTIONS[grid[[i, j]]]);
            }
            println!();
        }
        grid[[dest.r, dest.c]] = 8;
        let iter = path.iter().skip(1).take(n);
        let mut prev = &path[0];
        for cur in iter {
            grid[[cur.r, cur.c]] = LOOKUP[prev.dir][cur.dir];
            prev = cur;
        }
        info!("Connected path grid:");
        for i in 0..rows {
            for j in 0..cols {
                print!("{}", CONNECTED[grid[[i, j]]]);
            }
            println!();
        }
    } else {
        info!("No Hamiltonian path exists from the starting vertex.");
    }
}