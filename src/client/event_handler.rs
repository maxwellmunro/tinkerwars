use crate::constants;
use crate::game::game_data::State;
use crate::packet::TcpPacket;
use crate::{
    client::{client::Client, interface_handler},
    texture_handler::TextureId,
};
use sdl2::event::WindowEvent;
use sdl2::{event::Event, keyboard::Keycode, mouse::MouseButton, rect::Point};

pub(in crate::client) async fn handle_event(client: &mut Client<'_>, event: Event) {
    let state = client.game.state.read().await.clone();

    let c_event = event.clone();

    match state {
        State::MainMenu => handle_main_menu_event(client, event).await,
        State::JoiningMenu => handle_joining_menu_event(client, event).await,
        State::ConnectFailed => handle_connect_failed_menu_event(client, event).await,
        State::PartPicking => handle_part_picking_event(client, event).await,
        State::BuildingMenu => handle_building_menu_event(client, event).await,
        _ => {}
    };

    if let Event::Window { win_event, .. } = c_event {
        if let WindowEvent::Resized(w, h) = win_event {
            *client.game.window_size.write().await = (w as u32, h as u32);
        }
    }
}

async fn handle_main_menu_event(client: &mut Client<'_>, event: Event) {
    match event {
        Event::MouseButtonDown {
            x, y, mouse_btn, ..
        } => {
            if mouse_btn != MouseButton::Left {
                return;
            }

            let (window_w, window_h) = client.get_windowing().canvas.window().size();

            let play_button = client
                .get_texture_handler()
                .get_texture(TextureId::PlayButton);
            let quit_button = client
                .get_texture_handler()
                .get_texture(TextureId::QuitButton);
            let (join, quit) = interface_handler::get_main_menu_buttons(
                window_w,
                window_h,
                play_button.1,
                quit_button.1,
            );

            let mx = Point::new(x, y);

            if join.contains_point(mx) {
                *client.game.state.write().await = State::JoiningMenu;
            }

            if quit.contains_point(mx) {
                client.stop();
            }
        }
        _ => {}
    }
}

async fn handle_joining_menu_event(client: &mut Client<'_>, event: Event) {
    match event {
        Event::MouseButtonDown {
            x, y, mouse_btn, ..
        } => {
            if mouse_btn != MouseButton::Left {
                return;
            }

            println!("Left button down!");

            let mx = Point::new(x, y);

            let (window_w, window_h) = client.windowing.canvas.window().size();

            let ip_label = client.get_texture_handler().get_texture(TextureId::IpLabel);
            let ip_box = client.get_texture_handler().get_texture(TextureId::IpBox);
            let join_button = client
                .get_texture_handler()
                .get_texture(TextureId::JoinButton);
            let username_label = client
                .get_texture_handler()
                .get_texture(TextureId::UsernameLabel);
            let username_box = client
                .get_texture_handler()
                .get_texture(TextureId::UsernameBox);

            let (username_label, username_box, label, ip_box, join) =
                interface_handler::get_joining_menu_buttons(
                    window_w,
                    window_h,
                    ip_label.1,
                    ip_box.1,
                    join_button.1,
                    username_label.1,
                    username_box.1,
                );

            if join.contains_point(mx) {
                println!("Connecting client!");
                let _ = client.connect();
            }

            client.typing_ip = label.contains_point(mx) || ip_box.contains_point(mx);
            client.typing_username =
                username_label.contains_point(mx) || username_box.contains_point(mx);
        }
        Event::TextInput { text, .. } => {
            if client.typing_ip {
                client.server_address.push_str(&text);
            } else if client.typing_username {
                client.username.push_str(&text);
            }
        }
        Event::KeyDown {
            keycode: Some(k), ..
        } => {
            if k == Keycode::ESCAPE {
                *client.game.state.write().await = State::MainMenu;
                return;
            }
            if client.typing_ip && k == Keycode::BACKSPACE {
                client.server_address.pop();
            } else if client.typing_username && k == Keycode::BACKSPACE {
                client.username.pop();
            }
        }
        _ => {}
    }
}

async fn handle_connect_failed_menu_event(client: &mut Client<'_>, event: Event) {
    match event {
        Event::KeyDown {
            keycode: Some(k), ..
        } => {
            if k == Keycode::ESCAPE {
                *client.game.state.write().await = State::JoiningMenu;
            }
        }
        _ => {}
    }
}

async fn handle_part_picking_event(client: &mut Client<'_>, event: Event) {
    match event {
        Event::MouseButtonDown {
            x, y, mouse_btn, ..
        } => {
            if mouse_btn != MouseButton::Left {
                return;
            }

            if !*client.game.my_turn_picking.read().await {
                return;
            }

            let mut component_list = client.game.component_list.write().await;

            let mut item_selected = None;

            let (window_w, window_h) = client.windowing.canvas.window().size();

            for (id, item) in component_list.items() {
                let dx = (item.cur_x() * window_w as f32) as i32 - x;
                let dy = (item.cur_y() * window_h as f32) as i32 - y;

                let square_d = dx * dx + dy * dy;

                if square_d < constants::PART_SELECT_SQUARE_DIST {
                    item_selected = Some((id, item.kind(), item.count()).clone());
                    break;
                }
            }

            let Some(item_selected) = item_selected else {
                return;
            };

            let (&index, kind, count) = item_selected;

            component_list.remove(index);

            drop(component_list);

            let mut network_handler = client.network_handler.write().await;
            if let Some(network_handler) = network_handler.as_mut() {
                network_handler.queue_tcp_packet(TcpPacket::PickComponent { comp_id: index });
            } else {
                return;
            };

            let mut parts = client.game.building_components.write().await;

            if parts.contains_key(&kind) {
                let new_count = parts[&kind] + count;
                parts.insert(kind, new_count);
            } else {
                parts.insert(kind, count);
            }

            *client.game.my_turn_picking.write().await = false;
        }
        _ => {}
    }
}

async fn handle_building_menu_event(client: &mut Client<'_>, event: Event) {
    client
        .building_menu
        .handle_event(
            &client.windowing,
            &client.texture_handler,
            client.game.clone(),
            event,
        )
        .await;
}
