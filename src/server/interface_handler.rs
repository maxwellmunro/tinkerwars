use crate::game::game_data::State;
use crate::server::server::Server;
use crate::texture_handler::{TextureHandler, TextureId};
use crate::ticks;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::HashMap;
use std::f32::consts::PI;
use tokio::sync::RwLockReadGuard;

pub(in crate::server) async fn render_menu(server: &mut Server<'_>) -> Result<(), String> {
    let state = *server.game.state.read().await;

    match state {
        State::Lobby => render_lobby_menu(
            &mut server.windowing.canvas,
            &server.texture_handler,
            server.game.clients.read().await,
        ),
        State::PartPicking => render_part_picking_menu(
            &mut server.windowing.canvas,
            &server.texture_handler,
            server.game.clients.read().await,
            server.game.client_ids.read().await
                [*server.game.picking_id_index.read().await as usize],
        ),
        State::BuildingMenu => {
            render_building_menu(&mut server.windowing.canvas, &server.texture_handler)
        }
        State::InGame => {
            todo!("ingame")
        }
        _ => {
            todo!("Unhandled state: {:?}", state)
        }
    }
}

pub(in crate::server) fn get_lobby_buttons(
    window_w: u32,
    window_h: u32,
    start_button: (u32, u32),
) -> Rect {
    let cx = window_w as i32 / 2;
    let cy = window_h as i32 / 2 + 90;

    Rect::new(
        cx - start_button.0 as i32 / 2,
        cy - start_button.1 as i32 / 2,
        start_button.0,
        start_button.1,
    )
}

pub(in crate::server) fn get_part_picking_buttons(
    start_button: (u32, u32),
    next_part_button: (u32, u32),
) -> (Rect, Rect) {
    (
        Rect::new(
            10,
            next_part_button.1 as i32 + 20,
            start_button.0,
            start_button.1,
        ),
        Rect::new(10, 10, next_part_button.0, next_part_button.1),
    )
}

fn spread_angle(mut x: f32) -> f32 {
    x %= PI * 2.0;

    let d = if x > PI { PI } else { 0.0 };

    ((2.0 * (x - d)) / PI - 1.0).asin() + PI / 2.0 + d
}

fn render_lobby_menu(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
    clients: RwLockReadGuard<HashMap<u64, String>>,
) -> Result<(), String> {
    let (window_w, window_h) = canvas.window().size();

    render_menu_details(canvas, texture_handler)?;

    let cx = window_w as i32 / 2;
    let cy = window_h as i32 / 2 + 90;

    let tex = texture_handler.get_texture(TextureId::StartButton);
    let start_button = get_lobby_buttons(window_w, window_h, tex.1);
    canvas.copy(tex.0, None, Some(start_button))?;

    let r = window_h as i32 / 2 - 250;

    clients
        .iter()
        .enumerate()
        .try_for_each(|(i, (_, username))| {
            let angle = (i as f32 * PI * 2.0 / clients.len() as f32) + PI;

            let x = (cx as f32 + r as f32 * spread_angle(angle).sin() * 1.5) as i32;
            let y = (cy as f32 + r as f32 * spread_angle(angle).cos()) as i32;

            let mut tex = texture_handler.render_text(username, canvas, Color::RGB(42, 36, 34))?;

            tex.1.0 *= 3;
            tex.1.1 *= 3;

            let rect = Rect::new(
                x - tex.1.0 as i32 / 2,
                y - tex.1.1 as i32 / 2,
                tex.1.0,
                tex.1.1,
            );
            canvas.copy(&tex.0, None, Some(rect))?;

            Ok::<(), String>(())
        })?;

    Ok(())
}

fn render_part_picking_menu(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
    clients: RwLockReadGuard<HashMap<u64, String>>,
    picking_id: u64,
) -> Result<(), String> {
    let (window_w, window_h) = canvas.window().size();

    let cx = window_w as i32 / 2;
    let cy = window_h as i32 / 2 + 90;

    let arrow_tex = texture_handler.get_texture(TextureId::Arrow);
    let arrow_rect = Rect::new(
        cx - arrow_tex.1.0 as i32 / 2,
        cy - arrow_tex.1.1 as i32 / 2,
        arrow_tex.1.0,
        arrow_tex.1.1,
    );
    let arrow_center = Point::new(arrow_tex.1.0 as i32 / 2, arrow_tex.1.1 as i32 / 2);

    let r = window_h as i32 / 2 - 250;

    clients
        .iter()
        .enumerate()
        .try_for_each(|(i, (id, username))| {
            let angle = spread_angle((i as f32 * PI * 2.0 / clients.len() as f32) + PI);

            let x = (cx as f32 + r as f32 * angle.sin() * 1.5) as i32;
            let y = (cy as f32 + r as f32 * angle.cos()) as i32;

            let mut tex = texture_handler.render_text(username, canvas, Color::RGB(42, 36, 34))?;

            tex.1.0 *= 3;
            tex.1.1 *= 3;

            let rect = Rect::new(
                x - tex.1.0 as i32 / 2,
                y - tex.1.1 as i32 / 2,
                tex.1.0,
                tex.1.1,
            );
            canvas.copy(&tex.0, None, Some(rect))?;

            if *id == picking_id {
                canvas.copy_ex(
                    arrow_tex.0,
                    None,
                    Some(arrow_rect),
                    ((PI - angle) * 180.0 / PI) as f64,
                    arrow_center,
                    false,
                    false,
                )?;
            }

            Ok::<(), String>(())
        })?;

    let start_button = texture_handler.get_texture(TextureId::StartButton);
    let next_part = texture_handler.get_texture(TextureId::NextPart);

    let (start_rect, next_part_rect) = get_part_picking_buttons(start_button.1, next_part.1);

    canvas.copy(start_button.0, None, Some(start_rect))?;
    canvas.copy(next_part.0, None, Some(next_part_rect))?;

    Ok(())
}

