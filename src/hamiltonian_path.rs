#[macro_use]
extern crate scan_rules;
fn main() {
    print!("Enter the grid dimensions (rows by columns): ");
    let (rows, cols) = loop {
        let result = try_readln! {
            (let rows: u32, let cols: u32) => (rows, cols)
        };
        match result {
            Ok((rows, cols)) => {
                if rows == 0 || cols == 0 {
                    print!("Invalid input, please try again: ");
                } else {
                    break (rows, cols);
                }
            },
            Err(_) => print!("Invalid input, please try again: "),
        }
    };
    println!("rows = {rows}, cols = {cols}");
}