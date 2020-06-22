use crate::{
    shipyard::{
        *,
    },
    consts::{
        *,
    },
    tetra::{
        math::{
            Vec3,
            Vec2,
        },
        graphics::{
            Camera,
            Color,
        },
        input::{
            InputContext,
        },
    },
};

use vermarine_lib::{
    rendering::{
        draw_buffer::{
            DrawBuffer,
            DrawCommand,
        },
        Drawables,
    },
};

use rand::SeedableRng;
use rand::Rng;
use rand::rngs::StdRng;

pub struct HexTileData {
    pub ground_height: u8,
    pub wall_height: u8,
}

impl HexTileData {
    pub fn new(height: u8) -> HexTileData {
        HexTileData {
            ground_height: height,
            wall_height: height,
        }
    }
}

pub struct HexMap {
    pub tiles: Vec<HexTileData>,
    pub width: usize,
    pub height: usize,
    pub position: Vec2<f32>,
    pub tallest: u8,
}

impl HexMap {
    pub fn new(width: usize, height: usize) -> Self {
        let mut rand = StdRng::seed_from_u64(100);
        let mut tiles = Vec::<HexTileData>::with_capacity(width * height);
        let mut tallest = 0;
        for _ in 0..width * height {
            let value = rand.gen_range(0, MAX_FLOOR_HEIGHT + 1);
            tiles.push(HexTileData::new(value));
            if value > tallest {
                tallest = value;
            }
        }

        HexMap {
            tiles,
            width,
            height,
            position: Vec2::new(-(width as f32) / 2.0, -(height as f32) / 2.0),
            tallest,
        }
    }

    /// Returns a hex in offset coords
    pub fn pixel_to_hex(&mut self, pos: Vec2<f32>) -> Option<(i32, i32)> {
        let mut tallest_height: Option<(u8, i32, i32)> = None;

        for height in 0..=self.tallest {
            let height_offset = height as f32 * FLOOR_DEPTH_STEP;

            let mut pos = pos;
            pos -= Vec2::new(18., 18.);
            pos.x -= self.position.x * FLOOR_WIDTH;
            pos.y -= self.position.y * FLOOR_VERT_STEP;
            pos.y += height_offset;
    
            let size_x = FLOOR_WIDTH / f32::sqrt(3.0);
            // See axial_to_pixel for comment on why this value
            let size_y = 18.66666666666666666;
    
            let pos = Vec2::new(
                pos.x / size_x,
                pos.y / size_y,
            );
    
            let b0 = f32::sqrt(3.0) / 3.0;
            let b1 = -1.0 / 3.0;
            let b2 = 0.0;
            let b3 = 2.0 / 3.0;
    
            let q: f32 = b0 * pos.x + b1 * pos.y;
            let r: f32 = b2 * pos.x + b3 * pos.y;

            let (q, r, s) = (q, r, -r -q);

            let (q, r, _) = cube_round(q, r, s);
    
            let (x, y) = cube_to_offset(q, r);

            if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
                continue;
            }

            let tile = &self.tiles[self.width * y as usize + x as usize];
            let tile_height = tile.wall_height;

            if tile_height != height {
                continue;
            }
            if tallest_height.is_none() || tile_height > tallest_height.unwrap().0 {
                tallest_height = Some((tile_height, x, y));
            }
        }

        if let Some((_, x, y)) = tallest_height {
            return Some((x, y));
        }
        None
    }

    #[allow(dead_code)]
    pub fn axial_to_pixel(&mut self, q: f32, r: f32) -> (f32, f32) {
        let size_x = FLOOR_WIDTH / f32::sqrt(3.0);
        // this value is derived by solving for X in:
        // FLOOR_VERT_STEP * R = X * (3.0 / 2.0 * R) 
        // R can be 1 so we can simplify to:
        // FLOOR_VERT_STEP = X * 1.5
        // X = FLOOR_VERT_STEP / 1.5
        let size_y = 18.66666666666666666;

        let x = size_x * (f32::sqrt(3.0) * q + f32::sqrt(3.0) / 2.0 * r);
        let y = size_y * (3.0 / 2.0 * r);
        (x + 18. + (self.position.x * FLOOR_WIDTH), y + 18. + (self.position.y * FLOOR_VERT_STEP))
    }
}

fn cube_to_offset(q: i32, r: i32) -> (i32, i32) {
    let col = q + (r - (r & 1)) / 2;
    let row = r;

    (col, row)
}

#[allow(dead_code)]
fn offset_to_cube(off_x: i32, off_y: i32) -> (i32, i32, i32) {
    let x = off_x - (off_y - (off_y as i32 & 1)) / 2;
    let z = off_y;
    let y = -x-z;
    
    (x, y, z)
}

