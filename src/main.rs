use rand::Rng;
use sdl2::{
    event::Event, keyboard::Keycode, pixels::Color, rect::Rect, render::Canvas, video::Window,
};
use std::{thread, time::Duration};
const WIDTH: usize = 300;
const HEIGHT: usize = 150;
const CELL_SIZE: u32 = 5;
const DELAY_MS: u64 = 100;

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

    fn index(&self, x: isize, y: isize) -> usize {
        let x = (x + WIDTH as isize) as usize % WIDTH;
        let y = (y + HEIGHT as isize) as usize % HEIGHT;
        y * WIDTH + x
    }

    fn count_neighbors(&self, x: isize, y: isize) -> u8 {
        let mut count = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if self.grid[self.index(x + dx, y + dy)] {
                    count += 1;
                }
            }
        }
        count
    }

    fn step(&mut self) {
        let mut new_grid = self.grid.clone();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let idx = self.index(x as isize, y as isize);
                let neighbors = self.count_neighbors(x as isize, y as isize);
                new_grid[idx] = match (self.grid[idx], neighbors) {
                    (true, 2) | (_, 3) => true,
                    _ => false,
                };
            }
        }
        self.grid = new_grid;
    }
}

fn draw_grid(canvas: &mut Canvas<Window>, game: &GameOfLife) {
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
        thread::sleep(Duration::from_millis(DELAY_MS));
    }
}
