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
    [1, 0, 1, 0, 0, 0, 0, 1],
    [1, 0, 1, 0, 0, 0, 0, 1],
    [1, 0, 1, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 0, 1],
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
    for line in (0..SCREEN_H){
        // for pixel in 0..SCREEN_W{
        //     print!("{} ", screen[line][pixel].green());
        // }
        let s2: String = screen[line].iter().collect();
        print!("{s2}\n")
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
        for j in (0..MAP_W).rev() {
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


fn draw_fullscreen_player_ray(px: f32, py: f32, pdx: f32, pdy:f32, screen: &mut [[char; SCREEN_W]; SCREEN_H]){
    let scale_x = (SCREEN_W / MAP_W) as f32;
    let scale_y = (SCREEN_H / MAP_H) as f32;
    draw_line(screen,(px*scale_x) as usize, (py*scale_y) as usize, ((px+5.0*pdx)*scale_x) as usize, ((py+5.0*pdy)*scale_y) as usize, '*');
}

fn update_angle(change: f32, pa: &mut f32, pdx: &mut f32, pdy: &mut f32){
    *pa += change;
    *pdx = f32::cos(*pa)*5.0;
    *pdy = f32::sin(*pa)*5.0;
}

fn draw_ray(pa: f32, px: f32, py: f32, map: &[[u8;MAP_W];MAP_H], screen: &mut [[char; SCREEN_W]; SCREEN_H]){
    let(mut mx, mut my, mut dof) = (0usize, 0usize, 0usize);
    let(mut rx, mut ry, mut ra, mut xo, mut yo) = (0f32, 0f32, 0f32, 0f32, 0f32);
    let mut dis = f32::INFINITY;
    let mut dis_h = f32::INFINITY;
    let mut dis_v = f32::INFINITY;
    let mut hx = f32::INFINITY;; let mut hy = f32::INFINITY;;
    let mut vx = f32::INFINITY;; let mut vy = f32::INFINITY;;

    let change: f32 = 60.0*DEG/SCREEN_W as f32;     //FOV HAPPENING HER
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



fn main()-> Result<(), io::Error>{

    let (mut px, mut py, mut pa, mut pdx, mut pdy) = (3.5f32, 5.299f32, 0f32, 0.1f32, 0.1f32);
    pdx = f32::cos(pa)*0.2;
    pdy = f32::sin(pa)*0.2;
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
                        pa = if pa <= 0.0 {2.0*PI-DEG} else if pa >= 2.0*PI {DEG} else {pa};
                        pdx = f32::cos(pa)*0.2;
                        pdy = f32::sin(pa)*0.2;
                    },
                    KeyCode::Char('d') => {
                        pa -= 0.1;
                        pa = if pa <= 0.0 {2.0*PI-DEG} else if pa >= 2.0*PI {DEG} else {pa};
                        pdx = f32::cos(pa)*0.2;
                        pdy = f32::sin(pa)*0.2;
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
        // render_fullscreen2d_map(&MAP, &mut screen);
        // draw_fullscreen_player_ray(px, py, pdx, pdy, &mut screen);
        draw_ray(pa, px, py, &MAP, &mut screen);
        // render_fullscreen_player(px, py, &mut screen);

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
