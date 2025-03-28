use rand::Rng;

use rayon::scope;
use sdl2::{
    event::Event, keyboard::Keycode, pixels::Color, rect::Rect, render::Canvas, video::Window,
};
const WIDTH: usize = 1000;
const HEIGHT: usize = 800;
const THREADS: usize = 4;
const CELL_SIZE: u32 = 1;

struct GameOfLife {
    grid: Vec<bool>,
}

impl GameOfLife {
    fn new() -> Self {
        let mut grid = vec![false; WIDTH * HEIGHT];
        for cell in 0..WIDTH * HEIGHT {
            let mut rng = rand::thread_rng();
            let n: u32 = rng.gen_range(0..2);
            if n > 0 {
                grid[cell] = true;
            }
        }
        Self { grid }
    }

    fn step(&mut self) {
        let current_grid = self.grid.clone();
        let mut new_grid = vec![false; WIDTH * HEIGHT];

        let chunk_size = HEIGHT / THREADS;

        // Pre-split new_grid into slices to avoid overlapping mutable borrows
        let mut_chunks: Vec<&mut [bool]> = new_grid.chunks_mut(chunk_size * WIDTH).collect();

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
                    let thread_id = std::thread::current().id();
                    println!("Thread {:?} processing rows {} to {}", thread_id, from, to);

                    for y in from..to {
                        for x in 0..WIDTH {
                            let global_idx = y * WIDTH + x;
                            let local_idx = (y - from) * WIDTH + x;
                            //TODO: check if this can be multithreaded too;
                            let mut neighbors = 0;
                            for dy in -1..=1 {
                                for dx in -1..=1 {
                                    if dx == 0 && dy == 0 {
                                        continue;
                                    }
                                    let nx = (x as isize + dx + WIDTH as isize) as usize % WIDTH;
                                    let ny = (y as isize + dy + HEIGHT as isize) as usize % HEIGHT;
                                    let n_idx = ny * WIDTH + nx;

                                    if current_grid[n_idx] {
                                        neighbors += 1;
                                    }
                                }
                            }

                            let alive = current_grid[global_idx];
                            new_slice[local_idx] = matches!((alive, neighbors), (true, 2) | (_, 3));
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
    canvas.set_draw_color(Color::BLACK);
    canvas.clear();
    canvas.set_draw_color(Color::WHITE);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if game.grid[y * WIDTH + x] {
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
