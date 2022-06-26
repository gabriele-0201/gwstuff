use smithay_client_toolkit::{
    default_environment,
    environment::SimpleGlobal,
    new_default_environment,
    output::{with_output_info, OutputInfo},
    reexports::{
        calloop,
        client::protocol::{wl_output, wl_shm, wl_surface},
        client::{Attached, Main},
        protocols::wlr::unstable::layer_shell::v1::client::{
            zwlr_layer_shell_v1, zwlr_layer_surface_v1,
        },
    },
    shm::AutoMemPool,
    WaylandSource,
};

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::env;

use font_loader::system_fonts;
use rusttype::{point, Font, Scale};

mod parser;

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
    text: Vec<String>,
}

impl Surface {
    fn new(
        //output: &wl_output::WlOutput,
        surface: wl_surface::WlSurface,
        layer_shell: &Attached<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
        pool: AutoMemPool,
        dimensions: (u32, u32),
        display_dimensions: (u32, u32), // TODO
        anchor: zwlr_layer_surface_v1::Anchor,
        margins: (u8, u8), // margin %
        text: Vec<String>,
    ) -> Self {

        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            //Some(output), // maybe in the future if we are able to specify a monitor
            None, // only recently used monitor
            zwlr_layer_shell_v1::Layer::Overlay,
            "gwstuff".to_owned(),
        );

        layer_surface.set_size(dimensions.0, dimensions.1);

        if !anchor.contains(zwlr_layer_surface_v1::Anchor::from_raw(15).unwrap()) {

            layer_surface
                .set_anchor(anchor);

            let calc_px_margin = |val: u8, tot: u32| ((val as u32 * tot) / 100) as i32;

            let horizontal_margin_px = calc_px_margin(margins.0, display_dimensions.0);
            let vertical_margin_px = calc_px_margin(margins.1, display_dimensions.1);

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
        Self { surface, layer_surface, next_render_event, pool, dimensions: (0, 0), text }
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

        // TODO function for background
        // BACKGROUND
        for dst_pixel in canvas.chunks_exact_mut(4) {
            let pixel = 0xff00ff00u32.to_ne_bytes();
            dst_pixel[0] = pixel[0];
            dst_pixel[1] = pixel[1];
            dst_pixel[2] = pixel[2];
            dst_pixel[3] = pixel[3];
        }

        //draw_line(canvas, (width as u32, height as u32), (50, 10), (100, 200), 2, (155, 0, 0));
        let (font, scale) = load_font_and_scale("Arial", 100.0);


        let init_x: &mut f32 = &mut 10.0;
        let init_y: &mut f32 = &mut 10.0;

        draw_text(canvas, (init_x, init_y), self.dimensions.0 as f32, &font, &scale, &self.text, 0.0, Color { r: 255, g: 0, b: 0 });

        //draw_text(canvas, (&mut x_init, &mut y_init), &font, 14.0, &text, 0.0, 5.0, (height as u32, width as u32));

        // Attach the buffer to the surface and mark the entire surface as damaged
        self.surface.attach(Some(&buffer), 0, 0);
        self.surface.damage_buffer(0, 0, width as i32, height as i32);

        // Finally, commit the surface
        self.surface.commit();
    }
}

// TODO calc properly scale - and font type
fn load_font_and_scale(font_name: &str, font_size: f32) -> (Font, Scale) {
    
    // LOAD FONT
    let property = system_fonts::FontPropertyBuilder::new().family(font_name).build();
    let (font_data, _) = system_fonts::get(&property).unwrap();
    
    // TEST RUSTTYPE
    let font = Font::try_from_vec(font_data).unwrap_or_else(|| {
        panic!( "error constructing a Font from data at");
    });
    
    // FONT LOAD + SCALE DIMENSION
    /*
    let px_font = (font_size / 72.0) * 96.0;
    let height: f32 = px_font; // to get 80 chars across (fits most terminals); adjust as desired
    */
                            
    let scale = Scale::uniform(font_size);

    (font, scale)
}

