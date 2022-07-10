mod parser;

use smithay_client_toolkit::{
    default_environment,
    environment::SimpleGlobal,
    new_default_environment,
    output::{with_output_info, OutputInfo},
    reexports::{
        calloop::{timer::Timer, EventLoop, LoopSignal},
        client::protocol::{wl_output, wl_shm, wl_surface},
        client::{Attached, Main},
        protocols::wlr::unstable::layer_shell::v1::client::{
            zwlr_layer_shell_v1, zwlr_layer_surface_v1,
        },
    },
    shm::AutoMemPool,
    WaylandSource,
};

use std::cell::{RefCell, Cell};
use std::rc::Rc;
use std::env;

use font_loader::system_fonts;
use rusttype::{point, Font, Scale, PositionedGlyph};

use parser::Config;

default_environment!(Env,
    fields = [
        layer_shell: SimpleGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    ],
    singles = [
        zwlr_layer_shell_v1::ZwlrLayerShellV1 => layer_shell
    ],
);

struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(PartialEq, Copy, Clone)]
enum RenderEvent {
    Configure { width: u32, height: u32 },
    Closed,
}

struct Surface {
    surface: wl_surface::WlSurface,
    layer_surface: Main<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    next_render_event: Rc<Cell<Option<RenderEvent>>>,
    pool: AutoMemPool,
    dimensions: (u32, u32),
    vec_canvas: Vec<u32>
}

impl Surface {
    fn new(
        //output: &wl_output::WlOutput,
        surface: wl_surface::WlSurface,
        layer_shell: &Attached<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
        pool: AutoMemPool,
        display_dimensions: (u32, u32),
        config: Rc<Config>,
        text: Vec<String>,
    ) -> Self {

        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            //Some(output), // maybe in the future if we are able to specify a monitor
            None, // only recently used monitor
            zwlr_layer_shell_v1::Layer::Overlay,
            "gwstuff".to_owned(),
        );

        // Calc window dimensions and get glyphs alread positioned
        let ((win_w, win_h), vec_canvas) = get_dimensions_and_canvas(Rc::clone(&config), &text);
        
        layer_surface.set_size(win_w, win_h);

        let anchor = zwlr_layer_surface_v1::Anchor::from_raw(config.window.win_position.unwrap().0.to_raw() | config.window.win_position.unwrap().1.to_raw()).unwrap();

        if !anchor.contains(zwlr_layer_surface_v1::Anchor::from_raw(15).unwrap()) {

            layer_surface
                .set_anchor(anchor);

            let calc_px_margin = |val: u8, tot: u32| ((val as u32 * tot) / 100) as i32;

            let horizontal_margin_px = calc_px_margin(config.margins.horizontal_percentage, display_dimensions.0);
            let vertical_margin_px = calc_px_margin(config.margins.vertical_percentage, display_dimensions.1);

            let get_proper_margin = |a: zwlr_layer_surface_v1::Anchor, val: i32| if anchor.contains(a) { val } else { 0 };

            layer_surface
                .set_margin(
                    get_proper_margin(zwlr_layer_surface_v1::Anchor::Top, vertical_margin_px),
                    get_proper_margin(zwlr_layer_surface_v1::Anchor::Right, vertical_margin_px),
                    get_proper_margin(zwlr_layer_surface_v1::Anchor::Bottom, vertical_margin_px),
                    get_proper_margin(zwlr_layer_surface_v1::Anchor::Left, horizontal_margin_px),
                );
        }

        let next_render_event = Rc::new(Cell::new(None::<RenderEvent>));
        let next_render_event_handle = Rc::clone(&next_render_event);
        layer_surface.quick_assign(move |layer_surface, event, _| {
            match (event, next_render_event_handle.get()) {
                (zwlr_layer_surface_v1::Event::Closed, _) => {
                    next_render_event_handle.set(Some(RenderEvent::Closed));
                }
                (zwlr_layer_surface_v1::Event::Configure { serial, width, height }, next)
                    if next != Some(RenderEvent::Closed) =>
                {
                    layer_surface.ack_configure(serial);
                    next_render_event_handle.set(Some(RenderEvent::Configure { width, height }));
                }
                (_, _) => {}
            }
        });

        // Commit so that the server will send a configure event
        surface.commit();

