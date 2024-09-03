use std::{num::NonZeroU32, rc::Rc};

use anyhow::Result;
use flume::Receiver;
use num_complex::{Complex, ComplexFloat};
use rustfft::FftPlanner;
use softbuffer::{Buffer, Context, Surface};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoopBuilder},
    platform::windows::EventLoopBuilderExtWindows,
    window::{self, WindowAttributes, WindowId},
};

type Window = Rc<window::Window>;

const COLOR_SCHEME: &[Color] = &[
    Color::hex(0x000000),
    Color::hex(0x742975),
    Color::hex(0xDD562E),
    Color::hex(0xFD9719),
    Color::hex(0xFFD76B),
    Color::hex(0xFFFFFF),
];

struct Debug {
    state: Option<State>,
    rx: Receiver<Vec<Complex<f32>>>,
}

struct State {
    window: Window,
    _context: Context<Window>,
    surface: Surface<Window, Window>,
    size: (u32, u32),
}

impl ApplicationHandler for Debug {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Rc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );
        let context = Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();

        self.state = Some(State {
            window,
            _context: context,
            surface,
            size: (0, 0),
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();
        if window_id != state.window.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let mut buffer = state.surface.buffer_mut().unwrap();
                draw(&mut buffer, state.size, &self.rx);
                buffer.present().unwrap();
                state.window.request_redraw();
            }
            WindowEvent::Resized(new_size) => {
                state.size = (new_size.width, new_size.height);
                state
                    .surface
                    .resize(
                        NonZeroU32::new(new_size.width).unwrap(),
                        NonZeroU32::new(new_size.height).unwrap(),
                    )
                    .unwrap();
            }
            _ => (),
        }
    }
}

fn draw(buffer: &mut Buffer<Window, Window>, size: (u32, u32), rx: &Receiver<Vec<Complex<f32>>>) {
    while let Ok(mut row) = rx.try_recv() {
        FftPlanner::new()
            .plan_fft_forward(row.len())
            .process(&mut row);
        let row = &row[0..row.len() / 2];

        let last_row = (&buffer[size.0 as usize..(size.0 * size.1) as usize]).to_owned();
        buffer[0..(size.0 * (size.1 - 1)) as usize].copy_from_slice(&last_row);

        let bins_per_pixel = row.len() as f32 / size.0 as f32;
        for x in 0..size.0 {
            let start = (x as f32 * bins_per_pixel) as usize;
            let end = ((x + 1) as f32 * bins_per_pixel) as usize;
            let val = row[start..end].iter().sum::<Complex<f32>>() / (end - start) as f32;
            let color = color(1.0 - (-val.abs()).exp());
            buffer[(size.0 * (size.1 - 1) + x) as usize] = color.to_u32();
        }
    }
}

pub fn start(rx: Receiver<Vec<Complex<f32>>>) -> Result<()> {
    let event_loop = EventLoopBuilder::default().with_any_thread(true).build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut Debug { state: None, rx })?;
    Ok(())
}

fn color(val: f32) -> Color {
    debug_assert!((0. ..=1.).contains(&val));
    let sections = COLOR_SCHEME.len() - 2;
    let section = (sections as f32 * val).floor() as usize;

    COLOR_SCHEME[section].lerp(
        &COLOR_SCHEME[section + 1],
        val * sections as f32 - section as f32,
    )
}

#[derive(Copy, Clone)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    const fn hex(hex: u32) -> Self {
        Self::new(
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
        )
    }

    const fn to_u32(&self) -> u32 {
        (self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32
    }

    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(
            (self.r as f32 + (other.r as f32 - self.r as f32) * t) as u8,
            (self.g as f32 + (other.g as f32 - self.g as f32) * t) as u8,
            (self.b as f32 + (other.b as f32 - self.b as f32) * t) as u8,
        )
    }
}
