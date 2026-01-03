use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
use tracing::*;
use scan_rules::*;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::Uptime;
use tracing_subscriber::fmt::format::FmtSpan;
use ndarray::prelude::*;
use std::io::{BufRead, stdin, IsTerminal};
fn print_grid(grid: ArrayView2<bool>) {
    let [r, c] = grid.shape() else {panic!("Invalid dimensions")};
    for i in 0..*r {
        for j in 0..*c {
            print!("{}", if grid[[i, j]] {'#'} else {'.'});
        }
        println!();
    }
}
#[instrument(skip_all)]
fn solve(place_order: &mut Vec<usize>, lines_cleared: &mut Vec<usize>, pieces: &[Array2<bool>], state: &mut Array3<bool>) -> Option<usize> {
    let (n_pieces, rows, cols): (usize, usize, usize) = match state.shape() {
        [n, r, c] => (n-1, *r, *c),
        _ => panic!("Invalid dimensions"),
    };
    let grid_len = rows * cols;
    let (mut placed_pieces, mut total_lines_cleared, mut max_lines_cleared) = (0_usize, 0_usize, 0_usize);
    let mut working_place_order = place_order.clone();
    let mut working_lines_cleared = lines_cleared.clone();
    let mut working_state = state.clone();
    let mut row_filled: Vec<bool> = vec![false; rows];
    let mut col_filled: Vec<bool> = vec![false; cols];
    let mut solvable = false;
    debug!(?n_pieces, ?rows, ?cols, ?grid_len);
    trace!(?working_place_order, ?working_lines_cleared, ?working_state, ?pieces);
    let mut place = |placed_pieces: usize, idx: usize, pos_r: usize, pos_c: usize, working_state: &mut Array3<bool>| -> Option<usize> {
        let piece = &pieces[idx];
        let [piece_r, piece_c] = piece.shape() else {panic!("Invalid dimensions")};
        if pos_r + *piece_r > rows || pos_c + *piece_c > cols {
            return None;
        }
        for i in 0..*piece_r {
            for j in 0..*piece_c {
                if piece[[i, j]] && working_state[[placed_pieces, pos_r+i, pos_c+j]] {
                    return None;
                }
            }
        }
        let mut clear = 0_usize;
        let (mut wstate, src) = working_state.multi_slice_mut((s![placed_pieces+1, .., ..], s![placed_pieces, .., ..]));
        if let Some(d_raw) = wstate.as_slice_mut() &&
           let Some(s_raw) = src.as_slice() {
            d_raw.copy_from_slice(s_raw);
        } else {
            wstate.assign(&src);
        }
        for i in 0..*piece_r {
            for j in 0..*piece_c {
                if piece[[i, j]] {
                    wstate[[pos_r+i, pos_c+j]] = true;
                }
            }
        }
        'row: for i in 0..rows {
            let mut j = 0_usize;
            while wstate[[i, j]] {
                j += 1;
                if j == cols {
                    row_filled[i] = true;
                    clear += 1;
                    continue 'row;
                }
            }
            row_filled[i] = false;
        }
        'col: for i in 0..cols {
            let mut j = 0_usize;
            while wstate[[j, i]] {
                j += 1;
                if j == rows {
                    col_filled[i] = true;
                    clear += 1;
                    continue 'col;
                }
            }
            col_filled[i] = false;
        }
        for i in 0..rows {
            for j in 0..cols {
                if row_filled[i] || col_filled[j] {
                    wstate[[i, j]] = false;
                }
            }
        }
        Some(clear)
    };
    let mut blast = |working_place_order: &mut Vec<usize>| {
        let mut i = 2*placed_pieces+1;
        'outer: loop {
            {
                let piece_idx = working_place_order[i-1];
                let pos = &mut working_place_order[i];
                loop {
                    if *pos == grid_len {
                        *pos = 0;
                        if placed_pieces == 0 {
                            return;
                        }
                        placed_pieces -= 1;
                        i -= 2;
                        total_lines_cleared -= working_lines_cleared[placed_pieces];
                        working_place_order[i] += 1;
                        continue 'outer;
                    }
                    if let Some(clear) = place(placed_pieces, piece_idx, *pos/cols, *pos%cols, &mut working_state) {
                        working_lines_cleared[placed_pieces] = clear;
                        total_lines_cleared += clear;
                        break;
                    }
                    *pos += 1;
                }
            }
            if placed_pieces + 1 == n_pieces {
                if !solvable {solvable = true;}
                if total_lines_cleared > max_lines_cleared {
                    max_lines_cleared = total_lines_cleared;
                    *place_order = working_place_order.clone();
                    *lines_cleared = working_lines_cleared.clone();
                    *state = working_state.clone();
                }
                total_lines_cleared -= working_lines_cleared[placed_pieces];
                working_place_order[i] += 1;
                continue;
            }
            placed_pieces += 1;
            i += 2;
        }
    };
    blast(&mut working_place_order);
    // Heap's algorithm (non-recursive)
    let mut c: Vec<usize> = vec![0; n_pieces];
    let mut i = 1_usize;
    while i < n_pieces {
        debug!(?i, ?c, "Next iteration");
        if c[i] < i {
            let (l, r) = (if (i&1)==0 {0_usize} else {2*c[i]}, 2*i);
            debug!(?l, ?r, "Next permutation");
            working_place_order.swap(l, r);
            blast(&mut working_place_order);
            c[i] += 1;
            i = 1;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    if solvable {Some(max_lines_cleared)} else {None}
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
    let (rows, cols, n_pieces) = loop {
        if term {
            print!("Enter the grid dimensions (rows by columns) and the number of pieces: ");
        }
        let result = try_readln! {
            (let rows: usize, let cols: usize, let n_pieces: usize) => (rows, cols, n_pieces)
        };
        match result {
            Ok((rows, cols, n_pieces)) => {
                if rows == 0 || cols == 0 || n_pieces == 0 {
                    error!(?rows, ?cols, ?n_pieces, "Invalid input");
                    if !term {
                        return;
                    }
                } else {
                    break (rows, cols, n_pieces);
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
    debug!(?rows, ?cols, ?n_pieces);
    let mut place_order: Vec<usize> = vec![0; 2*n_pieces];
    let mut lines_cleared: Vec<usize> = vec![0; n_pieces];
    let mut pieces: Vec<Array2<bool>> = Vec::with_capacity(n_pieces);
    let mut state: Array3<bool> = Array::from_elem((n_pieces+1, rows, cols), false);
    if term {
        println!(r###"Enter the grid layout row by row.
  - Use '.' for an empty cell.
Any other character will be interpreted as a filled cell.
Row string input with insufficient length will leave the remaining cells empty."###);
    }
    {
        let mut iterator = stdin().lock().lines();
        for i in 0..rows {
            let row_str = iterator.next().unwrap().unwrap();
            for (j, c) in row_str.chars().enumerate() {
                if j == cols {
                    break;
                }
                state[[0, i, j]] = c != '.';
            }
        }
    }
    info!("Enter the dimensions and layout for each pieces");
    for i in 0..n_pieces {
        place_order[2*i] = i;
        let (r, c) = loop {
            if term {
                print!("Piece {}: ", i+1);
            }
            let result = try_readln! {
                (let r: usize, let c: usize) => (r, c)
            };
            match result {
                Ok((r, c)) => {
                    if r == 0 || c == 0 {
                        error!(?r, ?c, "Invalid input");
                        if !term {
                            return;
                        }
                    } else {
                        break (r, c);
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
        let mut piece: Array2<bool> = Array::from_elem((r, c), false);
        if term {println!("Layout:")}
        {
            let mut iterator = stdin().lock().lines();
            for j in 0..r {
                let row_str = iterator.next().unwrap().unwrap();
                for (k, ch) in row_str.chars().enumerate() {
                    if k == c {
                        break;
                    }
                    piece[[j, k]] = ch != '.';
                }
            }
        }
        pieces.push(piece);
    }
    let result = solve(&mut place_order, &mut lines_cleared, &pieces, &mut state);
    match result {
        Some(lines_cleared) => info!(?lines_cleared, "Solution found"),
        None => warn!("Unsolvable!"),
    };
    print_grid(state.slice(s![0, .., ..]));
    if result.is_some() {
        let mut j = 0_usize;
        for i in 1..=n_pieces {
            let pos = place_order[j+1];
            println!("Piece {}: {} {}", place_order[j]+1, pos/cols, pos%cols);
            print_grid(state.slice(s![i, .., ..]));
            j += 2;
        }
    }
}