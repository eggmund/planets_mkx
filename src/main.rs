mod tools;
mod planet;

use ggez::event::{self, KeyCode, KeyMods};
use ggez::graphics::{self, DrawParam, Mesh, MeshBuilder};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};
use ggez::timer;
use ggez::input::mouse::MouseButton;

use rand::prelude::*;
use rand::rngs::ThreadRng;

use std::collections::{HashMap, HashSet};
use std::cell::RefCell;
use std::time::Duration;
use std::f32::consts::PI;

use planet::{Planet, PlanetTrail, PLANET_DENSITY};

pub const G: f32 = 0.0001;    // Gravitational constant
pub const TWO_PI: f32 = PI * 2.0;
const SPAWN_PLANET_RADIUS: f32 = 5.0;
const FORCE_DEBUG_VECTOR_MULTIPLIER: f32 = 0.00001;
pub const SCREEN_DIMS: (f32, f32) = (1280.0, 860.0);
const TELEPORT_ON_EDGES: bool = false;       // When edge of window is reached, teleport to other side.

struct MainState {
    planet_id_count: usize,
    planets: HashMap<usize, RefCell<Planet>>,
    planet_trails: HashMap<usize, RefCell<PlanetTrail>>,
    mouse_info: MouseInfo,
    rand_thread: ThreadRng,

    show_planet_info_debug: bool,
    show_vector_debug: bool,
}

impl MainState {
    fn new(_ctx: &mut Context) -> GameResult<MainState> {
        let mut s = MainState {
            planet_id_count: 0,
            planets: HashMap::new(),
            planet_trails: HashMap::new(),
            mouse_info: MouseInfo::default(),
            rand_thread: rand::thread_rng(),

            show_planet_info_debug: false,
            show_vector_debug: false,
        };

        s.restart();

        Ok(s)
    }

    fn restart(&mut self) {
        self.clear();
        // const GAP: f32 = 100.0;
        // self.spawn_square_of_planets(
        //     Point2::new(GAP/2.0, GAP/2.0),
        //     (SCREEN_DIMS.0/GAP).ceil() as u16,
        //     (SCREEN_DIMS.1/GAP).ceil() as u16,
        //     GAP,
        //     10.0,
        // );

        self.add_planet_with_moons(
            Point2::new(640.0, 430.0),
            None,
            None,
            50.0,
            500,
            (15.0, 200.0),
            (0.5, 1.5),
            true,
        );

        // self.add_planet_with_moons(
        //     Point2::new(320.0, 430.0),
        //     None,
        //     None,
        //     50.0,
        //     500,
        //     (15.0, 100.0),
        //     (0.5, 1.5),
        //     true,
        // );
        // self.add_planet_with_moons(
        //     Point2::new(960.0, 430.0),
        //     None,
        //     None,
        //     50.0,
        //     500,
        //     (15.0, 100.0),
        //     (0.5, 1.5),
        //     true,
        // );

        // const DIV: f32 = 100.0;
        // self.add_random_planets(
        //     1000,
        //     (SCREEN_DIMS.0/DIV, SCREEN_DIMS.0 - SCREEN_DIMS.0/DIV),
        //     (SCREEN_DIMS.1/DIV, SCREEN_DIMS.1 - SCREEN_DIMS.1/DIV),
        //     (0.2, 1.0),
        //     Some((500.0, 1000.0)),
        // );
    }

    #[inline]
    fn clear(&mut self) {
        self.planets = HashMap::new();
    }

    #[inline]
    fn add_planet(&mut self, position: Point2<f32>, velocity: Option<Vector2<f32>>, mass: Option<f32>, radius: f32, spawn_protection: Option<Duration>) {
        self.add_planet_raw(Planet::new(
            self.planet_id_count,
            position,
            velocity,
            mass,
            radius,
            spawn_protection,
        ));
    }