fn render_building_menu(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
) -> Result<(), String> {
    let (window_w, window_h) = canvas.window().size();

    let text = String::from("BUILDING") + {
        let f = (ticks() / 1000) % 4;

        ".".repeat(f as usize)
    }
    .as_str();

    let mut tex = texture_handler.render_text(text.as_str(), canvas, Color::RGB(42, 36, 34))?;
    tex.1.0 *= 3;
    tex.1.1 *= 3;

    let rect = Rect::new(
        (window_w as i32 - tex.1.0 as i32) / 2,
        (window_h as i32 - tex.1.1 as i32) / 2,
        tex.1.0,
        tex.1.1,
    );
    canvas.copy(&tex.0, None, Some(rect))?;

    Ok(())
}

fn render_cog(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
    x: i32,
    y: i32,
    large: bool,
    clockwise: bool,
) -> Result<(), String> {
    let large_cog = texture_handler.get_texture(TextureId::TitleLargeGear);
    let small_cog = texture_handler.get_texture(TextureId::TitleSmallGear);

    let angle = ticks() / 50;

    let tex = if large { large_cog } else { small_cog };

    let rect = Rect::new(
        x - (tex.1.0 as i32 / 2),
        y - (tex.1.1 as i32 / 2),
        tex.1.0,
        tex.1.1,
    );
    let center = Point::new(tex.1.0 as i32 / 2, tex.1.1 as i32 / 2);
    canvas.copy_ex(
        tex.0,
        None,
        Some(rect),
        if clockwise { -1 } else { 1 } as f64 * if large { 0.5 } else { 1.0 } * angle as f64,
        center,
        false,
        false,
    )?;
    Ok::<(), String>(())
}

fn render_menu_details(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
) -> Result<(), String> {
    let (window_w, window_h) = canvas.window().size();

    let (title_tex, (title_w, title_h)) = texture_handler.get_texture(TextureId::Title);
    let title_rect = Rect::new((window_w as i32 - title_w as i32) / 2, 50, title_w, title_h);

    render_cog(
        canvas,
        texture_handler,
        title_rect.x + 40,
        title_rect.y + 40,
        true,
        true,
    )?;
    render_cog(
        canvas,
        texture_handler,
        title_rect.x + 580,
        title_rect.y + 55,
        false,
        false,
    )?;
    render_cog(
        canvas,
        texture_handler,
        title_rect.x + 300,
        title_rect.y + 15,
        false,
        false,
    )?;
    render_cog(
        canvas,
        texture_handler,
        title_rect.x + 150,
        title_rect.y + 115,
        false,
        false,
    )?;
    render_cog(
        canvas,
        texture_handler,
        title_rect.x + 500,
        title_rect.y + 10,
        false,
        true,
    )?;
    render_cog(
        canvas,
        texture_handler,
        title_rect.x + 400,
        title_rect.y + 100,
        true,
        false,
    )?;

    canvas.copy(title_tex, None, Some(title_rect))?;

    let mut seed = 12345;
    let mut t = 0.0;
    let mut i = 0;

    const LARGE_W: f64 = 70.0;
    const SMALL_W: f64 = 35.0;

    while t < 1.0 {
        i += 1;
        seed += 54321;
        seed ^= 12345;

        let large = seed % 5 == 0;

        t += if large { LARGE_W } else { SMALL_W } / window_w as f64;
        let x = t * window_w as f64;
        let y = if large { window_h + 20 } else { window_h + 10 };
        t += if large { LARGE_W } else { SMALL_W } / window_w as f64;
        render_cog(
            canvas,
            texture_handler,
            x as i32,
            y as i32,
            large,
            i % 2 == 0,
        )?;
    }

    Ok(())
}
