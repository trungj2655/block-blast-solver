#include <iostream>
#include <iterator>
#include <string>
#include <vector>
#include <algorithm>
using namespace std;
void print_grid(size_t& r, size_t& c, vector<bool>& grid) {
    for(size_t i = 0; i < r; ++i) {
        for(size_t j = 0; j < c; ++j) {
            cout << (grid[i*c+j] ? '#' : '.');
        }
        cout << '\n';
    }
}
bool solve(size_t& rows, size_t& cols, size_t& n_pieces, size_t& grid_size, size_t& max_lines_cleared,
           vector<size_t>& place_order, vector<size_t>& pieces_dims, vector<size_t>& lines_cleared,
           vector<vector<bool>>& pieces, vector<vector<bool>>& state) {
    size_t placed_pieces{}, pos_r, pos_c, total_lines_cleared{};
    vector<size_t> working_place_order(place_order);
    vector<size_t> working_lines_cleared(lines_cleared);
    vector<vector<bool>> wstate(state);
    auto piece_idx = working_place_order.begin();
    auto clear = working_lines_cleared.begin();
    auto pos = piece_idx + 1;
    auto cur_state = wstate.begin();
    vector<vector<bool>>::iterator working_state, piece;
    vector<size_t>::iterator piece_r, piece_c;
    vector<bool> row_filled(rows);
    vector<bool> col_filled(cols);
    bool backtrack = true, solvable = false;
    auto place = [&]() -> bool {
        if(pos_r + *piece_r > rows || pos_c + *piece_c > cols)
            return false;
        for(size_t i = 0; i < *piece_r; ++i) {
            for(size_t j = 0; j < *piece_c; ++j) {
                if((*piece)[i**piece_c+j] && (*cur_state)[(pos_r+i)*cols+(pos_c+j)])
                    return false;
            }
        }
        working_state = cur_state + 1;
        copy(cur_state->begin(), cur_state->end(), working_state->begin());
        for(size_t i = 0; i < *piece_r; ++i) {
            for(size_t j = 0; j < *piece_c; ++j) {
                if((*piece)[i**piece_c+j])
                    (*working_state)[(pos_r+i)*cols+(pos_c+j)] = true;
            }
        }
        for(size_t i = 0, j; i < rows; ++i) {
            for(j = 0; j < cols && (*working_state)[i*cols+j]; ++j);
            row_filled[i] = (j == cols);
            if(j == cols) ++*clear;
        }
        for(size_t i = 0, j; i < cols; ++i) {
            for(j = 0; j < rows && (*working_state)[j*cols+i]; ++j);
            col_filled[i] = (j == rows);
            if(j == rows) ++*clear;
        }
        total_lines_cleared += *clear;
        for(size_t i = 0; i < rows; ++i) {
            for(size_t j = 0; j < cols; ++j) {
                if(row_filled[i] || col_filled[j]) {
                    (*working_state)[i*cols+j] = false;
                }
            }
        }
        return true;
    };
    auto blast = [&]() -> void {
        piece = pieces.begin() + *piece_idx;
        piece_r = pieces_dims.begin() + 2**piece_idx;
        piece_c = piece_r + 1;
        while(true) {
            for(; *pos != grid_size; ++*pos) {
                pos_r = *pos / cols;
                pos_c = *pos % cols;
                if(place()) {
                    backtrack = false;
                    break;
                }
            }
            if(backtrack) {
                *pos = 0;
                if(!placed_pieces)
                    return;
                --placed_pieces;
                piece_idx -= 2;
                pos -= 2;
                --cur_state;
                --clear;
                total_lines_cleared -= *clear;
                *clear = 0;
                ++*pos;
            } else {
                backtrack = true;
                if(++placed_pieces == n_pieces) {
                    if(!solvable) solvable = true;
                    if(total_lines_cleared > max_lines_cleared) {
                        max_lines_cleared = total_lines_cleared;
                        copy(working_place_order.begin(), working_place_order.end(), place_order.begin());
                        copy(working_lines_cleared.begin(), working_lines_cleared.end(), lines_cleared.begin());
                        copy(wstate.begin(), wstate.end(), state.begin());
                    }
                    --placed_pieces;
                    total_lines_cleared -= *clear;
                    *clear = 0;
                    ++*pos;
                    continue;
                }
                piece_idx += 2;
                pos += 2;
                ++cur_state;
                ++clear;
            }
            piece = pieces.begin() + *piece_idx;
            piece_r = pieces_dims.begin() + 2**piece_idx;
            piece_c = piece_r + 1;
        }
    };
    blast();
    // Heap's algorithm (non-recursive)
    vector<size_t> c(n_pieces, 0);
    for(size_t i = 1; i < n_pieces;) {
        if(c[i] < i) {
            swap(working_place_order[(i&1) ? 2*c[i] : 0], working_place_order[2*i]);
            blast();
            ++c[i];
            i = 1;
        } else
            c[i++] = 0;
    }
    return solvable;
}
int main() {
    size_t rows, cols, n_pieces, grid_size, max_lines_cleared{};
    cout << "Enter the grid dimensions (rows by columns) and the number of pieces: ";
    while(true) {
        cin >> rows >> cols >> n_pieces;
        grid_size = rows * cols;
        if(grid_size && n_pieces)
            break;
        cout << "Invalid input, please try again: ";
    }
    vector<size_t> place_order(2 * n_pieces, 0);
    vector<size_t> pieces_dims(2 * n_pieces);
    vector<size_t> lines_cleared(n_pieces, 0);
    vector<vector<bool>> pieces;
    vector<vector<bool>> state(n_pieces + 1, vector<bool>(grid_size, false));
    auto initial_state = state.begin();
    cout << "Enter the grid layout row by row.\n"
            "  - Use '.' for an empty cell.\n"
            "Any other character will be interpreted as a filled cell.\n"
            "Row string input with insufficient length will leave the remaining cells empty.\n";
    for(size_t i = 0, minn; i < rows; ++i) {
        string row_str;
        cin >> row_str;
        minn = min(row_str.size(), cols);
        for(size_t j = 0; j < minn; ++j) {
            (*initial_state)[i*cols+j] = (row_str[j] != '.');
        }
    }
    cout << "Enter the dimensions and layout for each pieces.\n";
    for(size_t i = 0, minn, r, c; i < n_pieces; ++i) {
        place_order[2*i] = i;
        cout << "Piece " << (i + 1) << ": ";
        while(true) {
            cin >> r >> c;
            if(r && c)
                break;
            cout << "Invalid dimensions, please try again: ";
        }
        vector<bool> piece(r * c, false);
        pieces_dims[2*i] = r;
        pieces_dims[2*i+1] = c;
        cout << "Layout:\n";
        for(size_t j = 0; j < r; ++j) {
            string row_str;
            cin >> row_str;
            minn = min(row_str.size(), c);
            for(size_t k = 0; k < minn; ++k) {
                piece[j*c+k] = (row_str[k] != '.');
            }
        }
        pieces.push_back(piece);
    }
    cout << "Solving...\n";
    bool solvable = solve(rows, cols, n_pieces, grid_size, max_lines_cleared, place_order, pieces_dims, lines_cleared, pieces, state);
    if(solvable) {
        cout << "Solution found (" << max_lines_cleared << " lines cleared):\nInitial grid:\n";
    } else {
        cout << "Unsolvable!\nGrid:\n";
    }
    print_grid(rows, cols, *initial_state);
    if(solvable) {
        auto piece_idx = place_order.begin();
        auto pos = piece_idx + 1;
        auto cur_state = state.begin() + 1;
        auto state_end = state.end();
        auto clear = lines_cleared.begin();
        for(; cur_state != state_end; ++cur_state) {
            cout << "Piece " << (*piece_idx + 1) << ": " << (*pos / cols) << ' ' << (*pos % cols) << '\n';
            print_grid(rows, cols, *cur_state);
            if(*clear)
                cout << '(' << *clear << " lines cleared)\n";
            piece_idx += 2;
            pos += 2;
            ++clear;
        }
    }
}