fn cube_round(q: f32, r: f32, s: f32) -> (i32, i32, i32) {
    let mut qi = q.round() as i32;
    let mut ri = r.round() as i32;
    let mut si = s.round() as i32;

    let q_diff = f64::abs(qi as f64 - q as f64);
    let r_diff = f64::abs(ri as f64 - r as f64);
    let s_diff = f64::abs(si as f64 - s as f64);

    if q_diff > r_diff && q_diff > s_diff {
        qi = -ri - si;
    } else if r_diff > s_diff {
        ri = -qi - si;
    } else {
        si = -qi - ri;
    }

    (qi, ri, si)
}

pub fn render_hex_map(input_ctx: UniqueView<InputContext>, drawables: NonSendSync<UniqueViewMut<Drawables>>, mut draw_buffer: UniqueViewMut<DrawBuffer>, mut map: UniqueViewMut<HexMap>, camera: UniqueView<Camera>) {
    draw_buffer.new_command_pool(true);

    let mouse_pos = camera.mouse_position(&input_ctx);
    let selected_hex = map.pixel_to_hex(mouse_pos);

    let camera_pos: Vec2<f32> = camera.position / Vec2::new(FLOOR_WIDTH, FLOOR_VERT_STEP) - map.position;

    let startx = (camera_pos.x - 20.0).max(0.0).min(map.width as f32 - 1.0) as usize;
    let endx = (camera_pos.x + 20.0).max(0.0).min(map.width as f32 - 1.0) as usize;
    let starty = (camera_pos.y - 20.0).max(0.0).min(map.height as f32 - 1.0) as usize;
    let endy = (camera_pos.y + 20.0).max(0.0).min(map.height as f32 - 1.0) as usize;

    let (top_tex, wall_tex, brick_tex, brick_floor_tex) = (drawables.alias[textures::FLOOR], drawables.alias[textures::WALL], drawables.alias[textures::WALL_BRICK], drawables.alias[textures::FLOOR_BRICK]);
    for y in starty..=endy {
        for i in 0..3 {
            for x in startx..=endx {
                let (draw_x, draw_y) =
                (
                    if y % 2 == 1 {
                        (x as i32) as f32 * FLOOR_WIDTH + (FLOOR_WIDTH / 2.0)
                    } else {
                        (x as i32) as f32 * FLOOR_WIDTH
                    },
                    (y as i32) as f32 * (FLOOR_VERT_STEP)
                );

                let (draw_x, draw_y) =
                    (
                        draw_x + map.position.x * FLOOR_WIDTH,
                        draw_y + map.position.y * FLOOR_VERT_STEP,
                    );
                let tile = &map.tiles[map.width * y + x];

                if i == 0 {
                    render_hex_walls(&mut draw_buffer, draw_x, draw_y, tile, wall_tex);
                }
                if i == 1 {
                    render_hex_bricks(&mut draw_buffer, draw_x, draw_y, tile, brick_tex);
                }
                if i == 2 {
                    let color = if let Some((sel_x, sel_y)) = selected_hex {
                        let color = if x == sel_x as usize && y == sel_y as usize {
                            Color::RED
                        } else {
                            Color::WHITE
                        };
                        color
                    } else {
                        Color::WHITE
                    };


                    if tile.ground_height >= tile.wall_height {
                        render_hex_top(&mut draw_buffer, draw_x, draw_y, tile.ground_height, top_tex, color);
                    } else {
                        render_hex_brick_top(&mut draw_buffer, draw_x, draw_y, tile.wall_height, brick_floor_tex, color);
                    };
                }
            }
        }
    }

    // Draw dots at hex centers
    /*let marker_tex = drawables.alias[textures::MARKER];
    for y_tile in starty..=endy {
        for x_tile in startx..=endx {
            let (q, _, s) = offset_to_cube(x_tile as i32, y_tile as i32);
            let (x, y) = map.axial_to_pixel(q as f32, s as f32);
            let tile = &map.tiles[map.width * y_tile + x_tile];

            draw_buffer.draw(
                DrawCommand::new(marker_tex)
                    .position(Vec3::new(
                        x - 2.0, y - 2.0, tile.wall_height as f32 * FLOOR_DEPTH_STEP 
                    ))
                    .draw_iso(true)
            );
        }
    }*/

    draw_buffer.end_command_pool();
}

