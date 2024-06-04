use std::collections::{HashMap, VecDeque};
use std::io;
use std::time::Duration;
use crossterm::{
    event::{Event, KeyCode, KeyEvent, poll, read},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use crossterm::style::Stylize;

const PI: f32 = std::f32::consts::PI;
const SCREEN_H: usize = 32;
const SCREEN_W: usize = 64;
const MAP_H: usize = 8;
const MAP_W: usize = 8;
// const MAP: [[u8; MAP_W]; MAP_H] = [
//     [1, 1, 1, 1, 1, 1, 1, 1],
//     [1, 0, 1, 1, 0, 0, 0, 1],
//     [1, 0, 1, 1, 0, 1, 0, 1],
//     [1, 0, 1, 1, 0, 1, 0, 1],
//     [1, 0, 1, 1, 0, 1, 0, 1],
//     [1, 0, 1, 1, 0, 1, 0, 1],
//     [1, 0, 0, 0, 0, 1, 0, 1],
//     [1, 1, 1, 1, 1, 1, 1, 1]
// ];

const MAP: [[u8; MAP_W]; MAP_H] = [
    [1, 1, 1, 1, 1, 1, 1, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 1, 0, 0, 1, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 1, 0, 0, 1, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1]
];

fn get_screen() -> [[char; SCREEN_W]; SCREEN_H] {
    [['.'; SCREEN_W]; SCREEN_H]
}

fn reset_screen(screen: &mut [[char; SCREEN_W]; SCREEN_H]) {
    for line in screen {
        for pixel in line {
            *pixel = ' ';
        }
    }
}

fn print_screen(screen: &[[char; SCREEN_W]; SCREEN_H]) {
    clearscreen::clear().unwrap();
    for line in screen {
        for pixel in line {
            print!("{} ", pixel.green());
        }
        print!("\n")
    }
}

fn render_player(px: f32, py: f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]) {
    screen[px as usize][py as usize] = '@';
}

fn render_enemy(px: f32, py: f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]) {
    screen[px as usize][py as usize] = 'E';
}

fn render_fullscreen_player(px: f32, py: f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]) {
    let scale_x = (SCREEN_W / MAP_W) as f32;
    let scale_y = (SCREEN_H / MAP_H) as f32;
    screen[(py * scale_y) as usize][(px * scale_x) as usize] = '@';
}

fn render_fullscreen_enemy(px: f32, py: f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]) {
    let scale_x = (SCREEN_W / MAP_W) as f32;
    let scale_y = (SCREEN_H / MAP_H) as f32;
    screen[(py * scale_y) as usize][(px * scale_x) as usize] = 'E';
}

fn render_fullscreen2d_map(map: &[[u8; MAP_W]; MAP_H], screen: &mut [[char; SCREEN_W]; SCREEN_H]) {
    let scale_x = SCREEN_W / MAP_W;
    let scale_y = SCREEN_H / MAP_H;
    for i in 0..MAP_H {
        for j in 0..MAP_W {
            if map[i][j] > 0 {
                for k in 0..scale_y {
                    for l in 0..scale_x {
                        screen[scale_y * i + k][scale_x * j + l] = '#';
                    }
                }
            }
        }
    }
}

fn draw_line(matrix: &mut [[char; SCREEN_W]; SCREEN_H], x0: usize, y0: usize, x1: usize, y1: usize) {
    let dx = (x1 as isize - x0 as isize).abs();
    let dy = (y1 as isize - y0 as isize).abs();
    let sx = if x0 < x1 { 1 } else { -1 }; // Use isize instead of usize
    let sy = if y0 < y1 { 1 } else { -1 }; // Use isize instead of usize
    let mut err = dx - dy;

    let mut x = x0;
    let mut y = y0;

    while x != x1 || y != y1 {
        if x < SCREEN_W && y < SCREEN_H {
            matrix[y][x] = '*';
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x = x.wrapping_add(sx as usize); // Use wrapping_add to handle potential overflow
        }
        if e2 < dx {
            err += dx;
            y = y.wrapping_add(sy as usize); // Use wrapping_add to handle potential overflow
        }
    }
}

