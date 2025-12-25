#include <array>
#include <iostream>
#include <optional>
#include <print>
#include <string>
#include <vector>
using namespace std;
// right, down, left, up
constexpr array<int, 4> dr {0, 1, 0, -1};
constexpr array<int, 4> dc {1, 0, -1, 0};
/*
Lookup table for use to determine which box-drawing character to print
2: ─
3: │
4: ┌
5: ┐
6: └
7: ┘
*/
constexpr array<array<size_t, 4>, 4> lookup {{
    {2, 5, 2, 7},
    {6, 3, 7, 3},
    {2, 4, 2, 6},
    {4, 3, 5, 3}
}};
struct state {
    size_t r, c;
    size_t dir; // 0: right | 1: down | 2: left | 3: up
};
optional<vector<state>> find_hamiltonian_path(size_t& rows, size_t& cols, vector<vector<int>> grid, size_t& startR, size_t& startC, size_t& total_vertices) {
    size_t nextR, nextC;
    size_t path_length {1};
    vector<state> path(total_vertices);
    path[0] = {startR, startC, 0};
    grid[startR][startC] = 2;
    while(true) {
        if(path_length == total_vertices) {
            return path;
        }
        state& cur_state = path[path_length - 1];
        for(; cur_state.dir < 4; ++cur_state.dir) {
            nextR = cur_state.r + dr[cur_state.dir];
            nextC = cur_state.c + dc[cur_state.dir];
            if(nextR < rows && nextC < cols && !grid[nextR][nextC]) {
                path[path_length++] = {nextR, nextC, 0};
                grid[nextR][nextC] = 2;
                break;
            }
        }
        if(cur_state.dir == 4) {
            grid[cur_state.r][cur_state.c] = 0;
            --path_length;
            if(!path_length)
                return nullopt;
            ++path[path_length - 1].dir;
        }
    }
}
int main() {
    size_t rows, cols;
    size_t startR, startC;
    size_t total_vertices{};
    bool start_found = false;
    cout << "Enter the grid dimensions (rows by columns): ";
    cin >> rows >> cols;
    vector<vector<int>> grid(rows, vector<int>(cols)); // 0: valid, unvisited | 1: hole | 2: visited
    cout << "Enter the grid layout row by row.\n"
            "  - Use '.' for a valid path cell.\n"
            "  - Use '#' for a hole.\n"
            "  - Use 'S' for the starting point.\n";
    for(size_t i = 0; i < rows; ++i) {
        string row_str;
        cin >> row_str;
        if(row_str.size() < cols) {
            cerr << "Error: Insufficient input!\n";
            return 1;
        }
        for(size_t j = 0; j < cols; ++j) {
            switch(row_str[j]) {
                case 'S': case 's':
                    if(start_found) {
                        cerr << "Error: Multiple start points ('S') found. Please specify only one.\n";
                        return 1;
                    }
                    startR = i;
                    startC = j;
                    grid[i][j] = 0; // Start is a valid, unvisited cell
                    ++total_vertices;
                    start_found = true;
                    break;
                case '.':
                    grid[i][j] = 0; // Valid, unvisited
                    ++total_vertices;
                    break;
                case '#':
                    grid[i][j] = 1; // Hole
                    break;
                default:
                    cerr << "Error: Invalid character '" << row_str[j] << "' in grid input.\n";
                    return 1;
            }
        }
    }
    if(!start_found) {
        cerr << "Error: Starting point 'S' not found in the grid.\n";
        return 1;
    }
    cout << "Finding...\n";
    auto path { find_hamiltonian_path(rows, cols, grid, startR, startC, total_vertices) };
    if(path.has_value()) {
        cout << "Hamiltonian path found:\n";
        for(auto sol : *path) {
            cout << sol.r << ' ' << sol.c << '\n';
        }
        auto end_it = path->end() - 1;
        for(auto it = path->begin(); it != end_it; ++it) {
            grid[it->r][it->c] = it->dir + 2;
        }
        cout << "Path directions grid:\n";
        for(size_t i = 0; i < rows; ++i) {
            for(size_t j = 0; j < cols; ++j) {
                switch(grid[i][j]) {
                    case 0:
                        cout << 'E';
                        break;
                    case 2:
                        print("→");
                        break;
                    case 3:
                        print("↓");
                        break;
                    case 4:
                        print("←");
                        break;
                    case 5:
                        print("↑");
                        break;
                    default:
                        cout << '#';
                }
            }
            cout << '\n';
        }
        auto itr = path->begin();
        grid[itr->r][itr->c] = lookup[itr->dir][itr->dir];
        for(auto it = itr + 1; it != end_it; ++it, ++itr) {
            grid[it->r][it->c] = lookup[itr->dir][it->dir];
        }
        cout << "Connected path grid:\n";
        for(size_t i = 0; i < rows; ++i) {
            for(size_t j = 0; j < cols; ++j) {
                switch(grid[i][j]) {
                    case 0:
                        cout << 'E';
                        break;
                    case 2:
                        print("─");
                        break;
                    case 3:
                        print("│");
                        break;
                    case 4:
                        print("┌");
                        break;
                    case 5:
                        print("┐");
                        break;
                    case 6:
                        print("└");
                        break;
                    case 7:
                        print("┘");
                        break;
                    default:
                        cout << '#';
                }
            }
            cout << '\n';
        }
    } else
        cout << "No Hamiltonian path exists from the starting vertex.\n";
}