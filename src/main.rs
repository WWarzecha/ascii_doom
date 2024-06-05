use std::collections::{HashMap, VecDeque};
use std::ffi::c_uint;
use std::io;
use std::time::Duration;
use crossterm::{
    event::{Event, KeyCode, KeyEvent, poll, read},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use crossterm::style::Stylize;

const PI: f32 = std::f32::consts::PI;
const PI_1_OVER_2: f32 = std::f32::consts::FRAC_PI_2;
const PI_3_OVER_2: f32 = 3.0 * std::f32::consts::FRAC_PI_2;
const DEG: f32 = 0.0174533;
const SCREEN_H: usize = 160;
const SCREEN_W: usize = 320;
const MAP_H: usize = 8;
const MAP_W: usize = 8;
// const MAP: [[u8; MAP_W]; MAP_H] = [
//     [1, 0, 0, 0, 0, 0, 0, 1],
//     [0, 0, 0, 0, 0, 0, 0, 0],
//     [0, 0, 0, 0, 0, 1, 0, 0],
//     [0, 0, 0, 0, 0, 0, 0, 0],
//     [0, 0, 0, 0, 0, 0, 0, 0],
//     [0, 0, 1, 0, 0, 0, 0, 0],
//     [0, 0, 0, 0, 0, 0, 0, 0],
//     [1, 0, 0, 0, 0, 0, 0, 1]
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
fn get_screen()->[[char; SCREEN_W];SCREEN_H]{
    [['.'; SCREEN_W];SCREEN_H]
}
fn reset_screen(screen: &mut [[char;SCREEN_W];SCREEN_H]){
    for line in screen{
        for pixel in line{
            *pixel = ' ';
        }
    }
}
fn print_screen(screen: &[[char; SCREEN_W]; SCREEN_H]){
    clearscreen::clear().unwrap();
    for line in screen.iter() { // Correctly iterate over each line
        for pixel in line.iter() { // Correctly iterate over each pixel in the line
            if *pixel == 'E' {
                print!("{}", pixel.to_string().red()); // Use `to_string().red()` to print in red
            } else {
                print!("{}", pixel);
            }
        }
        println!(); // Move to the next line after each line is printed
    }
}

fn draw_line(matrix: &mut [[char; SCREEN_W]; SCREEN_H], x0: usize, y0: usize, x1: usize, y1: usize, c: char) {
    let dx = (x1 as isize - x0 as isize).abs();
    let dy = (y1 as isize - y0 as isize).abs();
    let sx = if x0 < x1 { 1 } else { -1 }; // Use isize instead of usize
    let sy = if y0 < y1 { 1 } else { -1 }; // Use isize instead of usize
    let mut err = dx - dy;

    let mut x = x0;
    let mut y = y0;

    while x != x1 || y != y1 {
        if x < SCREEN_W && y < SCREEN_H {
            matrix[y][x] = c;
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

fn distance(x1: f32,y1: f32, x2: f32, y2: f32, ra: f32) -> f32{
    f32::sqrt(f32::powi(x1-x2,2) + f32::powi(y1-y2,2))
    // f32::cos(ra)*(x1-x2)-f32::sin(ra)*(y2-y1)
}


fn update_angle(change: f32, pa: &mut f32, pdx: &mut f32, pdy: &mut f32){
    *pa += change;
    *pdx = f32::cos(*pa)*5.0;
    *pdy = f32::sin(*pa)*5.0;
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


fn draw_ray(pa: f32, px: f32, py: f32, map: &[[u8;MAP_W];MAP_H], screen: &mut [[char; SCREEN_W]; SCREEN_H]){
    let(mut mx, mut my, mut dof) = (0usize, 0usize, 0usize);
    let(mut rx, mut ry, mut ra, mut xo, mut yo) = (0f32, 0f32, 0f32, 0f32, 0f32);
    let mut dis = f32::INFINITY;
    let mut dis_h = f32::INFINITY;
    let mut dis_v = f32::INFINITY;
    let mut hx = f32::INFINITY;; let mut hy = f32::INFINITY;;
    let mut vx = f32::INFINITY;; let mut vy = f32::INFINITY;;

    let change: f32 = 60.0*DEG/SCREEN_W as f32;     //FOV HAPPENING HERE
    ra = pa-30.0*DEG;
    if ra<0.0{ra+=2.0*PI};
    if ra>2.0*PI{ra-=2.0*PI};

    for r in 0..SCREEN_W{
        // HORIZONTAL LINES
        dof = 0;
        let tan: f32 = f32::tan(ra);
        let ctg: f32 = 1.0 / tan;
        if(ra>PI){
            ry = (py as usize) as f32-0.0001;
            rx = (ry-py) * ctg + px;
            yo = -1.0;
            xo = yo * ctg;
        }
        else if(ra > 0.0 && ra < PI){
            ry = (py as usize+1) as f32;
            rx = (ry-py) * ctg + px;
            yo = 1.0;
            xo = yo* ctg;
        }
        else{
            rx = px; ry = py; dof = MAP_W; dis_h = f32::INFINITY;;
        }

        while dof < MAP_W {
            mx = rx as usize; my = ry as usize;
            // if rx < 0.0 || rx > MAP_W as f32 || ry < 0.0 || ry > MAP_H as f32{
            //     rx = px + 100000.0*f32::cos(pa); ry = py + 100000.0*f32::sin(pa); dof = MAP_W;
            // }
            if mx < MAP_W && my < MAP_H && map[my][mx]>0u8{
                hx = rx;
                hy = ry;
                dis_h = distance(px, py, rx, ry, ra);
                dof = 8;
            }
            else{
                rx += xo; ry += yo; dof +=1;
            }
        }
        let scale_x = SCREEN_W as f32/ MAP_W as f32;
        let scale_y = SCREEN_H as f32 / MAP_H as f32;

        //horizontal rays on 2d map
        // draw_line(screen, (px*scale_x) as usize, (py*scale_y) as usize, (rx*scale_x) as usize, (ry*scale_y) as usize, '*');

        // VERTICAL LINES
        dof = 0;
        if(ra>PI_1_OVER_2 && ra<PI_3_OVER_2){
            // if f32::cos(ra) < -0.001{
            rx = (px as usize) as f32-0.001;
            ry = (rx-px) * tan + py;
            xo = -1.0;
            yo = xo * tan;
        }
        else if(ra < PI_1_OVER_2 || ra  > PI_3_OVER_2){
            // else if f32::cos(ra) > 0.001{
            rx = (px as usize) as f32 + 1.0;
            ry = (rx-px) * tan + py;
            xo = 1.0;
            yo = xo * tan;
        }
        else{
            rx = px; ry = py; dof = MAP_W; dis_v = f32::INFINITY;
        }
        while dof < MAP_W {

            mx = rx as usize; my = ry as usize;
            // print!("\nrx {rx}, ry {ry}, xo{xo}, yo{yo}, mx{mx}, my{my}");
            // if rx < 0.0 || rx > MAP_W as f32 || ry < 0.0 || ry > MAP_H as f32{
            //     rx = px + 100000.0*f32::cos(pa); ry = py + 100000.0*f32::sin(pa); dof = MAP_W;
            // }
            if(mx < MAP_W && my < MAP_H && map[my][mx]>0u8){
                vx = rx;
                vy = ry;
                dis_v = distance(px, py, rx, ry, ra);
                dof = MAP_W;
            }
            else {
                rx += xo;
                ry += yo;
                dof += 1;
            }
        }
        let c:char;
        if dis_v < dis_h {rx = vx; ry = vy; dis = dis_v; c='|'} else{rx = hx; ry = hy; dis = dis_h; c = '-'};




        //any ray
        // draw_line(screen, (px*scale_x) as usize, (py*scale_y) as usize, (rx*scale_x) as usize, (ry*scale_y) as usize, '*');


        //3D SCREEN
        let mut ca:f32 = pa-ra; if ca < 0.0 {ca += 2.0*PI} else if ca > 2.0*PI {ca -= 2.0*PI};
        dis *= f32::cos(ca);
        let mut line_h = (SCREEN_H as f32/ dis) as usize; if line_h > SCREEN_H {line_h = SCREEN_H};
        let tmp = SCREEN_H/2 - (line_h/2);
        draw_line(screen, r,tmp,r,tmp+line_h, c);


        // increment angle before another loop
        ra += change;
        if ra<0.0{ra+=2.0*PI};
        if ra>2.0*PI{ra-=2.0*PI};

    }

}


// fn render_enemy(px: f32, py: f32, pa: f32, ex: f32, ey: f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]) {
//     let distance = distance(px, py, ex, ey, 0.0);
//     let angular_difference = angle(px, py, pa, ex, ey);
//
//     // Check if the enemy is within the 30-degree FOV to either side
//     if angular_difference.abs() <= std::f32::consts::PI / 6.0 {
//         let max_size = 140.0;
//         let size_factor = distance.powi(2) / 5.0 + 1.0;
//         let size = ((max_size / size_factor).max(1.0)).min(max_size) as usize;
//
//         // Calculate horizontal screen position based on the angle
//         let half_fov = std::f32::consts::PI / 6.0;
//         let fov_scale = SCREEN_W as f32 / (2.0 * half_fov);
//         let screen_position_x = ((angular_difference + half_fov) * fov_scale).round() as isize - size as isize / 2;
//
//         let screen_position_y = (SCREEN_H as isize / 2) - (size as isize / 2);
//
//         for i in 0..size {
//             for j in 0..size {
//                 let draw_x = screen_position_x + j as isize;
//                 let draw_y = screen_position_y + i as isize;
//                 if draw_x >= 0 && draw_x < SCREEN_W as isize && draw_y >= 0 && draw_y < SCREEN_H as isize {
//                     screen[draw_y as usize][draw_x as usize] = 'E';
//                 }
//             }
//         }
//     }
// }

fn render_enemy(px: f32, py: f32, pa: f32, ex: f32, ey: f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]) {
    let distance = distance(px, py, ex, ey, 0.0);
    let angular_difference = angle(px, py, pa, ex, ey);

    // Check if the enemy is within the 30-degree FOV to either side
    if angular_difference.abs() <= std::f32::consts::PI / 6.0 {
        let max_size = 140.0;
        let size_factor = distance.powi(2) / 5.0 + 1.0;
        let size = ((max_size / size_factor).max(1.0)).min(max_size) as usize;

        // Calculate horizontal screen position based on the angle
        let half_fov = std::f32::consts::PI / 6.0;
        let fov_scale = SCREEN_W as f32 / (2.0 * half_fov);
        let screen_position_x = ((angular_difference + half_fov) * fov_scale).round() as isize - (size / 2) as isize;

        let screen_position_y = (SCREEN_H as isize / 2) - (size / 2) as isize;

        // Ensure the square is centered on the enemy's position
        for i in 0..size {
            for j in 0..size {
                let draw_x = screen_position_x + j as isize;
                let draw_y = screen_position_y + i as isize;
                if draw_x >= 0 && draw_x < SCREEN_W as isize && draw_y >= 0 && draw_y < SCREEN_H as isize {
                    screen[draw_y as usize][draw_x as usize] = 'E';
                }
            }
        }
    }
}



fn angle(px: f32, py: f32, pa: f32, ex: f32, ey: f32) -> f32 {
    let enemy_angle = f32::atan2(ey - py, ex - px);
    let pa_normalized = pa.rem_euclid(std::f32::consts::PI * 2.0);
    let enemy_angle_normalized = enemy_angle.rem_euclid(std::f32::consts::PI * 2.0);

    // Calculate the difference in a way that determines direction
    let mut diff = enemy_angle_normalized - pa_normalized;

    // Adjust the difference to be the minimal angle in the correct direction
    if diff > std::f32::consts::PI {
        diff -= 2.0 * std::f32::consts::PI;
    } else if diff < -std::f32::consts::PI {
        diff += 2.0 * std::f32::consts::PI;
    }

    diff
}


// Optional: Conversion function to degrees
fn to_degrees(radians: f32) -> f32 {
    radians * 180.0 / std::f32::consts::PI
}

fn can_see_enemy(px: f32, py: f32, ex: f32, ey: f32, map: &[[u8; MAP_W]; MAP_H]) -> bool {
    let mut x0 = px.round() as isize;
    let mut y0 = py.round() as isize;
    let x1 = ex.round() as isize;
    let y1 = ey.round() as isize;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    while x0 != x1 || y0 != y1 {
        if x0 >= 0 && x0 < MAP_W as isize && y0 >= 0 && y0 < MAP_H as isize {
            if map[y0 as usize][x0 as usize] > 0 {
                return false; // The ray has hit a wall
            }
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    true // The ray has not hit any wall
}


fn main()-> Result<(), io::Error>{

    let (mut px, mut py, mut pa, mut pdx, mut pdy) = (3f32, 3f32, 0f32, 0.1f32, 0.1f32);
    pdx = f32::cos(pa)*0.2;
    pdy = f32::sin(pa)*0.2;
    let mut screen = get_screen();

    let (mut e_px, mut e_py) = (6f32, 6f32); // Start enemy at position (6, 5.5)
    let graph = create_graph(&MAP);

    // render2d_map(&MAP, &mut screen);
    // render_player(px, py, &mut screen);
    //render_fullscreen2d_map(&MAP, &mut screen);
    // print_screen(&screen);

    //get input from now on
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
        // println!("Calculated path:");
        // for node in &path {
        //     println!("{:?}", node);
        // }

        // Move enemy towards the player using the path
        let new_position = move_enemy_towards_player(e_px, e_py, px, py, &path);
        e_px = new_position.0; // Update enemy's x-position
        e_py = new_position.1; // Update enemy's y-position


        // Your other loop logic here
        reset_screen(&mut screen);

        //draw_enemy_ray(pa, px, py, &mut screen, e_px, e_py);
        //render_enemy(px, py, pa, e_px, e_py, &mut screen);

        draw_ray(pa, px, py, &MAP, &mut screen);

        if can_see_enemy(px, py, e_px, e_py, &MAP) {
            render_enemy(px, py, pa, e_px, e_py, &mut screen);
        }

        //draw_ray(pa, px, py, &MAP, &mut screen);


        print_screen(&screen);
        print!("\npx: {},py: {},pa: {}, pdx:{}, pdy:{}\n", px, py, pa*180.0/PI, pdx, pdy);
        // For demonstration purposes, we'll just sleep for a short duration
        std::thread::sleep(Duration::from_millis(100));
    }

    disable_raw_mode()?;
    Ok(())
    // print_map(px, py, &map);
    // println!("Hello, world!");
}