        // TODO how this work? Why need (0, 0) in dimensions?
        Self { 
            surface, 
            layer_surface, 
            next_render_event, 
            pool, 
            //dimensions: (win_w, win_h), 
            dimensions: (0, 0), 
            vec_canvas
        }
    }

    /// Handles any events that have occurred since the last call, redrawing if needed.
    /// Returns true if the surface should be dropped.
    fn handle_events(&mut self) -> bool {
        match self.next_render_event.take() {
            Some(RenderEvent::Closed) => true,
            Some(RenderEvent::Configure { width, height }) => {
                if self.dimensions != (width, height) {
                    self.dimensions = (width, height);
                    self.draw();
                }
                false
            }
            None => false,
        }
    }

    fn draw(&mut self) {
        let stride = 4 * self.dimensions.0 as i32;
        let width = self.dimensions.0 as i32;
        //let width_win = self.dimensions.0 as i32;
        let height = self.dimensions.1 as i32;

        // Note: unwrap() is only used here in the interest of simplicity of the example.
        // A "real" application should handle the case where both pools are still in use by the
        // compositor.
        let (canvas, buffer) =
            self.pool.buffer(width, height, stride, wl_shm::Format::Argb8888).unwrap();


        //draw_line(canvas, (width as u32, height as u32), (50, 10), (100, 200), 2, (155, 0, 0));

        for (index, color) in self.vec_canvas.iter().enumerate() {


            let pixel = color.to_ne_bytes();
            let pixel_canvas = canvas.chunks_exact_mut(4).nth(index).unwrap();

            pixel_canvas[0] = pixel[0];
            pixel_canvas[1] = pixel[1];
            pixel_canvas[2] = pixel[2];
            pixel_canvas[3] = pixel[3];
        }

        // here I should place the canvas_vec into the canvas


        // Attach the buffer to the surface and mark the entire surface as damaged
        self.surface.attach(Some(&buffer), 0, 0);
        self.surface.damage_buffer(0, 0, width as i32, height as i32);

        // Finally, commit the surface
        self.surface.commit();
    }
}

fn get_canvas(config: Rc<Config>, text_and_width: &Vec<(Vec<PositionedGlyph>, u32)>, dimensions: (u32, u32)) -> Vec<u32> {

    let mut canvas: Vec<u32> = Vec::new();
    set_backgorund(Rc::clone(&config), &mut canvas, dimensions);

    //let pixel = config.font.color.to_ne_bytes();

    let dim_y = text_and_width[0].0[0].scale().y as u32;
    let mut init_x: u32;
    let mut init_y: u32 = config.window.vertical_padding;

    for (glyphs, width_line) in text_and_width.iter() {

        match config.font.text_alignment {
            parser::TextAlignment::Left => {
                init_x = config.window.horizontal_padding;
            },
            parser::TextAlignment::Right => {
                init_x = dimensions.0 - config.window.horizontal_padding - width_line;
            },
            parser::TextAlignment::Center => {
                init_x = (dimensions.0 / 2) - (width_line / 2);
            }
        }

        for g in glyphs.iter() {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw(|x, y, v| {

                    // v should be in the range 0.0 to 1.0
                    let x = x as i32 + bb.min.x;
                    let y = y as i32 + bb.min.y;
                    // There's still a possibility that the glyph clips the boundaries of the bitmap
                    if x >= 0 && x < *width_line as i32 && y >= 0 && y < dim_y as i32 /*&& v >= 0.1*/ {
                        let x = x as u32;
                        let y = y as u32;
                        
                        //let pixel = add_opacity(config.font.color, 0);
                        //println!("{}", (v * 255.0).floor() as u8);
                        //let pixel = add_opacity(config.font.color, (v * 255.0).floor() as u8);
                        //
                        let mul_color = |color: u32, coverage: f32| -> u32 {

                            let mut color_bytes = color.to_ne_bytes();
                            for i in 0..4 {
                                color_bytes[i] = (color_bytes[i] as f32 * coverage).floor() as u8;
                            }
                            ((color_bytes[3] as u32) << 24) + ((color_bytes[2] as u32) << 16) + ((color_bytes[1] as u32) << 8) + (color_bytes[0] as u32)
                        };

                        // config color: rgb 0000rrrrggggbbbbbbbbb

                        let pixel_font = mul_color(add_opacity(config.font.color, 255), v);
                        let pixel_bg = mul_color(add_opacity(config.window.background_color, percentage_to_u8(config.window.background_opacity)), 1.0 - v);
                        let pixel = (pixel_font + pixel_bg) as u32;

                        canvas[(init_x + x + ((init_y + y) * dimensions.0)) as usize] = pixel;
                        
                        /*
                        let pixel_canvas = canvas.chunks_exact_mut(4).nth((init_x + x + ((init_y + y) * dimensions.0)) as usize).unwrap();

                        pixel_canvas[0] = pixel[0];
                        pixel_canvas[1] = pixel[1];
                        pixel_canvas[2] = pixel[2];
                        pixel_canvas[3] = pixel[3];
                        */
                    }
                })
            }
        }
        init_y += (config.font.intra_line as u32) + dim_y;
    }
    canvas
}