    // Spawns a planet with other 
    fn add_planet_with_moons(
        &mut self,
        position: Point2<f32>,
        velocity: Option<Vector2<f32>>,
        main_planet_mass: Option<f32>,
        main_planet_radius: f32,
        moon_num: usize,
        moon_orbit_radius_range: (f32, f32),    // Starting from surface of planet
        moon_body_radius_range: (f32, f32),
        orbit_direction_clockwise: bool,  // anticlockwise = false, clockwise = true
    ) {
        self.add_planet(position, velocity, main_planet_mass, main_planet_radius, None);  // Add main planet
        let (main_planet_mass, frame_velocity) = {
            let p = self.planets.get(&(self.planet_id_count - 1)).unwrap().borrow();
            (p.mass, p.velocity)
        };

        for _ in 0..moon_num {
            let orbit_radius = main_planet_radius + self.rand_thread.gen_range(moon_orbit_radius_range.0, moon_orbit_radius_range.1);
            let orbit_speed = tools::circular_orbit_speed(main_planet_mass, orbit_radius);
            let start_angle = self.rand_thread.gen_range(0.0, TWO_PI);      // Angle from main planet to moon
            let start_pos = tools::get_components(orbit_radius, start_angle);   // Position on circle orbit where planet will start
            let start_velocity = tools::get_components(
                orbit_speed,
                if orbit_direction_clockwise {
                    start_angle + PI/2.0
                } else {
                    start_angle - PI/2.0
                }
            );  // 90 degrees to angle with planet
            let moon_radius = self.rand_thread.gen_range(moon_body_radius_range.0, moon_body_radius_range.1);

            self.add_planet(
                position + start_pos,
                Some(start_velocity + frame_velocity),  // Add velocity of main planet
                None,
                moon_radius,
                None,
            );
        }
    }

    #[inline]
    fn add_planet_raw(&mut self, mut planet: Planet) {
        planet.id = self.planet_id_count;

        self.planet_trails.insert(
            self.planet_id_count,
            RefCell::new(PlanetTrail::new(planet.position))
        );

        self.planets.insert(
            self.planet_id_count,
            RefCell::new(planet)
        );

        self.planet_id_count += 1;
    }

    #[inline]
    fn add_random_planets(&mut self, n: usize, x_range: (f32, f32), y_range: (f32, f32), radius_range: (f32, f32), speed_range: Option<(f32, f32)>) {
        assert!(x_range.1 > x_range.0);
        assert!(y_range.1 > y_range.0);
        assert!(radius_range.1 > radius_range.0);
        assert!(n > 0);

        for _ in 0..n {
            let x_pos = self.rand_thread.gen_range(x_range.0, x_range.1);
            let y_pos = self.rand_thread.gen_range(y_range.0, y_range.1);
            let radius = self.rand_thread.gen_range(radius_range.0, radius_range.1);

            let velocity = if let Some(speed_range) = speed_range {
                assert!(speed_range.1 > speed_range.0);

                let speed = self.rand_thread.gen_range(speed_range.0, speed_range.1);
                let angle = self.rand_thread.gen_range(0.0, TWO_PI);
                Some(tools::get_components(speed, angle))
            } else {
                None
            };

            self.add_planet(
                Point2::new(x_pos, y_pos),
                velocity,
                None,
                radius,
                None,
            );
        }
    }

    #[inline]
    fn remove_planet(&mut self, id: usize) {
        if self.planets.remove(&id).is_none() {
            println!("WARNING: Tried to remove planet {} but it wasn't in the hashmap.", id);
        }
    }

    #[inline]
    fn draw_debug_info(&self, ctx: &mut Context) -> GameResult {
        let text = graphics::Text::new(
            format!(
                "{:.3}\nBodies: {}\nPlanet Trails: {}\nTrail Node Count: {}",
                timer::fps(ctx),
                self.planets.len(),
                self.planet_trails.len(),
                self.node_count(),
            )
        );
        graphics::draw(
            ctx,
            &text,
            DrawParam::new().dest([10.0, 10.0])
        )
    }

    pub fn draw_mouse_drag(ctx: &mut Context, mouse_info: &MouseInfo) -> GameResult {
        let line = Mesh::new_line(
            ctx,
            &[mouse_info.down_pos, mouse_info.current_drag_position],
            2.0,
            [0.0, 1.0, 0.0, 1.0].into(),
        )?;
        graphics::draw(ctx, &line, DrawParam::default())?;
        tools::draw_circle(ctx, mouse_info.down_pos, SPAWN_PLANET_RADIUS, [1.0, 1.0, 1.0, 0.4].into())?;

        Ok(())
    }

