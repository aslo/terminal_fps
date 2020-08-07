use std::io::{stdin, stdout, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::terminal_size;

// Rendering constants
const FOV: f64 = 3.14159 / 4.0;
const MAX_RENDER_DIST: f64 = 16.0;

struct Screen {
    width: usize,
    height: usize,
    screen: Vec<char>,
}

impl Screen {
    fn new(width: usize, height: usize) -> Screen {
        return Screen {
            width: width,
            height: height,
            screen: vec![' '; width * height],
        };
    }

    // Writes s into screen vector.
    fn draw(&mut self, x: usize, y: usize, s: &str) {
        let offset = y * self.width + x;
        for (i, c) in s.chars().enumerate() {
            if offset + i >= self.screen.len() {
                break;
            }
            self.screen[offset + i] = c;
        }
    }

    fn flush<W: Write>(&self, out: &mut W) {
        let screen_str: String = self.screen.iter().collect();
        write!(out, "{}{}", termion::cursor::Goto(1, 1), screen_str).unwrap();
        out.flush().unwrap();
    }
}

fn wall_shade(dist: f64) -> char {
    if dist <= MAX_RENDER_DIST / 4.0 {
        return std::char::from_u32(0x2588).unwrap();
    } else if dist < MAX_RENDER_DIST / 3.0 {
        return std::char::from_u32(0x2593).unwrap();
    } else if dist < MAX_RENDER_DIST / 2.0 {
        return std::char::from_u32(0x2592).unwrap();
    } else if dist < MAX_RENDER_DIST {
        return std::char::from_u32(0x2591).unwrap();
    }
    return ' ';
}

fn main() {
    let map_str = "################
#.....#........#
#.....#........#
#.....#........#
#.....#........#
#.....#........#
#.....#........#
#....##...######
#....#.........#
#..............#
##########.....#
#..............#
#..............#
#..............#
#..............#
################";

    // Parse map into 2d vector
    let mut map: Vec<Vec<char>> = Vec::new();
    map.push(Vec::new());
    let mut line = 0;
    for c in map_str.chars() {
        if c != '\n' {
            map[line].push(c);
        } else {
            map.push(Vec::new());
            line += 1;
        }
    }

    // Spawn thread to listen for user input events
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for c in stdin().events() {
            let evt = c.unwrap();
            tx.send(evt).unwrap();
        }
    });

    // Get terminal size
    let mut stdout = stdout().into_raw_mode().unwrap();
    let (window_width, window_height) = terminal_size().unwrap();

    let mut t_prev = Instant::now();

    let mut player_x = 1.0;
    let mut player_y = 1.0;
    let mut player_angle = 0.0;

    // Clear terminal
    write!(stdout, "{}", termion::clear::All).unwrap();
    stdout.flush().unwrap();

    loop {
        // Create screen buffer
        let mut screen = Screen::new(window_width as usize, window_height as usize);

        // Compute elapsed time since last frame
        let t_curr = Instant::now();
        let t_elapsed = t_curr.duration_since(t_prev);
        t_prev = t_curr;

        //
        // Handle input
        //
        const PLAYER_V: f64 = 10.0;
        const PLAYER_ROT_V: f64 = 5.0;

        //  Check for input on user input channel and update character position
        for event in rx.try_recv() {
            match event {
                Event::Key(Key::Char('q')) => {
                    // Clear screen and exit.
                    write!(stdout, "{}", termion::clear::All).unwrap();
                    return;
                }
                Event::Key(Key::Left) => {
                    player_angle -= PLAYER_ROT_V * t_elapsed.as_secs_f64() as f64
                }
                Event::Key(Key::Right) => {
                    player_angle += PLAYER_ROT_V * t_elapsed.as_secs_f64() as f64
                }
                Event::Key(Key::Up) => {
                    player_x += player_angle.sin() * PLAYER_V * t_elapsed.as_secs_f64();
                    player_y += player_angle.cos() * PLAYER_V * t_elapsed.as_secs_f64();
                    // Collision detection
                    if map[player_x as usize][player_y as usize] == '#' {
                        player_x -= player_angle.sin() * PLAYER_V * t_elapsed.as_secs_f64();
                        player_y -= player_angle.cos() * PLAYER_V * t_elapsed.as_secs_f64();
                    }
                }
                Event::Key(Key::Down) => {
                    player_x -= player_angle.sin() * PLAYER_V * t_elapsed.as_secs_f64();
                    player_y -= player_angle.cos() * PLAYER_V * t_elapsed.as_secs_f64();
                    // Collision detection
                    if map[player_x as usize][player_y as usize] == '#' {
                        player_x += player_angle.sin() * PLAYER_V * t_elapsed.as_secs_f64();
                        player_y += player_angle.cos() * PLAYER_V * t_elapsed.as_secs_f64();
                    }
                }
                _ => {
                    screen.draw(0, 75, &format!("got unexpected event: {:?}", event));
                }
            }
        }

        //
        // Handle drawing
        //

        // Ray casting
        for x in 0..screen.width {
            // For each column, calculate the projected ray angle into world space
            let ray_angle: f64 =
                (player_angle - FOV / 2.0) + (x as f64 / screen.width as f64) * FOV;

            // Find distance to wall
            let step_size = 0.1;
            let mut ray_distance = 0.0;
            let mut hit_wall = false;

            // Unit vector for ray in player space
            let eye_x = ray_angle.sin();
            let eye_y = ray_angle.cos();

            // Incrementally cast ray from player, along ray angle, testing for
            // intersection with a block
            while !hit_wall && ray_distance < MAX_RENDER_DIST {
                ray_distance += step_size;
                let dx = player_x + eye_x * ray_distance;
                let dy = player_y + eye_y * ray_distance;

                // Test if ray is out of bounds
                if dx < 0.0 || dy >= screen.width as f64 || dy < 0.0 || dy >= screen.height as f64 {
                    // Just set distance to maximum depth
                    hit_wall = true;
                    ray_distance = MAX_RENDER_DIST;
                } else {
                    // Check if ray hit a wall cell
                    if map[dx as usize][dy as usize] == '#' {
                        hit_wall = true;
                    }
                }
            }

            // Calculate distance to ceiling and floor
            let ceiling_index =
                (screen.height as f64 / 2.0) - (screen.height as f64 / ray_distance);
            let ceiling_index = ceiling_index as usize;
            let floor_index = screen.height - ceiling_index;

            for y in 0..screen.height {
                if y < ceiling_index {
                    // Ceiling
                    screen.draw(x, y, " ");
                } else if y > ceiling_index && y <= floor_index {
                    // Wall
                    screen.draw(x, y, &wall_shade(ray_distance).to_string());
                } else {
                    // Floor - Shade based on distance
                    let b = 1.0
                        - ((y as f64 - screen.height as f64 / 2.0) / (screen.height as f64 / 2.0));
                    let floor_shade;
                    if b < 0.25 {
                        floor_shade = "#";
                    } else if b < 0.5 {
                        floor_shade = "x";
                    } else if b < 0.75 {
                        floor_shade = ".";
                    } else if b < 0.9 {
                        floor_shade = "-";
                    } else {
                        floor_shade = " ";
                    };
                    screen.draw(x, y, floor_shade);
                }
            }
        }

        // Draw minimap
        {
            let player_x = player_x as usize;
            let player_y = player_y as usize;
            // Compute directional player icon. Zero degrees is facing "south", to match our
            // coordinate system with origin at upper left.
            let mut player_angle = player_angle % (2.0 * 3.14159);
            if player_angle < 1.0 {
                player_angle *= -1.0;
            }
            let player_icon = if player_angle >= 5.498 || player_angle < 0.785 {
                "v"
            } else if player_angle >= 0.785 && player_angle < 2.356 {
                "<"
            } else if player_angle >= 2.356 && player_angle < 3.927 {
                "^"
            } else {
                ">"
            };

            for (j, row) in map.iter().enumerate() {
                for (i, c) in row.iter().enumerate() {
                    if player_x == i && player_y == j {
                        screen.draw(i + 2, j + 2, player_icon);
                    } else {
                        screen.draw(i + 2, j + 2, &c.to_string());
                    }
                }
            }
        }

        // Write stats
        let fps = 1.0 / t_elapsed.as_secs_f64();
        screen.draw(
            2,
            1,
            &format!(
                "FPS={0:.3}, X={1:.3}, Y={2:.3}, A={3:.3}",
                fps, player_x, player_y, player_angle
            ),
        );

        screen.flush(&mut stdout);
    }
}
