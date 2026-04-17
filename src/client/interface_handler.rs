use crate::client::client::Client;
use crate::client::component_list::ComponentListSet;
use crate::game::game_data::State;
use crate::texture_handler::{TextureHandler, TextureId, destroy};
use crate::ticks;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::HashMap;
use tokio::sync::RwLockReadGuard;

pub(in crate::client) async fn render_menu(client: &mut Client<'_>) -> Result<(), String> {
    let state = *client.game.state.read().await;

    match state {
        State::MainMenu => render_main_menu(&mut client.windowing.canvas, &client.texture_handler),
        State::JoiningMenu => render_joining_menu(
            &mut client.windowing.canvas,
            &client.texture_handler,
            &client.server_address,
            &client.username,
            client.typing_ip,
            client.typing_username,
        ),
        State::Connecting => {
            render_connecting_menu(&mut client.windowing.canvas, &client.texture_handler)
        }
        State::ConnectFailed => render_connect_failed_menu(
            &mut client.windowing.canvas,
            &client.texture_handler,
            client.game.connect_err.read().await,
        ),
        State::Lobby => render_lobby_menu(
            &mut client.windowing.canvas,
            &client.texture_handler,
            client.game.clients.read().await,
        ),
        State::PartPicking => render_part_picking_menu(
            &mut client.windowing.canvas,
            &client.texture_handler,
            client.game.component_list.read().await,
        ),
        State::BuildingMenu => {
            client
                .building_menu
                .render(
                    &mut client.windowing,
                    &client.texture_handler,
                    client.game.clone(),
                )
                .await
        }

        State::InGame => {
            todo!("Game renderer")
        }
    }
}

fn render_main_menu(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
) -> Result<(), String> {
    let (window_w, window_h) = canvas.window().size();

    render_menu_details(canvas, texture_handler)?;

    let play_button = texture_handler.get_texture(TextureId::PlayButton);
    let quit_button = texture_handler.get_texture(TextureId::QuitButton);

    let (play_button_rect, quit_button_rect) =
        get_main_menu_buttons(window_w, window_h, play_button.1, quit_button.1);

    canvas.copy(play_button.0, None, Some(play_button_rect))?;
    canvas.copy(quit_button.0, None, Some(quit_button_rect))?;

    Ok(())
}

pub(in crate::client) fn get_main_menu_buttons(
    window_w: u32,
    window_h: u32,
    play_button: (u32, u32),
    quit_button: (u32, u32),
) -> (Rect, Rect) {
    let play_button_rect = Rect::new(
        (window_w as i32 - play_button.0 as i32) / 2,
        250.max(window_h as i32 / 2 - play_button.1 as i32 - 10),
        play_button.0,
        play_button.1,
    );
    let quit_button_rect = Rect::new(
        (window_w as i32 - quit_button.0 as i32) / 2,
        play_button_rect.bottom() + 20,
        quit_button.0,
        quit_button.1,
    );
    (play_button_rect, quit_button_rect)
}

pub(in crate::client) fn get_joining_menu_buttons(
    window_w: u32,
    window_h: u32,
    ip_label: (u32, u32),
    ip_box: (u32, u32),
    join_button: (u32, u32),
    username_label: (u32, u32),
    username_box: (u32, u32),
) -> (Rect, Rect, Rect, Rect, Rect) {
    let ip_line_w = ip_label.0 + ip_box.0 + 20;
    let ip_label_rect = Rect::new(
        (window_w as i32 - ip_line_w as i32) / 2,
        250.max(window_h as i32 / 2 - (join_button.1 as f32 * 1.5) as i32),
        ip_label.0,
        ip_label.1,
    );
    let ip_box_rect = Rect::new(
        ip_label_rect.right() + 20,
        ip_label_rect.y(),
        ip_box.0,
        ip_box.1,
    );

    let username_line_w = username_label.0 + username_box.0 + 20;

    let username_label_rect = Rect::new(
        (window_w as i32 - username_line_w as i32) / 2,
        ip_label_rect.bottom() + 20,
        username_label.0,
        username_label.1,
    );

    let username_box_rect = Rect::new(
        username_label_rect.right() + 20,
        username_label_rect.y(),
        username_box.0,
        username_box.1,
    );

    let join_button_rect = Rect::new(
        (window_w as i32 - join_button.0 as i32) / 2,
        username_box_rect.bottom() + 20,
        join_button.0,
        join_button.1,
    );

    (
        username_label_rect,
        username_box_rect,
        ip_label_rect,
        ip_box_rect,
        join_button_rect,
    )
}