fn draw_fullscreen_player_ray(px: f32, py: f32, pdx: f32, pdy: f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]) {
    let scale_x = (SCREEN_W / MAP_W) as f32;
    let scale_y = (SCREEN_H / MAP_H) as f32;
    draw_line(
        screen,
        (px * scale_x) as usize,
        (py * scale_y) as usize,
        (px * scale_x + 10.0 * pdx * scale_x) as usize,
        (py * scale_y + 10.0 * pdy * scale_y) as usize,
    );
}

fn update_angle(change: f32, pa: &mut f32, pdx: &mut f32, pdy: &mut f32) {
    *pa += change;
    *pdx = f32::cos(*pa) * 5.0;
    *pdy = f32::sin(*pa) * 5.0;
}

fn create_graph(map: &[[u8; MAP_W]; MAP_H]) -> HashMap<(usize, usize), Vec<(usize, usize)>> {
    let mut graph = HashMap::new();
    for i in 0..MAP_H {
        for j in 0..MAP_W {
            if map[i][j] == 0 {
                let mut neighbors = Vec::new();
                let directions = [(-1, 0), (1, 0), (0, -1), (0, 1)]; // Up, Down, Left, Right
                for &(di, dj) in &directions {
                    let ni = i as isize + di;
                    let nj = j as isize + dj;
                    if ni >= 0 && ni < MAP_H as isize && nj >= 0 && nj < MAP_W as isize {
                        if map[ni as usize][nj as usize] == 0 {
                            neighbors.push((ni as usize, nj as usize));
                        }
                    }
                }
                graph.insert((i, j), neighbors);
            }
        }
    }
    graph
}

fn bfs_path(
    graph: &HashMap<(usize, usize), Vec<(usize, usize)>>,
    start: (usize, usize),
    goal: (usize, usize)
) -> Vec<(usize, usize)> {
    let mut queue = VecDeque::new();
    let mut visited = HashMap::new();
    let mut prev = HashMap::new();

    queue.push_back(start);
    visited.insert(start, true);

    while let Some(node) = queue.pop_front() {
        if node == goal {
            break;
        }

        for &neighbor in graph.get(&node).unwrap_or(&Vec::new()) {
            if !visited.contains_key(&neighbor) {
                queue.push_back(neighbor);
                visited.insert(neighbor, true);
                prev.insert(neighbor, node);
            }
        }
    }

    // Reconstruct path from end to start using prev HashMap
    let mut path = Vec::new();
    let mut current = goal;
    while current != start {
        if let Some(&p) = prev.get(&current) {
            path.push(current);
            current = p;
        } else {
            // No path found
            return Vec::new();
        }
    }
    path.push(start); // optional: include the starting point
    path.reverse(); // path is constructed backwards, so reverse it

    path
}

fn is_within_bounds(x: f32, y: f32) -> bool {
    x >= 0.0 && x < MAP_W as f32 && y >= 0.0 && y < MAP_H as f32
}

fn is_free(x: usize, y: usize) -> bool {
    MAP[y][x] == 0
}