fn draw_text(canvas : &mut [u8], (init_x, init_y): (&mut f32, &mut f32), width_win: f32, font: &Font, scale: &Scale, text: &Vec<String>, intra_line: f32, color: Color) {

        let v_metrics = font.v_metrics(*scale);
        for line in text {

            let offset = point(*init_y * width_win + *init_x, v_metrics.ascent);

            let glyphs: Vec<_> = font.layout(line, *scale, offset).collect();

            // println!("{:?}", glyphs);

            // Find the most visually pleasing width to display
            let width_line = glyphs
                .iter()
                .rev()
                .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
                .next()
                .unwrap_or(0.0)
                .ceil() as usize;

            for g in glyphs {
                if let Some(bb) = g.pixel_bounding_box() {
                    g.draw(|x, y, v| {
                        
                        // v should be in the range 0.0 to 1.0
                        let x = x as i32 + bb.min.x;
                        let y = y as i32 + bb.min.y;
                        // There's still a possibility that the glyph clips the boundaries of the bitmap
                        if x >= 0 && x < width_line as i32 && y >= 0 && y < scale.y as i32 && v >= 0.01 {
                            let x = x as usize;
                            let y = y as usize;

                            let pixel = to_pixel(color.r, color.g, color.b, (v * 255.0).floor() as u8);
                            let pixel_canvas = canvas.chunks_exact_mut(4).nth(x + (y * width_win as usize)).unwrap();

                            pixel_canvas[0] = pixel[0];
                            pixel_canvas[1] = pixel[1];
                            pixel_canvas[2] = pixel[2];
                            pixel_canvas[3] = pixel[3];
                         }
                    })
                }
            }
            *init_y += intra_line + scale.y;
        }
}

/*
fn draw_text(canvas : &mut [u8], (init_x, init_y): (&mut f32, &mut f32), font: &FontRef, pt_size: f32, text: &String, intra_letter: f32, intra_line: f32, (height, width): (u32, u32)) {

    // Qui devo prendermi tutte le proprieta' del font per capire dove cavolo disegnare il coso
    
    //calc the font unit
    let screen_scale_factor = 1.0;
    let px_per_em = pt_size * screen_scale_factor * (96.0 / 72.0);
    let units_per_em = font.units_per_em().unwrap();
    let height_font_unscaled = font.height_unscaled();
    let px_scale = PxScale::from(px_per_em * height_font_unscaled / units_per_em);

    let scaled_font = font.into_scaled(px_scale);

    let descend = scaled_font.descent();
    let ascent = scaled_font.ascent();

    *init_y += ascent;

    for line in text.lines() {
        // qui devo chiaare il draw letter per ogni lettera ed ipoteticamente passargli anche la
        // posizione di inizio -> devo scegliere se fare io i conti dell'offset oppure utilizzare
        // il metodo with_scale_and_position e quindi impostare anche il point di inizio (penso)
        
        draw_text_line(line, canvas, &scaled_font, (init_x, init_y), intra_letter, (height, width));

        *init_y += descend + intra_line + ascent;


    }

}
*/

/*
fn draw_text_line(line: &str, canvas : &mut [u8], scaled_font: &PxScaleFont<&FontRef>, (init_x, init_y): (&mut f32, &mut f32), intra_letter: f32, (height, width): (u32, u32)) {

    let width_font = scaled_font.scale().x;

    for c in line.chars() {

        draw_letter(c, canvas, &scaled_font, (*init_x, *init_y), (height, width));

        *init_x += width_font + intra_letter;
    }

}
*/