fn render_joining_menu(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
    address: &str,
    username: &str,
    typing_ip: bool,
    typing_username: bool,
) -> Result<(), String> {
    let (window_w, window_h) = canvas.window().size();

    let ip_label = texture_handler.get_texture(TextureId::IpLabel);
    let join_button = texture_handler.get_texture(TextureId::JoinButton);
    let ip_box = texture_handler.get_texture(TextureId::IpBox);
    let username_label = texture_handler.get_texture(TextureId::UsernameLabel);
    let username_box = texture_handler.get_texture(TextureId::UsernameBox);

    let (username_label_rect, username_box_rect, ip_label_rect, ip_box_rect, join_button_rect) =
        get_joining_menu_buttons(
            window_w,
            window_h,
            ip_label.1,
            ip_box.1,
            join_button.1,
            username_label.1,
            username_box.1,
        );

    canvas.copy(ip_label.0, None, Some(ip_label_rect))?;
    canvas.copy(join_button.0, None, Some(join_button_rect))?;
    canvas.copy(ip_box.0, None, Some(ip_box_rect))?;
    canvas.copy(username_label.0, None, Some(username_label_rect))?;
    canvas.copy(username_box.0, None, Some(username_box_rect))?;

    let mut ip_text = texture_handler.render_text(address, canvas, Color::RGB(42, 36, 34))?;
    ip_text.1.0 *= 3;
    ip_text.1.1 *= 3;

    let ip_text_rect = Rect::new(
        if (ip_text.1.0 as i32) < ip_box.1.0 as i32 - 40 {
            ip_box_rect.x() + 20
        } else {
            ip_box_rect.right() - ip_text.1.0 as i32 - 20
        },
        ip_box_rect.y() + (ip_box_rect.height() as i32 - ip_text.1.1 as i32) / 2,
        ip_text.1.0,
        ip_text.1.1,
    );

    let clip = canvas.clip_rect();
    canvas.set_clip_rect(Rect::new(
        ip_box_rect.x() + 20,
        ip_box_rect.y(),
        ip_box_rect.width() - 40,
        ip_box_rect.height(),
    ));
    canvas.copy(&ip_text.0, None, Some(ip_text_rect))?;
    destroy(ip_text);

    canvas.set_clip_rect(clip);

    if typing_ip {
        let rect = Rect::new(
            ip_text_rect.right(),
            ip_box_rect.y() + 10,
            10,
            ip_box_rect.height() - 20,
        );

        canvas.set_draw_color(Color::RGB(42, 36, 34));
        canvas.fill_rect(rect)?;
    }

    /////////////////

    let mut username_text =
        texture_handler.render_text(username, canvas, Color::RGB(42, 36, 34))?;
    username_text.1.0 *= 3;
    username_text.1.1 *= 3;

    let username_text_rect = Rect::new(
        if (username_text.1.0 as i32) < username_box.1.0 as i32 - 40 {
            username_box_rect.x() + 20
        } else {
            username_box_rect.right() - username_text.1.0 as i32 - 20
        },
        username_box_rect.y() + (username_box_rect.height() as i32 - username_text.1.1 as i32) / 2,
        username_text.1.0,
        username_text.1.1,
    );

    let clip = canvas.clip_rect();
    canvas.set_clip_rect(Rect::new(
        username_box_rect.x() + 20,
        username_box_rect.y(),
        (username_box_rect.width() as i32 - 40) as u32,
        username_box_rect.height(),
    ));
    canvas.copy(&username_text.0, None, Some(username_text_rect))?;
    destroy(username_text);

    canvas.set_clip_rect(clip);

    if typing_username {
        let rect = Rect::new(
            username_text_rect.right(),
            username_box_rect.y() + 10,
            10,
            (username_box_rect.height() as i32 - 20) as u32,
        );

        canvas.set_draw_color(Color::RGB(42, 36, 34));
        canvas.fill_rect(rect)?;
    }

    render_menu_details(canvas, texture_handler)?;

    Ok(())
}

fn render_connecting_menu(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
) -> Result<(), String> {
    let (window_w, window_h) = canvas.window().size();

    render_menu_details(canvas, texture_handler)?;

    let text = String::from("CONNECTING") + {
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
    destroy(tex);

    Ok(())
}

pub fn render_connect_failed_menu(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
    err: RwLockReadGuard<String>,
) -> Result<(), String> {
    let (window_w, window_h) = canvas.window().size();

    render_menu_details(canvas, texture_handler)?;

    let words = err.split_whitespace().collect::<Vec<_>>();
    let mut lines = vec![String::new()];
    let mut cur_len = 0;

    words.into_iter().for_each(|w| {
        cur_len += w.len();
        if cur_len < 20 {
            lines.last_mut().unwrap().push(' ');
            lines.last_mut().unwrap().push_str(w);
        } else {
            lines.push(w.to_string());
            cur_len = 0;
        }
    });

    const LINE_HEIGHT: i32 = 96;

    let top_y = (window_h as i32 - lines.len() as i32 * LINE_HEIGHT) / 2;

    let mut i = 0;

    for line in lines {
        let mut text = texture_handler.render_text(&line, canvas, Color::RGB(42, 36, 34))?;
        text.1.0 *= 4;
        text.1.1 *= 4;

        let x = (window_w as i32 - text.1.0 as i32) / 2;
        let y = top_y + (i * 96);

        i += 1;

        let rect = Rect::new(x, y, text.1.0, text.1.1);
        canvas.copy(&text.0, None, Some(rect))?;
        destroy(text);
    }

    Ok(())
}

fn render_lobby_menu(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
    clients: RwLockReadGuard<HashMap<u64, String>>,
) -> Result<(), String> {
    let (window_w, window_h) = canvas.window().size();

    render_menu_details(canvas, texture_handler)?;

    let top = (window_h as i32 / 2) - (clients.len() as i32 * 48);

    let mut i = 0;

    for username in clients.values() {
        let mut text = texture_handler.render_text(username, canvas, Color::RGB(42, 36, 34))?;

        text.1.0 *= 4;
        text.1.1 *= 4;

        let rect = Rect::new(
            (window_w as i32 - text.1.0 as i32) / 2,
            top + (i * 96),
            text.1.0,
            text.1.1,
        );

        i += 1;

        canvas.copy(&text.0, None, Some(rect))?;
        destroy(text);
    }

    Ok(())
}

fn render_part_picking_menu(
    canvas: &mut Canvas<Window>,
    texture_handler: &TextureHandler,
    parts: RwLockReadGuard<ComponentListSet>,
) -> Result<(), String> {
    parts
        .items()
        .iter()
        .try_for_each(|part| part.1.render(canvas, texture_handler))?;

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
