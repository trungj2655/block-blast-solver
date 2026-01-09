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
use core::char::from_u32;
#[expect(clippy::as_conversions, reason = "should be safe to convert char to usize directly")]
fn print_grid(grid: &Array2<usize>) {
    let (r, c) = grid.dim();
    for i in 0..r {
        for j in 0..c {
            let k = match grid[[i, j]] {
                n @ 1..=9 => n + '0' as usize,
                n @ 10..=35 => n + 'A' as usize - 10,
                n @ 36..=61 => n + 'a' as usize - 36,
                n @ 62..=64 => n + '<' as usize - 62,
                _ => '.' as usize,
            };
            print!("{}", from_u32(k as u32).unwrap());
        }
        println!();
    }
}
#[instrument(skip_all)]
#[expect(nonstandard_style, reason = "temporary variable names")]
fn solve_sudoku(rows: usize, cols: usize, grid: &mut Array2<usize>, row_contains: &mut Array2<bool>, col_contains: &mut Array2<bool>, subgrid_contains: &mut Array2<bool>, empty_cells_len: usize) -> bool {
    let grid_size = rows * cols;
    let mut placed_cells = 0_usize;
    let mut backtrack = false;
    let mut empty_cells: Vec<(usize, usize)> = Vec::with_capacity(empty_cells_len);
    for ((i, j), k) in grid.indexed_iter() {
        if *k == 0 {
            empty_cells.push((i, j));
        }
    }
    'cell: loop {
        if placed_cells == empty_cells_len {
            return true;
        }
        let (i, j) = empty_cells[placed_cells];
        let S = (i / rows) * rows + j / cols;
        let mut n = grid[[i, j]];
        if backtrack {
            backtrack = false;
            grid[[i, j]] = 0;
            row_contains[[i, n-1]] = false;
            col_contains[[j, n-1]] = false;
            subgrid_contains[[S, n-1]] = false;
        }
        while n < grid_size {
            let r = &mut row_contains[[i, n]];
            let c = &mut col_contains[[j, n]];
            let s = &mut subgrid_contains[[S, n]];
            if !(*r || *c || *s) {
                grid[[i, j]] = n + 1;
                placed_cells += 1;
                *r = true;
                *c = true;
                *s = true;
                continue 'cell;
            }
            n += 1;
        }
        if placed_cells == 0 {
            return false;
        }
        placed_cells -= 1;
        backtrack = true;
    }
}
fn main() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_line_number(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_timer(Uptime::default())
        .init();
    let _main_span = info_span!("main").entered();
    let term = stdin().is_terminal();
    info!(terminal = ?term);
    let (rows, cols, grid_size) = loop {
        if term {
            print!("Enter the subgrid dimensions (rows by columns): ");
        }
        let result = try_readln! {
            (let rows: usize, let cols: usize) => (rows, cols)
        };
        match result {
            Ok((rows, cols)) => {
                let grid_size = rows * cols;
                if grid_size == 0 {
                    error!(?rows, ?cols, "Invalid input");
                } else if grid_size > 64 {
                    error!(?rows, ?cols, ?grid_size, "Grid size can't be larger than 64");
                } else {
                    break (rows, cols, grid_size);
                }
                if !term {return;}
            },
            Err(e) => {
                error!(error = %e, "Invalid input");
                if !term {return;}
            },
        }
    };
    let mut empty_cells_len = grid_size * grid_size;
    debug!(?rows, ?cols, ?grid_size, ?empty_cells_len);
    let mut grid: Array2<usize> = Array::zeros((grid_size, grid_size));
    let mut row_contains: Array2<bool> = Array::from_elem((grid_size, grid_size), false);
    let mut col_contains: Array2<bool> = Array::from_elem((grid_size, grid_size), false);
    let mut subgrid_contains: Array2<bool> = Array::from_elem((grid_size, grid_size), false);
    if term {
        println!(r"Enter the {grid_size}x{grid_size} sudoku grid row by row.
  - Use 1-9 for a number 1-9 cell.
  - Use A-Z for a number 10-35 cell.
  - Use a-z for a number 36-61 cell.
  - Use <=> for a number 62-64 cell.
Any other character will be interpreted as an empty cell.
Row string input with insufficient length will leave the remaining cells empty.");
    }
    {
        let mut iterator = stdin().lock().lines();
        for i in 0..grid_size {
            let row_str = iterator.next().unwrap().unwrap();
            for (j, c) in row_str.chars().enumerate() {
                if j == grid_size {
                    break;
                }
                #[expect(clippy::as_conversions, reason = "should be safe to convert char to usize directly")]
                let k: usize = match c {
                    '1'..='9' => c as usize - '0' as usize,
                    'A'..='Z' => c as usize - 'A' as usize + 10,
                    'a'..='z' => c as usize - 'a' as usize + 36,
                    '<'..='>' => c as usize - '<' as usize + 62,
                    _ => 0,
                };
                #[expect(nonstandard_style, reason = "temporary variable names")]
                if k != 0 && k <= grid_size {
                    let S = (i / rows) * rows + j / cols;
                    let r = &mut row_contains[[i, k-1]];
                    let C = &mut col_contains[[j, k-1]];
                    let s = &mut subgrid_contains[[S, k-1]];
                    if *r || *C || *s {
                        error!(ch = ?c, n = ?k, r = ?i, c = ?j, "Invalid sudoku grid!");
                        return;
                    }
                    empty_cells_len -= 1;
                    grid[[i, j]] = k;
                    *r = true;
                    *C = true;
                    *s = true;
                }
            }
        }
    }
    debug!(?empty_cells_len);
    let solvable = solve_sudoku(rows, cols, &mut grid, &mut row_contains, &mut col_contains, &mut subgrid_contains, empty_cells_len);
    if solvable {
        info!("Solution found:");
    } else {
        warn!("Unsolvable!");
    }
    print_grid(&grid);
}
