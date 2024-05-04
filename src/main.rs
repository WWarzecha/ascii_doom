use std::ffi::c_uint;
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
fn print_screen(screen: &[[char;SCREEN_W];SCREEN_H]){
    clearscreen::clear().unwrap();
    for line in screen{
        for pixel in line{
            print!("{} ", pixel.green());
        }
        print!("\n")
    }
}

fn render_player(px: f32, py: f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]){
    screen[px as usize][py as usize] = '@';
}

fn render_fullscreen_player(px: f32, py: f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]){
    let scale_x = (SCREEN_W / MAP_W) as f32;
    let scale_y = (SCREEN_H / MAP_H) as f32;
    screen[(py * scale_y) as usize][(px * scale_x) as usize] = '@';
}

fn render2d_map(map: &[[u8;MAP_W];MAP_H], screen: &mut [[char; SCREEN_W]; SCREEN_H]) {

    for i in 0..MAP_H {
        for j in 0..MAP_W {
            if map[i][j] > 0 {
                screen[i][j] = '#';
            }
        }
    }
}

fn render_fullscreen2d_map(map: &[[u8;MAP_W];MAP_H], screen: &mut [[char; SCREEN_W]; SCREEN_H]) {
    let scale_x = SCREEN_W / MAP_W;
    let scale_y = SCREEN_H / MAP_H;
    for i in 0..MAP_H {
        for j in 0..MAP_W {
            if map[i][j] > 0 {
                for k in 0..scale_y {
                    for l in 0..scale_x {
                        screen[scale_y*i+k][scale_x*j+l] = '#';
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




fn draw_fullscreen_player_ray(px: f32, py: f32, pdx: f32, pdy:f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]){
    let scale_x = (SCREEN_W / MAP_W) as f32;
    let scale_y = (SCREEN_H / MAP_H) as f32;
    draw_line(screen,(px*scale_x) as usize, (py*scale_y) as usize, (px*scale_x+10.0*pdx*scale_x) as usize, (py*scale_y+10.0*pdy*scale_y) as usize);
}

fn update_angle(change: f32, pa: &mut f32, pdx: &mut f32, pdy: &mut f32){
    *pa += change;
    *pdx = f32::cos(*pa)*5.0;
    *pdy = f32::sin(*pa)*5.0;
}

fn draw_ray(pa: f32, px: f32, py: f32, map: &[[u8;MAP_W];MAP_H], screen: &mut [[char; SCREEN_W]; SCREEN_H]){
    let(mut mx, mut my, mut dof) = (0usize, 0usize, 0usize);
    let(mut rx, mut ry, mut ra, mut xo, mut yo) = (0f32, 0f32, 0f32, 0f32, 0f32);
    ra = pa;
    // let scale_x = SCREEN_W / MAP_W;
    // let scale_y = SCREEN_H / MAP_H;
    // let scaled_py = *py * scale_y as f32;
    // let scaled_px = *px * scale_x as f32;
    let scale_x = (SCREEN_W / MAP_W) as f32;
    let scale_y = (SCREEN_H / MAP_H) as f32;
    for r in 0..1{
        dof = 0;
        let ctg: f32 = -1.0/f32::tan(ra);
        if(ra>PI){
            // ry = ((py/64.0 as f32) as i32 * 64.0 as i32) as f32 -0.0001;
            // rx = (py-ry) * ctg + px;
            // yo = -(64.0 as f32);
            // xo = -yo* ctg;
            ry = (py as usize - (py as usize)%MAP_H) as f32 -0.0001;
            rx = (ry - py)* ctg + px;
            yo = -1.0;
            xo = yo* ctg;
        }
        else if(ra > 0.0 && ra < PI){
            // ry = ((py/64.0 as f32) as i32 * 64.0 as i32) as f32 + 64.0 as f32;
            // rx = (py-ry) * ctg + px;
            // yo = 64.0 as f32;
            // xo = -yo* ctg;
            ry = (py as usize - (py as usize)%MAP_H) as f32 + 1.0;
            rx = (py - ry)* ctg + px;
            yo = 1.0;
            xo = yo* ctg;
        }
        else if(ra == 0.0 || ra == PI){
            rx = px; ry = py; dof = MAP_W;
        }
        while dof < MAP_W {
            mx = (rx as f32) as usize; my = (ry as f32) as usize;
            if(0 < mx && mx < MAP_W && 0 < my && my < MAP_H && map[mx][my]>0){
                dof = 8;
            }
            else{
                rx += xo; ry += yo; dof +=1;
            }
        }

        draw_line(screen, (px*scale_x) as usize, (py*scale_y) as usize, (mx as f32 *scale_x) as usize, (my as f32 *scale_y) as usize);
    }
}



fn main()-> Result<(), io::Error>{

    let (mut px, mut py, mut pa, mut pdx, mut pdy) = (2f32, 2f32, 0f32, 0f32, 0f32);
    let mut screen = get_screen();

    // render2d_map(&MAP, &mut screen);
    // render_player(px, py, &mut screen);
    render_fullscreen2d_map(&MAP, &mut screen);
    // print_screen(&screen);

    //get input from now on
    enable_raw_mode()?;

    loop {
        if poll(Duration::from_millis(10))? {
            if let Event::Key(KeyEvent { code, .. }) = read()? {
                match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('w') => (px, py) = (px + pdx, py + pdy),
                    KeyCode::Char('s') =>(px, py) = (px - pdx, py - pdy),
                    KeyCode::Char('a') => {
                        pa += 0.1;
                        pa = if pa < 0.0 {2.0*PI} else if pa > 2.0*PI {0.0} else {pa};
                        pdx = f32::cos(pa)*0.1;
                        pdy = f32::sin(pa)*0.1;
                    },
                    KeyCode::Char('d') => {
                        pa -= 0.1;
                        pa = if pa < 0.0 {2.0*PI} else if pa > 2.0*PI {0.0} else {pa};
                        pdx = f32::cos(pa)*0.1;
                        pdy = f32::sin(pa)*0.1;
                    },

                    // Handle other key events as needed
                    _ => {}
                }
            }

        }
        // Your other loop logic here
        reset_screen(&mut screen);
        // render2d_map(&MAP, &mut screen);
        // render_player(px, py, &mut screen);
        render_fullscreen2d_map(&MAP, &mut screen);
        draw_fullscreen_player_ray(px, py, pdx, pdy, &mut screen);
        draw_ray(pa, px, py, &MAP, &mut screen);
        render_fullscreen_player(px, py, &mut screen);

        print_screen(&screen);

        // For demonstration purposes, we'll just sleep for a short duration
        std::thread::sleep(Duration::from_millis(50));
    }

    disable_raw_mode()?;
    Ok(())
    // print_map(px, py, &map);
    // println!("Hello, world!");
}