        #[inline]
    fn collide_planets(pl1: &mut Planet, pl2: &Planet) {  // Makes pl1 the new planet
        // Conservation of momentum
        let total_mass = pl1.mass + pl2.mass;
        let total_momentum = pl1.mass * pl1.velocity + pl2.mass * pl2.velocity;
        pl1.radius = tools::inverse_volume_of_sphere(total_mass/PLANET_DENSITY);
        // Use centre of mass as new position
        pl1.position = Point2::new(
            (pl1.position.x * pl1.mass + pl2.position.x * pl2.mass)/total_mass,
            (pl1.position.y * pl1.mass + pl2.position.y * pl2.mass)/total_mass
        );
        pl1.velocity = total_momentum/total_mass;   // Inelastic collision
        pl1.mass = total_mass;
        pl1.update_color(); // Will have changed colour due to increase in mass
    }

    fn spawn_square_of_planets(
        &mut self,
        top_left: Point2<f32>,
        w: u16,
        h: u16,
        gap: f32,
        rad: f32,
    ) {
        for i in 0..w {
            for j in 0..h {
                self.add_planet(
                    Point2::new(top_left.x + i as f32 * gap, top_left.y + j as f32 * gap),
                    None,
                    None,
                    rad,
                    None,
                );
            }
        }
    }

    fn update_planet_trails(&mut self, dt_duration: &Duration) {
        for (id, trail) in self.planet_trails.iter_mut() {
            trail.borrow_mut().update(
                dt_duration,
                if let Some(planet) = self.planets.get(&id) {
                    Some(planet.borrow().position)
                } else {
                    None
                },
            );
        }
    }

    fn node_count(&self) -> usize {
        let mut total = 0;
        for (_, trail) in self.planet_trails.iter() {
            total += trail.borrow().node_count();
        }

        total
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let dt_duration = timer::delta(ctx);
        let dt = timer::duration_to_f64(dt_duration) as f32;

        // For holding planets that have collided
        let mut collided_planets: Vec<usize> = Vec::with_capacity(self.planets.len()/2);
        let mut planets_to_remove: Vec<usize> = Vec::with_capacity(self.planets.len()/2);
        
        // Remove dead particle emitters
        self.planet_trails.retain(|_, trail| !trail.borrow().is_dead());

        let keys: Vec<&usize> = self.planets.keys().collect();
        let len = self.planets.len();

        if len > 0 {
            // Update planets
            for (_, pl) in self.planets.iter() {
                pl.borrow_mut().update(dt, &dt_duration);
            }

            for i in 0..len-1 {
                let already_collided = collided_planets.contains(&i);
                if !already_collided {
                    let pl1 = self.planets.get(keys[i]).expect("Couldn't get planet 1");
                    for j in i+1..len {
                        let already_collided = collided_planets.contains(&j);
                        if !already_collided {
                            let pl2 = self.planets.get(keys[j]).expect("Couldn't get planet 2");
    
                            let (colliding, dist_vec, square_distance, protection) = {
                                let bpl1 = pl1.borrow();
                                let bpl2 = pl2.borrow();
                                let dist_vec = bpl2.position - bpl1.position;
                                let min_dist = bpl1.radius + bpl2.radius;
                                let square_dist = dist_vec.x.powi(2) + dist_vec.y.powi(2);
                                (
                                    // AABB then circle collision
                                    dist_vec.x.abs() <= min_dist && dist_vec.y.abs() <= min_dist && square_dist <= min_dist.powi(2),
                                    dist_vec,
                                    square_dist,
                                    bpl1.has_spawn_protection() || bpl2.has_spawn_protection()
                                )
                            };
            
                            // Check for collision even if they have spawn protection, since I do not want to apply grav
                            // force when planets are inside of each other (as they become very speedy).
                            // protection is true if either planets have spawn protection
                            if colliding && !protection {
                                Self::collide_planets(&mut pl1.borrow_mut(), &pl2.borrow());
                                collided_planets.push(*keys[i]);
                                collided_planets.push(*keys[j]);
                                planets_to_remove.push(*keys[j])
                            } else if !colliding {
                                tools::newtonian_grav(&mut pl1.borrow_mut(), &mut pl2.borrow_mut(), square_distance, dist_vec);
                            }
                        }
                    }
                }
                
            }
        }

        self.planets.retain(|id, _| !planets_to_remove.contains(id));

        // Update trails
        self.update_planet_trails(&dt_duration);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, [0.0, 0.0, 0.0, 1.0].into());