/*
fn draw_letter(letter: char, canvas : &mut [u8], scaled_font: &PxScaleFont<&FontRef>, (init_x, init_y): (f32, f32), (height, width): (u32, u32)) {
        
        let glyph = scaled_font.scaled_glyph(letter);
        let ascent = scaled_font.ascent();
        let descent = scaled_font.descent();
        let h_font = scaled_font.height();

        //println!("{}", ascent);
        //println!("{}", descent);
        //println!("{}", h_font);

        /*
        println!("x: {}", scaled_font.scale().x);
        println!("y: {}", scaled_font.scale().y);
        */

        // TODO TEST

        //calc the font unit
        let font_name = String::from("ciao");
        let font = load_font(&font_name).unwrap();

        let pt_size = 15.0;
        let screen_scale_factor = 1.0;
        let px_per_em = pt_size * screen_scale_factor * (96.0 / 72.0);
        let units_per_em = font.units_per_em().unwrap();
        let height_font_unscaled = font.height_unscaled();
        let px_scale = PxScale::from(px_per_em * height_font_unscaled / units_per_em);

        let q_glyph: Glyph = font.glyph_id(letter).with_scale(px_per_em * height_font_unscaled / units_per_em);

        let mut max_y = 0;
        let mut max_x = 0;
        
        println!("ascent: {}", ascent);
        println!("descent: {}", descent);
        println!("h_font: {}", h_font);

        // Draw it.
        if let Some(q) = /*scaled_font*/font.outline_glyph(/*glyph*/q_glyph) {

            println!("x: {}", q.px_bounds().width());
            println!("y: {}", q.px_bounds().height());

            q.draw(|x, y, c| {
                if c <= 0.3 {
                    return;
                }

                if max_y < y {
                    max_y = y;
                }
                if max_x < x {
                    max_x = x;
                }

                let pixel = to_pixel(255,0, 0, (c * 255.0) as u32);

                //let y_with_offset = if y > ascent.floor() as u32 {init_y - y as f32} else {init_y + y as f32};
                let y_with_offset = y;
                let pixel_canvas = canvas.chunks_exact_mut(4).nth(((init_x as u32 + x) as u32 + (y_with_offset as u32) * width as u32) as usize).unwrap();

                pixel_canvas[0] = pixel[0];
                pixel_canvas[1] = pixel[1];
                pixel_canvas[2] = pixel[2];
                pixel_canvas[3] = pixel[3];
            });
        }
        println!("max_y: {}", max_y);
        println!("max_x: {}", max_x);

}
*/

fn to_pixel(r: u8, g: u8, b: u8, t: u8) -> [u8; 4] {
    (((t as u32) << 24) + ((r as u32) << 16) + ((g as u32) << 8) + (b as u32)).to_ne_bytes()
}

fn length ((x, y): (f32, f32)) -> f32 {
    f32::sqrt(x.powf(2.0) + y.powf(2.0))
}

fn normalize ((x, y): (f32, f32)) -> (f32, f32) {
    let ln = length((x, y));
    ((x / ln), (y / ln))
}

/*
impl ops::Mul<f32> for (f32, f32) {
    type Output = (f32, f32);

    fn add(self, val: f32) -> Output {
        (self.0 * val, self.1 * val)
    }
}
*/

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

    println!("{}", args[0][..1].to_string());

    let config_name: Option<String> = match args[0][..1] == "--".to_owned() {
        true => {
            let name = args[0][2..].to_string();
            args.remove(0);
            Some(name)
        },
        false => None,
    };

    println!("{:?}", args);

    let gwstuff_config: parser::Config = parser::init_toml_config(config_name);

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

        let win_position: (parser::Placement, parser::Placement);

        if let Some(pos) = gwstuff_config.window.win_position {
            win_position = pos;
        }
        else{
            win_position = (parser::Placement::CenterHorizontal, parser::Placement::CenterVertical);
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
                                                (gwstuff_config.window.width, gwstuff_config.window.height),
                                                display_dim,
                                                zwlr_layer_surface_v1::Anchor::from_raw(win_position.0.to_raw() | win_position.1.to_raw()).unwrap(), // TODO remove unwrap
                                                (gwstuff_config.margins.horizontal_percentage, gwstuff_config.margins.vertical_percentage),
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

    let mut event_loop = calloop::EventLoop::<()>::try_new().unwrap();

    WaylandSource::new(queue).quick_insert(event_loop.handle()).unwrap();

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
        event_loop.dispatch(None, &mut ()).unwrap();
    }
}