pub fn render_hex_top(draw_buffer: &mut DrawBuffer, x: f32, y: f32, height: u8, texture: u64, color: Color) {
    let mut draw_command = create_floor_draw_cmd(x, y, height as f32 * FLOOR_DEPTH_STEP, height, texture); 
    if color != Color::WHITE {
        draw_command = draw_command.color(color);
    }
    draw_buffer.draw(draw_command);
}

fn create_floor_draw_cmd(x: f32, y: f32, height: f32, color: u8, texture: u64) -> DrawCommand {
    let color = 
        if color == 0 {
            let v = 0.55;
            Color::rgba(v, v, v, 1.0)
        } else if color == 1 {
            let v = 0.8;
            Color::rgba(v, v, v, 1.0)
        } else {
            let v = 1.0;
            Color::rgba(v, v, v, 1.0)
        };

    DrawCommand::new(texture)
        .position(Vec3::new(x, y, height))
        .draw_layer(draw_layers::FLOOR)
        .draw_iso(true)
        .color(color)
}

pub fn render_hex_brick_top(draw_buffer: &mut DrawBuffer, x: f32, y: f32, height: u8, texture: u64, color: Color) {
    let mut draw_command = create_brick_floor_draw_cmd(x, y, height as f32 * FLOOR_DEPTH_STEP, height, texture); 
    if color != Color::WHITE {
        draw_command = draw_command.color(color);
    }
    draw_buffer.draw(draw_command);
}

fn create_brick_floor_draw_cmd(x: f32, y: f32, height: f32, color: u8, texture: u64) -> DrawCommand {
    let color = 
        if color == 1 {
            let v = 0.65;
            Color::rgba(v, v, v, 1.0)
        } else if color == 2 {
            let v = 0.8;
            Color::rgba(v, v, v, 1.0)
        } else if color == 3 {
            let v = 0.9;
            Color::rgba(v, v, v, 1.0)
        } else {
            let v = 1.0;
            Color::rgba(v, v, v, 1.0)
        };

    DrawCommand::new(texture)
        .position(Vec3::new(x, y, height))
        .draw_layer(draw_layers::FLOOR)
        .draw_iso(true)
        .color(color)
}

pub fn render_hex_walls(draw_buffer: &mut DrawBuffer, x: f32, y: f32, tile: &HexTileData, wall_tex: u64) {
    let height = tile.ground_height as f32;
    let start_height = height * FLOOR_DEPTH_STEP - WALL_VERT_OFFSET;
    for i in 0..height as usize {
        let color = 
            if (height as usize - i) % 2 == 1 {
                1
            } else {
                2
            };
        
        draw_buffer.draw(
            create_wall_draw_cmd(x, y, start_height - (i as f32 * WALL_VERT_STEP), color, wall_tex)
        );
    }
}

fn create_wall_draw_cmd(x: f32, y: f32, height: f32, color: u8, texture: u64) -> DrawCommand {
    let color =
        if color == 1 {
            let v = 0.5;
            Color::rgba(v, v, v, 1.0)
        } else if color == 2{
            let v = 0.7;
            Color::rgba(v, v, v, 1.0)
        } else {
            let v = 1.0;
            Color::rgba(v, v, v, 1.0)
        };

    DrawCommand::new(texture)
        .position(Vec3::new(x, y, height))
        .draw_layer(draw_layers::WALL)
        .draw_iso(true)
        .color(color)
}

pub fn render_hex_bricks(draw_buffer: &mut DrawBuffer, x: f32, y: f32, tile: &HexTileData, brick_tex: u64) {
    let start_height = tile.ground_height as f32 * FLOOR_DEPTH_STEP - WALL_VERT_STEP;
    if tile.wall_height > tile.ground_height {
        for i in 1..=(tile.wall_height - tile.ground_height) {
            let color = tile.ground_height + i;
            draw_buffer.draw(
                create_wall_brick_draw_cmd(x, y, start_height + (i as f32 * WALL_VERT_STEP), color, brick_tex)
            );
        }
    }
}

fn create_wall_brick_draw_cmd(x: f32, y: f32, height: f32, color: u8, texture: u64) -> DrawCommand {
    let color =
        if color == 1 {
            let v = 0.3;
            Color::rgba(v, v, v, 1.0)
        } else if color == 2 {
            let v = 0.55;
            Color::rgba(v, v, v, 1.0)
        } else if color == 3 {
            let v = 0.7;
            Color::rgba(v, v, v, 1.0)
        } else {
            let v = 0.80;
            Color::rgba(v, v, v, 1.0)
        };

    DrawCommand::new(texture)
        .position(Vec3::new(x, y, height))
        .draw_layer(draw_layers::WALL)
        .draw_iso(true)
        .color(color)
}