fn get_dimensions_and_canvas(config: Rc<Config>, text: &Vec<String>) -> ((u32, u32), Vec<u32>) {

    let (font, scale) = load_font_and_scale(config.font.name.clone(), config.font.size);

    let mut text_glyphs_and_width: Vec<(Vec<PositionedGlyph>, u32)> = Vec::new();

    let v_metrics = font.v_metrics(scale);

    //let text_glyphs_and_width: Rc<RefCell<Vec<(Vec<PositionedGlyph>, u32)>>> = Rc::new(RefCell::new(Vec::new()));

    let mut win_h: f32 = 0.0;
    let mut win_w: u32 = 0;

    for (index, line) in text.iter().enumerate() {

        let layout_iter: rusttype::LayoutIter = font.layout(line, scale, point(0.0, v_metrics.ascent));
        let glyphs: Vec<PositionedGlyph> = layout_iter.collect();

        // Find the most visually pleasing width to display
        let width_line = glyphs
            .iter()
            .rev()
            .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
            .next()
            .unwrap_or(0.0)
            .ceil() as usize;

        text_glyphs_and_width.push((glyphs, width_line as u32));

        if width_line as u32 > win_w {
            win_w = width_line as u32;
        }

        win_h += scale.y;

        if index != 0 && index != text.len() {
            win_h += config.font.intra_line;
        }
    }

    win_w += 2 * config.window.horizontal_padding;
    win_h += (2 * config.window.vertical_padding) as f32;

    //((win_w, win_h.ceil() as u32), Rc::clone(&text_glyphs_and_width))
    ((win_w, win_h.ceil() as u32), get_canvas(Rc::clone(&config), &text_glyphs_and_width, (win_w, win_h.ceil() as u32))) 
}

// FONT LOAD + SCALE DIMENSION -> TODO properly
fn load_font_and_scale(font_name: String, font_size: f32) -> (Font<'static>, Scale) {
    
    // LOAD FONT
    let property = system_fonts::FontPropertyBuilder::new().family(&font_name[..]).build();
    let (font_data, _) = system_fonts::get(&property).unwrap();
    
    // RUSTTYPE
    let font = Font::try_from_vec(font_data).unwrap_or_else(|| {
        panic!( "error constructing a Font from data at");
    });
    
    let px_font = font_size * 96.0 / 72.0;
                            
    let scale = Scale::uniform(px_font);

    (font, scale)
}

fn set_backgorund (config: Rc<Config>, canvas_vec: &mut Vec<u32>, dimensions: (u32, u32)) {

    let opacity = percentage_to_u8(config.window.background_opacity);
    let pixel = add_opacity(config.window.background_color, opacity);
    for _ in 0..dimensions.1 {
        for _ in 0..dimensions.0 {
            canvas_vec.push(pixel);
        }
    }

}

fn to_pixel(r: u8, g: u8, b: u8, t: u8) -> [u8; 4] {
    (((t as u32) << 24) + ((r as u32) << 16) + ((g as u32) << 8) + (b as u32)).to_ne_bytes()
}

fn add_opacity(color: u32, transparency: u8) -> u32 {
    ((transparency as u32) << 24 ) + color
}

fn percentage_to_u8(t: u32) -> u8 {
    ((t * 255) / 100) as u8
}

fn length ((x, y): (f32, f32)) -> f32 {
    f32::sqrt(x.powf(2.0) + y.powf(2.0))
}