        if self.mouse_info.down && self.mouse_info.button_down == MouseButton::Left &&
            (self.mouse_info.down_pos.x - self.mouse_info.current_drag_position.x).powi(2) +
            (self.mouse_info.down_pos.y - self.mouse_info.current_drag_position.y).powi(2) >= 4.0
        {
            Self::draw_mouse_drag(ctx, &self.mouse_info)?;
            //self.draw_fake_planet(ctx, self.mouse_info.down_pos, 5.0)?;
        }

        // Draw particles
        {
            let mut lines_mesh_builder = MeshBuilder::new();
            let mut can_draw = false;
    
            for (_, trail) in self.planet_trails.iter() {
                if trail.borrow().draw(&mut lines_mesh_builder)? && !can_draw {
                    can_draw = true;
                }
            }
            
            if can_draw {     // Prevents lyon error when building mesh
                let line_mesh = lines_mesh_builder.build(ctx)?;
                graphics::draw(ctx, &line_mesh, DrawParam::default())?;
            }
        }


        // Draw planets on top of particles
        if !self.planets.is_empty() {
            let mut planets_mesh_builder = MeshBuilder::new();

            for (_, planet) in self.planets.iter() {
                planet.borrow().draw(
                    if self.show_planet_info_debug { Some(ctx) } else { None },
                    &mut planets_mesh_builder,
                    self.show_planet_info_debug,
                    self.show_vector_debug,
                )?;
            }
    
            let planets_mesh = planets_mesh_builder.build(ctx)?;
            graphics::draw(ctx, &planets_mesh, DrawParam::default())?;
        }

        self.draw_debug_info(ctx)?;
        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        self.mouse_info.down = true;
        self.mouse_info.button_down = button;
        self.mouse_info.down_pos = Point2::new(x, y);
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        self.mouse_info.down = false;

        if button == MouseButton::Left {
            self.add_planet(
                self.mouse_info.down_pos,
                Some(self.mouse_info.down_pos - Point2::new(x, y)),
                None,
                SPAWN_PLANET_RADIUS,
                None,
            );
        }
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.mouse_info.current_drag_position = Point2::new(x, y);
    }


    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymod: KeyMods,
        _repeat: bool,
    ) {
        match keycode {
            KeyCode::D => self.show_vector_debug = !self.show_vector_debug,
            KeyCode::I => self.show_planet_info_debug = !self.show_planet_info_debug,
            KeyCode::R => self.restart(),
            KeyCode::C => self.clear(),
            _ => (),
        }
    }
}


struct MouseInfo {
    down: bool,
    button_down: MouseButton,
    down_pos: Point2<f32>,
    current_drag_position: Point2<f32>,
}

impl Default for MouseInfo {
    fn default() -> MouseInfo {
        MouseInfo {
            down: false,
            button_down: MouseButton::Left,
            down_pos: Point2::new(0.0, 0.0),
            current_drag_position: Point2::new(1.0, 0.0),
        }
    }
}

pub fn main() -> GameResult {
    use std::path;
    use std::env;
    use ggez::conf::{WindowMode, WindowSetup, NumSamples};

    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    } else {
        path::PathBuf::from("./resources")
    };

    let cb = ggez::ContextBuilder::new("Planets", "ggez")
        .add_resource_path(resource_dir)
        .window_mode(
            WindowMode::default()
                .dimensions(SCREEN_DIMS.0, SCREEN_DIMS.1)
        )
        .window_setup(
            WindowSetup::default()
                .samples(NumSamples::Four)
        );

    let (ctx, event_loop) = &mut cb.build()?;
    let state = &mut MainState::new(ctx)?;
    event::run(ctx, event_loop, state)
}
