use rand::Rng;

use rayon::scope;
use sdl2::{
    event::Event, keyboard::Keycode, pixels::Color, rect::Rect, render::Canvas, video::Window,
};
const WIDTH: usize = 500;
const HEIGHT: usize = 400;
const THREADS: usize = 4;
const CELL_SIZE: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub alive: bool,
    pub color: Option<[u8; 3]>, // RGB color, if alive
}

// Helper function to convert RGB to HSV
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r_f = r as f32 / 255.0;
    let g_f = g as f32 / 255.0;
    let b_f = b as f32 / 255.0;

    let max = r_f.max(g_f).max(b_f);
    let min = r_f.min(g_f).min(b_f);
    let delta = max - min;

    // Calculate hue
    let h = if delta < 0.00001 {
        0.0
    } else if max == r_f {
        60.0 * (((g_f - b_f) / delta) % 6.0)
    } else if max == g_f {
        60.0 * (((b_f - r_f) / delta) + 2.0)
    } else {
        60.0 * (((r_f - g_f) / delta) + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h } / 360.0; // Normalize to 0-1

    let s = if max < 0.00001 { 0.0 } else { delta / max };

    let v = max;

    (h, s, v)
}

// Helper function to convert HSV back to RGB
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h * 360.0; // Convert back to 0-360 degrees

    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r_prime, g_prime, b_prime) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    [
        ((r_prime + m) * 255.0) as u8,
        ((g_prime + m) * 255.0) as u8,
        ((b_prime + m) * 255.0) as u8,
    ]
}

impl Cell {
    pub fn dead() -> Self {
        Self {
            alive: false,
            color: Some([0, 0, 0]),
        }
    }

    pub fn random_alive() -> Self {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        Self {
            alive: true,
            color: Some([
                rng.gen_range(50..=255),
                rng.gen_range(50..=255),
                rng.gen_range(50..=255),
            ]),
        }
    }

    pub fn from_parents(parents: &[Cell]) -> Self {
        let mut color_sum = [0u32; 3];
        let mut count = 0;

        for parent in parents.iter().filter(|c| c.alive && c.color.is_some()) {
            let [r, g, b] = parent.color.unwrap();
            color_sum[0] += r as u32;
            color_sum[1] += g as u32;
            color_sum[2] += b as u32;
            count += 1;
        }

        if count == 0 {
            return Cell::random_alive();
        }

        let mut avg = [
            (color_sum[0] / count) as u8,
            (color_sum[1] / count) as u8,
            (color_sum[2] / count) as u8,
        ];

        for c in avg.iter_mut() {
            let offset: i8 = rand::random::<i8>() % 10; // -9..9
            *c = c.saturating_add_signed(offset);
        }

        let (h, _, v) = rgb_to_hsv(avg[0], avg[1], avg[2]);

        let fixed_saturation = 0.8;
        let new_rgb = hsv_to_rgb(h, fixed_saturation, v);

        Self {
            alive: true,
            color: Some(new_rgb),
        }
    }
}

struct GameOfLife {
    grid: Vec<Cell>,
}

impl GameOfLife {
    fn new() -> Self {
        let mut grid = vec![Cell::dead(); WIDTH * HEIGHT];
        for position in 0..WIDTH * HEIGHT {
            let mut rng = rand::thread_rng();
            let n: u32 = rng.gen_range(0..2);

            if n > 0 {
                let cell = Cell::random_alive();
                grid[position] = cell;
            }
        }
        Self { grid }
    }

    fn step(&mut self) {
        let current_grid = self.grid.clone();
        let mut new_grid = self.grid.clone();

        let chunk_size = HEIGHT / THREADS;

        // Pre-split new_grid into slices to avoid overlapping mutable borrows
        let mut_chunks: Vec<&mut [Cell]> = new_grid.chunks_mut(chunk_size * WIDTH).collect();

        scope(|s| {
            for (t, new_slice) in mut_chunks.into_iter().enumerate() {
                let from = t * chunk_size;
                let to = if t == THREADS - 1 {
                    HEIGHT
                } else {
                    (t + 1) * chunk_size
                };

                let current_grid = &current_grid;

                s.spawn(move |_| {
                    for y in from..to {
                        for x in 0..WIDTH {
                            let global_idx = y * WIDTH + x;
                            let local_idx = (y - from) * WIDTH + x;

                            // Count neighbors and collect parent cells
                            let mut neighbors = 0;
                            let mut parent_cells = Vec::with_capacity(8);

                            for dy in -1..=1 {
                                for dx in -1..=1 {
                                    if dx == 0 && dy == 0 {
                                        continue;
                                    }
                                    let nx = (x as isize + dx + WIDTH as isize) as usize % WIDTH;
                                    let ny = (y as isize + dy + HEIGHT as isize) as usize % HEIGHT;
                                    let n_idx = ny * WIDTH + nx;

                                    if current_grid[n_idx].alive {
                                        neighbors += 1;
                                        parent_cells.push(current_grid[n_idx]);
                                    }
                                }
                            }

                            let alive = current_grid[global_idx].alive;
                            let will_be_alive = matches!((alive, neighbors), (true, 2) | (_, 3));

                            if will_be_alive {
                                if !alive && neighbors == 3 {
                                    // A new cell is born, inherit color from parents
                                    new_slice[local_idx] = Cell::from_parents(&parent_cells);
                                } else {
                                    // Cell stays alive, keep its current color - DON'T DARKEN IT
                                    new_slice[local_idx].alive = true;
                                    // Keep the original color - no darkening
                                    new_slice[local_idx].color = current_grid[global_idx].color;
                                }
                            } else {
                                // Cell dies or stays dead
                                new_slice[local_idx] = Cell::dead();
                            }
                        }
                    }
                });
            }
        });

        self.grid = new_grid;
    }
}

fn draw_grid(canvas: &mut Canvas<Window>, game: &GameOfLife) {
    //TODO: color by organism instead of by cell alive / dead;
    canvas.set_draw_color(Color::RGB(50, 50, 50));
    canvas.clear();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let cell_index = y * WIDTH + x;
            let cell = game.grid[cell_index];
            // println!("cell drawing index: {}", game.grid[cell_index].alive);
            if cell.alive {
                // if let Some(rgb) = cell.color {
                //     println!("  Color: [{}, {}, {}]", rgb[0], rgb[1], rgb[2]);
                // } else {
                //     println!("  No color defined");
                // }
                if let Some(rgb) = cell.color {
                    canvas.set_draw_color(Color::RGB(rgb[0], rgb[1], rgb[2]));
                }
                let _ = canvas.fill_rect(Rect::new(
                    (x as u32 * CELL_SIZE) as i32,
                    (y as u32 * CELL_SIZE) as i32,
                    CELL_SIZE,
                    CELL_SIZE,
                ));
            }
        }
    }
    canvas.present();
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window(
            "Conway's Game of Life",
            WIDTH as u32 * CELL_SIZE,
            HEIGHT as u32 * CELL_SIZE,
        )
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    let mut game = GameOfLife::new();
    // println!("creating new game");
    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    println!("close icon detected, quitting.");
                    break 'running;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    ..
                } => {
                    println!("Q key pressed, quitting.");
                    break 'running;
                }
                _ => {}
            }
        }
        game.step();
        draw_grid(&mut canvas, &game);
        //TODO: generation slider instead of delay; so the user can see any generation;
        // thread::sleep(Duration::from_millis(DELAY_MS));
    }
}