fn normalize ((x, y): (f32, f32)) -> (f32, f32) {
    let ln = length((x, y));
    ((x / ln), (y / ln))
}

/*
fn draw_line(canvas : &mut [u8], (buf_x, buf_y): (u32, u32), (x_init, y_init): (u32, u32),(x_end, y_end): (u32, u32), thikness: u32, (r, g, b): (u32, u32, u32)) {

    //println!("dim x: {}", buf_x);
    //println!("dim y: {}", buf_y);

    let pixel = to_pixel(r, g, b, 1);
    let half_thik = thikness as f32 / 2.0;

    // Vector between the start and end of the line
    //let vec1: (f32, f32) = (x_init as f32 - x_end as f32, y_init as f32 - y_end as f32);
    let vec1: (f32, f32) = (x_end as f32 - x_init as f32, y_end as f32 - y_init as f32);
    let len_vec1 = length(vec1);
    //println!("length of the retta: {}", len_vec1);

    for (i, dst_pixel) in canvas.chunks_exact_mut(4).enumerate() {

        let x = (i as u32 % buf_x) as f32;
        let y = (i as u32 / buf_y) as f32;

        //println!("i: {}", i);
        //println!("x: {}", x);
        //println!("y: {}", y);

        // vector be the start and the i-esimo? point
        //let vec2: (f32, f32) = (x_init as f32 - x as f32, y_init as f32 - y as f32);
        let vec2: (f32, f32) = (x - x_init as f32, y - y_init  as f32);

        let len_proj = (vec1.0 * vec2.0 + vec1.1 * vec2.1) / len_vec1;

        if len_proj > len_vec1 {
            continue;
        }

        let normalize_vec1 = normalize(vec1);
        let vec_proj = (normalize_vec1.0 * len_proj, normalize_vec1.1 * len_proj);

        let point_proj = (x_init as f32 + vec_proj.0, y_init as f32 + vec_proj.1);

        //let anti_len_proj = length((x_end as f32 - vec_proj.0 , y_end as f32 - vec_proj.1));
        let anti_len_proj = length((point_proj.0 - x_end as f32 , point_proj.1 - y_end as f32 ));

        if anti_len_proj > len_vec1 {
            continue;
        }

        //let len_vec2 =  length(vec2);

        //let ln = f32::sin(f32::acos(len_proj / len_vec2)) * len_vec2;
        //let ln = f32::sqrt(len_vec2.powf(2.0) - len_proj.powf(2.0));
        let point_proj = (x_init as f32 + vec_proj.0, y_init as f32 + vec_proj.1);
        let ln = length((point_proj.0 - x, point_proj.1 - y));

        if ln <= half_thik {
            dst_pixel[0] = pixel[0];
            dst_pixel[1] = pixel[1];
            dst_pixel[2] = pixel[2];
            dst_pixel[3] = pixel[3];
        }
    }
}
*/

