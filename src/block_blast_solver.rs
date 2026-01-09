use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
use clap::Parser;
use tracing::*;
use scan_rules::*;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::Uptime;
use tracing_subscriber::fmt::format::FmtSpan;
use ndarray::prelude::*;
use std::io::{BufRead as _, stdin, IsTerminal as _};
#[derive(Copy, Clone, Debug)]
struct Chunk(usize, usize);
#[derive(Copy, Clone, Debug)]
struct Available(usize, usize);
impl Available {
    const fn new(rows: usize, cols: usize, r: usize, c: usize) -> Self {
        let avail_r = rows - r + 1;
        let avail_c = cols - c + 1;
        let avail_len = avail_r * avail_c;
        Self(avail_c, avail_len)
    }
}
#[derive(Parser, Debug, Clone)]
#[command(version, about = "Block Blast! solver written in Rust", long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = false, help = "Also clears subgrids/\"chunks\" with sizes specified in the program")]
    wooden: bool,
    #[arg(short, long, default_value_t = false, help = "Use Steinhaus-Johnson-Trotter algorithm for permutations instead")]
    sjt: bool,
}
#[derive(Debug)]
struct Permutation {
    perm: Vec<usize>,
    pos: Vec<usize>,
    dir: Vec<bool>,
    n: usize,
    sjt: bool,
}
impl Permutation {
    fn new(n: usize, sjt: bool) -> Self {
        let perm: Vec<usize> = (0..n).collect();
        let mut pos: Vec<usize> = vec![0; n];
        let dir: Vec<bool> = vec![false; n];
        if sjt {
            pos = (0..n).collect();
        }
        Self { perm, pos, dir, n, sjt }
    }
    // Steinhaus-Johnson-Trotter algorithm (Even's speedup)
    // https://en.wikipedia.org/wiki/Steinhaus%E2%80%93Johnson%E2%80%93Trotter_algorithm#Even's_speedup
    fn permute_sjt(&mut self) -> bool {
        let mut k = self.n - 1;
        while k > 0 {
            let c_idx = self.pos[k];
            let target_idx = if self.dir[k] {
                let next = c_idx + 1;
                (next < self.n).then_some(next)
            } else {
                c_idx.checked_sub(1)
            };
            if let Some(t_idx) = target_idx && self.perm[t_idx] < k {
                let neighbor_val = self.perm[t_idx];
                self.perm.swap(c_idx, t_idx);
                self.pos[k] = t_idx;
                self.pos[neighbor_val] = c_idx;
                debug!(l = ?c_idx, r = ?t_idx, perm = ?self.perm, pos = ?self.pos, dir = ?self.dir);
                return true;
            }
            self.dir[k] = !self.dir[k];
            k -= 1;
        }
        false
    }
    // Heap's algorithm (non-recursive)
    // https://en.wikipedia.org/wiki/Heap%27s_algorithm#cite_ref-3
    fn permute_heap(&mut self) -> bool {
        let mut i = 1_usize;
        while i < self.n {
            if self.pos[i] < i {
                let l = if (i&1)==0 {0_usize} else {self.pos[i]};
                self.perm.swap(l, i);
                self.pos[i] += 1;
                debug!(?l, r = ?i, perm = ?self.perm, c = ?self.pos);
                return true;
            }
            self.pos[i] = 0;
            i += 1;
        }
        false
    }
    fn permute(&mut self) -> bool {
        if self.sjt {
            self.permute_sjt()
        } else {
            self.permute_heap()
        }
    }
}
fn print_grid(grid: &ArrayView2<bool>) {
    let (r, c) = grid.dim();
    for i in 0..r {
        for j in 0..c {
            print!("{}", if grid[[i, j]] {'#'} else {'.'});
        }
        println!();
    }
}
#[instrument(skip_all)]
fn solve(place_order: &mut Vec<usize>, piece_order: &mut Vec<usize>, lines_cleared: &mut Vec<usize>, pieces: &[Array2<bool>], piece_avail: &[Available], state: &mut Array3<bool>, sjt: bool, chunk: Option<Chunk>) -> Option<usize> {
    let (_, rows, cols) = state.dim();
    let n_pieces = pieces.len();
    let wooden = chunk.is_some();
    let (chunk_r, chunk_c) = match chunk {
        Some(Chunk(r, c)) => (r, c),
        None => (0, 0),
    };
    let (mut placed_pieces, mut total_lines_cleared, mut max_lines_cleared) = (0_usize, 0_usize, 0_usize);
    let mut working_place_order = place_order.clone();
    let mut working_lines_cleared = lines_cleared.clone();
    let mut working_state = state.clone();
    let mut piece_perm = Permutation::new(n_pieces, sjt);
    let mut row_filled: Vec<bool> = vec![false; rows];
    let mut col_filled: Vec<bool> = vec![false; cols];
    let mut solvable = false;
    debug!(?n_pieces, ?rows, ?cols, ?piece_avail);
    trace!(?working_place_order, ?working_lines_cleared, ?working_state, ?pieces);
    let mut place = |placed_pieces: usize, idx: usize, pos_r: usize, pos_c: usize, working_state: &mut Array3<bool>| -> Option<usize> {
        let piece = &pieces[idx];
        for ((i, j), k) in piece.indexed_iter() {
            if *k && working_state[[placed_pieces, pos_r+i, pos_c+j]] {
                return None;
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
        for ((i, j), k) in piece.indexed_iter() {
            if *k {
                wstate[[pos_r+i, pos_c+j]] = true;
            }
        }
        for (i, row) in wstate.axis_iter(Axis(0)).enumerate() {
            row_filled[i] = row.iter().all(|&x| x);
        }
        for (i, col) in wstate.axis_iter(Axis(1)).enumerate() {
            col_filled[i] = col.iter().all(|&x| x);
        }
        if wooden {
            for mut chunk in wstate.exact_chunks_mut((chunk_r, chunk_c)) {
                if chunk.iter().all(|&x| x) {
                    clear += 1;
                    chunk.fill(false);
                }
            }
        }
        for (i, mut row) in wstate.axis_iter_mut(Axis(0)).enumerate() {
            if row_filled[i] {
                clear += 1;
                row.fill(false);
            }
        }
        for (i, mut col) in wstate.axis_iter_mut(Axis(1)).enumerate() {
            if col_filled[i] {
                clear += 1;
                col.fill(false);
            }
        }
        Some(clear)
    };
    let mut blast = |working_piece_order: &Vec<usize>| {
        'outer: loop {
            {
                let piece_idx = working_piece_order[placed_pieces];
                let pos = &mut working_place_order[placed_pieces];
                let Available(avail_c, avail_len) = piece_avail[piece_idx];
                loop {
                    if *pos == avail_len {
                        *pos = 0;
                        if placed_pieces == 0 {
                            return;
                        }
                        placed_pieces -= 1;
                        total_lines_cleared -= working_lines_cleared[placed_pieces];
                        working_place_order[placed_pieces] += 1;
                        continue 'outer;
                    }
                    if let Some(clear) = place(placed_pieces, piece_idx, *pos / avail_c, *pos % avail_c, &mut working_state) {
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
                    place_order.clone_from(&working_place_order);
                    piece_order.clone_from(working_piece_order);
                    lines_cleared.clone_from(&working_lines_cleared);
                    state.clone_from(&working_state);
                }
                total_lines_cleared -= working_lines_cleared[placed_pieces];
                working_place_order[placed_pieces] += 1;
                continue;
            }
            placed_pieces += 1;
        }
    };
    debug!("{:?}", piece_perm);
    loop {
        blast(&piece_perm.perm);
        if !piece_perm.permute() {
            break;
        }
    }
    solvable.then_some(max_lines_cleared)
}
fn main() {
    let args = Args::parse();
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
    info!(terminal = ?term, ?args);
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
    let mut chunk = Chunk(0, 0);
    if args.wooden {
        chunk = loop {
            if term {
                print!("Enter the chunk dimensions (rows by columns): ");
            }
            let result = try_readln! {
                (let r: usize, let c: usize) => (r, c)
            };
            match result {
                Ok((r, c)) => {
                    let invalid = r == 0 || c == 0;
                    if invalid || rows % r != 0 || cols % c != 0 {
                        if invalid {
                            error!(?r, ?c, "Invalid input");
                        } else {
                            error!(?rows, ?cols, ?r, ?c, "Chunks do not distribute over the grid evenly");
                        }
                        if !term {
                            return;
                        }
                    } else {
                        break Chunk(r, c);
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
        debug!(?chunk);
    }
    let mut place_order: Vec<usize> = vec![0; n_pieces];
    let mut piece_order: Vec<usize> = vec![0; n_pieces];
    let mut lines_cleared: Vec<usize> = vec![0; n_pieces];
    let mut pieces: Vec<Array2<bool>> = Vec::with_capacity(n_pieces);
    let mut piece_avail: Vec<Available> = Vec::with_capacity(n_pieces);
    let mut state: Array3<bool> = Array::from_elem((n_pieces+1, rows, cols), false);
    if term {
        println!(r"Enter the grid layout row by row.
  - Use '.' for an empty cell.
Any other character will be interpreted as a filled cell.
Row string input with insufficient length will leave the remaining cells empty.");
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
        let (r, c) = loop {
            if term {
                print!("Piece {}: ", i+1);
            }
            let result = try_readln! {
                (let r: usize, let c: usize) => (r, c)
            };
            match result {
                Ok((r, c)) => {
                    let invalid = r == 0 || c == 0;
                    if invalid || rows < r || cols < c {
                        if invalid {
                            error!(?r, ?c, "Invalid input");
                        } else {
                            error!(?rows, ?cols, ?r, ?c, "Piece overflow!");
                        }
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
        piece_avail.push(Available::new(rows, cols, r, c));
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
    let result = solve(&mut place_order, &mut piece_order, &mut lines_cleared, &pieces, &piece_avail, &mut state, args.sjt, args.wooden.then_some(chunk));
    if let Some(clears) = result {
        info!(?clears, "Solution found");
    } else {
        warn!("Unsolvable!");
    }
    print_grid(&state.slice(s![0_usize, .., ..]));
    if result.is_some() {
        for (i, grid) in state.axis_iter(Axis(0)).skip(1).enumerate() {
            let avail_c = piece_avail[i].0;
            let pos = place_order[i];
            println!("Piece {}: {} {}", piece_order[i]+1, pos / avail_c, pos % avail_c);
            print_grid(&grid);
            match lines_cleared[i] {
                0 => {},
                1 => println!("(1 clear)"),
                c => println!("({c} clears)"),
            }
        }
    }
}
