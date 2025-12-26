#include <iostream>
#include <string>
#include <vector>
#include <string_view>
#include <array>
using namespace std;
constexpr string_view str { ".123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz<=>" };
constexpr size_t str_size = str.size();
constexpr auto arr = []() -> array<size_t, 128> {
    array<size_t, 128> a {0};
    for(size_t i = 0; i < str_size; ++i) {
        a.at(str[i]) = i;
    }
    return a;
}();
void print_grid(size_t& grid_size, vector<vector<size_t>>& grid) {
    for(size_t i = 0; i < grid_size; ++i) {
        for(size_t j = 0; j < grid_size; ++j) {
            cout << str[grid[i][j]];
        }
        cout << '\n';
    }
}
bool solve_sudoku(size_t& rows, size_t& cols, size_t& grid_size,
                  vector<vector<size_t>>& grid,
                  vector<vector<bool>>& row_contains,
                  vector<vector<bool>>& col_contains,
                  vector<vector<bool>>& subgrid_contains,
                  vector<pair<size_t, size_t>>& empty_cells) {
    bool backtrack = false;
    size_t placed_cells = 0;
    size_t n, S;
    size_t empty_cells_size = empty_cells.size();
    while(true) {
        if(placed_cells == empty_cells_size)
            return true;
        auto& [i, j] = empty_cells[placed_cells];
        S = (i / rows) * rows + j / cols;
        n = grid[i][j];
        if(backtrack) {
            grid[i][j] = 0;
            backtrack = false;
            row_contains[i][n-1] = false;
            col_contains[j][n-1] = false;
            subgrid_contains[S][n-1] = false;
        }
        for(; n < grid_size; ++n) {
            auto r = row_contains[i][n];
            auto c = col_contains[j][n];
            auto s = subgrid_contains[S][n];
            if(!(r || c || s)) {
                grid[i][j] = n + 1;
                ++placed_cells;
                r = true;
                c = true;
                s = true;
                break;
            }
        }
        if(n == grid_size) {
            if(!placed_cells)
                return false;
            --placed_cells;
            backtrack = true;
        }
    }
}
int main() {
    size_t rows, cols; // subgrid dimensions
    size_t grid_size; // square grid
    cout << "Enter the subgrid dimensions (rows by columns): ";
    cin >> rows >> cols;
    grid_size = rows * cols;
    if(!(grid_size && grid_size < str_size)) {
        cerr << "Error: Grid size can't be ";
        if(grid_size)
            cerr << "larger than ";
        cerr << (grid_size ? str_size - 1 : 0) << "\nNote: Grid size is determined by the number of cells in subgrid.\n";
        return 1;
    }
    vector<vector<size_t>> grid(grid_size, vector<size_t>(grid_size));
    vector<vector<bool>> row_contains(grid_size, vector<bool>(grid_size, false));
    vector<vector<bool>> col_contains(grid_size, vector<bool>(grid_size, false));
    vector<vector<bool>> subgrid_contains(grid_size, vector<bool>(grid_size, false));
    vector<pair<size_t, size_t>> empty_cells;
    empty_cells.reserve(grid_size);
    cout << "Enter the " << grid_size << " x " << grid_size << " sudoku grid row by row.\n"
            "  - Use 1-9 for a number 1-9 cell.\n"
            "  - Use A-Z for a number 10-35 cell.\n"
            "  - Use a-z for a number 36-61 cell.\n"
            "  - Use <=> for a number 62-64 cell.\n"
            "Any other character will be interpreted as an empty cell.\n";
    for(size_t i = 0, S; i < grid_size; ++i) {
        string row_str;
        cin >> row_str;
        if(row_str.size() < grid_size) {
            cerr << "Error: Insufficient input!\n";
            return 1;
        }
        for(size_t j = 0; j < grid_size; ++j) {
            size_t& k = grid[i][j];
            if((k = arr[row_str[j]]) && (k <= grid_size || (k = 0))) {
                S = (i / rows) * rows + j / cols;
                auto r = row_contains[i][k-1];
                auto c = col_contains[j][k-1];
                auto s = subgrid_contains[S][k-1];
                if(r || c || s) {
                    cerr << "Error: Invalid sudoku grid!\n";
                    return 1;
                }
                r = true;
                c = true;
                s = true;
            } else
                empty_cells.emplace_back(i, j);
        }
    }
    cout << "Solving...\n";
    if(solve_sudoku(rows, cols, grid_size, grid, row_contains, col_contains, subgrid_contains, empty_cells))
        cout << "Solution found:\n";
    else
        cout << "No solution exists for the sudoku grid!\n";
    print_grid(grid_size, grid);
}