impl Drop for Surface {
    fn drop(&mut self) {
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}

fn main() {

    // Take from line argument the text and render the glyph + screen size
    let mut args: Vec<String> = env::args().collect();
    args.remove(0); // remove the name of the file

    if args.len() == 0 {
        println!("No text specified");
        return;
    }

    let config_name: Option<String> = match args[0][..2] == "--".to_owned() {
        true => {
            let name = args[0][2..].to_string();
            args.remove(0);
            Some(name)
        },
        false => None,
    };

    let gwstuff_config: Rc<Config> = Rc::new(parser::init_toml_config(config_name));
    let duration_timer = gwstuff_config.window.duration as u64;

    let (env, display, queue) =
        new_default_environment!(Env, fields = [layer_shell: SimpleGlobal::new(),])
            .expect("Initial roundtrip failed!");

    let surfaces = Rc::new(RefCell::new(Vec::new()));

    let layer_shell = env.require_global::<zwlr_layer_shell_v1::ZwlrLayerShellV1>();

    let env_handle = env.clone();
    let surfaces_handle = Rc::clone(&surfaces);
    let output_handler = move |output: wl_output::WlOutput, info: &OutputInfo| {

        let mut display_dim: (u32, u32) = (1, 1);
        for &mode in info.modes.iter() {
            if mode.is_current {
                display_dim = (mode.dimensions.0 as u32, mode.dimensions.1 as u32);
            }
        }

        if info.obsolete {
            // an output has been removed, release it
            surfaces_handle.borrow_mut().retain(|(i, _)| *i != info.id);
            output.release();
        } else {
            // an output has been created, construct a surface for it
            let surface = env_handle.create_surface().detach();
            let pool = env_handle.create_auto_pool().expect("Failed to create a memory pool!");
            (*surfaces_handle.borrow_mut())
                .push(
                    (
                        info.id, Surface::new(
                                                //&output,
                                                surface,
                                                &layer_shell.clone(),
                                                pool,
                                                //(gwstuff_config.window.width, gwstuff_config.window.height),
                                                display_dim,
                                                //zwlr_layer_surface_v1::Anchor::from_raw(win_position.0.to_raw() | win_position.1.to_raw()).unwrap(), // TODO remove unwrap
                                                //(gwstuff_config.margins.horizontal_percentage, gwstuff_config.margins.vertical_percentage),
                                                Rc::clone(&gwstuff_config),
                                                args.clone()
                                             )
                       )
                    );
        }
    };

    // Process currently existing outputs
    for output in env.get_all_outputs() {
        if let Some(info) = with_output_info(&output, Clone::clone) {
            output_handler(output, &info);
        }
   }

    // Setup a listener for changes
    // The listener will live for as long as we keep this handle alive
    let _listner_handle =
        env.listen_for_outputs(move |output, info, _| output_handler(output, info));

    //let mut event_loop = EventLoop::<()>::try_new().unwrap();

    // START

    // Create the event loop. The loop is parameterised by the kind of shared
    // data you want the callbacks to use. In this case, we want to be able to
    // stop the loop when the timer fires, so we provide the loop with a
    // LoopSignal, which has the ability to stop the loop from within events. We
    // just annotate the type here; the actual data is provided later in the
    // run() call.

    let mut event_loop: EventLoop<LoopSignal> =
        EventLoop::try_new().expect("Failed to initialize the event loop!");

    // Retrieve a handle. It is used to insert new sources into the event loop
    // It can be cloned, allowing you to insert sources from within source
    // callbacks.
    let handle = event_loop.handle();

    // Create our event source, a timer. Note that this is also parameterised by
    // the data for the events it generates. We've let Rust infer that here.
    let source = Timer::new().expect("Failed to create timer event source!");

    // Most event source APIs provide two things: an event source to go into the
    // event loop, and some way of triggering that source from elsewhere. In
    // this case, we use a handle to the timer to set timeouts.
    //
    // Note that this can go before or after the call to insert_source(), and
    // even inside another event callback.
    let timer_handle = source.handle();
    timer_handle.add_timeout(std::time::Duration::from_millis(duration_timer), "Timeout reached!");

    // Inserting an event source takes this general form. It can also be done
    // from within the callback of another event source.
    handle
        .insert_source(
            // a type which implements the EventSource trait
            source,
            // a callback that is invoked whenever this source generates an event
            |_event, _metadata, shared_data| {
                // This callback is given 3 values:
                // - the event generated by the source (in our case, a string slice)
                // - &mut access to some metadata, specific to the event source (in our case, a
                //   timer handle)
                // - &mut access to the global shared data that was passed to EventLoop::run or
                //   EventLoop::dispatch (in our case, a LoopSignal object to stop the loop)
                //
                // The return type is just () because nothing uses it. Some
                // sources will expect a Result of some kind instead.

                //println!("Event fired: {}", event);
                shared_data.stop();
            },
        )
        .expect("Failed to insert event source!");

    // Create the shared data for our loop.
    let mut shared_data = event_loop.get_signal();
    // FINISH

    WaylandSource::new(queue).quick_insert(event_loop.handle()).unwrap();

    let mut counter_loop = 0;
    loop {
        // This is ugly, let's hope that some version of drain_filter() gets stabilized soon
        // https://github.com/rust-lang/rust/issues/43244
        {
            let mut surfaces = surfaces.borrow_mut();
            let mut i = 0;
            while i != surfaces.len() {
                if surfaces[i].1.handle_events() {
                    surfaces.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        display.flush().unwrap();
        //event_loop.dispatch(None, &mut ()).unwrap();
        event_loop.dispatch(None, &mut shared_data).unwrap();

        if counter_loop == 2 {
            break;
        }

        counter_loop+=1;
    }
}