fn move_enemy_towards_player(
    mut ex: f32, mut ey: f32,
    px: f32, py: f32,
    path: &[(usize, usize)]
) -> (f32, f32) {
    // Convert float positions to map indices
    let ex_idx = ex as usize;
    let ey_idx = ey as usize;
    let px_idx = px as usize;
    let py_idx = py as usize;

    // Check if enemy is in the same vertex as the player
    if ex_idx == px_idx && ey_idx == py_idx {
        let dx = px - ex;
        let dy = py - ey;
        let distance = (dx * dx + dy * dy).sqrt();
        let step_size_x = dx / distance * 0.05;
        let step_size_y = dy / distance * 0.05;

        // Make a direct movement towards the player's precise position
        let new_ex = ex + step_size_x;
        let new_ey = ey + step_size_y;

        if is_within_bounds(new_ex, new_ey) && is_free(new_ex as usize, new_ey as usize) {
            ex = new_ex;
            ey = new_ey;
        }
    } else {
        // Follow the path provided by BFS algorithm
        if let Some(&(next_x, next_y)) = path.get(1) { // Get the next step from the path
            // Calculate the center of the next cell to move towards
            let center_x = next_x as f32 + 0.5;
            let center_y = next_y as f32 + 0.5;
            let dx = center_x - ex;
            let dy = center_y - ey;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < 0.1 {
                // Reached the center of the cell
                ex = center_x;
                ey = center_y;
            } else {
                let step_size_x = dx / distance * 0.03;
                let step_size_y = dy / distance * 0.03;

                let new_ex = ex + step_size_x;
                let new_ey = ey + step_size_y;

                if is_within_bounds(new_ex, new_ey) && is_free(new_ex as usize, new_ey as usize) {
                    ex = new_ex;
                    ey = new_ey;
                }
            }
        }
    }

    (ex, ey)
}



fn main() -> Result<(), io::Error> {
    let (mut px, mut py, mut pa, mut pdx, mut pdy) = (3f32, 3f32, 0f32, 0f32, 0f32);
    let (mut e_px, mut e_py) = (6f32, 5.5f32); // Start enemy at position (6, 5.5)
    let mut screen = get_screen();
    let graph = create_graph(&MAP);

    enable_raw_mode()?;

    loop {
        if poll(Duration::from_millis(10))? {
            if let Event::Key(KeyEvent { code, .. }) = read()? {
                match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('w') => {
                        let next_x = px + pdx;
                        let next_y = py + pdy;
                        if is_within_bounds(next_x, next_y) && is_free(next_x as usize, next_y as usize) {
                            px = next_x;
                            py = next_y;
                        }
                    },
                    KeyCode::Char('s') => {
                        let next_x = px - pdx;
                        let next_y = py - pdy;
                        if is_within_bounds(next_x, next_y) && is_free(next_x as usize, next_y as usize) {
                            px = next_x;
                            py = next_y;
                        }
                    },
                    KeyCode::Char('d') => {
                        pa += 0.1;
                        if pa > 2.0 * PI {
                            pa -= 2.0 * PI;
                        }
                        pdx = f32::cos(pa) * 0.1;
                        pdy = f32::sin(pa) * 0.1;
                    },
                    KeyCode::Char('a') => {
                        pa -= 0.1;
                        if pa < 0.0 {
                            pa += 2.0 * PI;
                        }
                        pdx = f32::cos(pa) * 0.1;
                        pdy = f32::sin(pa) * 0.1;
                    },
                    _ => {}
                }
            }
        }

        let path = bfs_path(&graph, (e_px as usize, e_py as usize), (px as usize, py as usize));

        // Debug: Print the calculated path
        println!("Calculated path:");
        for node in &path {
            println!("{:?}", node);
        }

        // Move enemy towards the player using the path
        let new_position = move_enemy_towards_player(e_px, e_py, px, py, &path);
        e_px = new_position.0; // Update enemy's x-position
        e_py = new_position.1; // Update enemy's y-position

        // Debug: Print enemy's position
        println!("Enemy position: ({:.2}, {:.2})", e_px, e_py);

        // Render game state
        reset_screen(&mut screen);
        render_fullscreen2d_map(&MAP, &mut screen);
        draw_fullscreen_player_ray(px, py, pdx, pdy, &mut screen);
        render_fullscreen_player(px, py, &mut screen);
        render_fullscreen_enemy(e_px, e_py, &mut screen); // Ensure enemy is rendered correctly
        print_screen(&screen);

        std::thread::sleep(Duration::from_millis(50));
    }

    disable_raw_mode()?;
    Ok(())
}

