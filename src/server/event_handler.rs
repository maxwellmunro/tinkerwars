use crate::game::component::ComponentKind;
use crate::game::game_data::State;
use crate::packet::TcpPacket;
use crate::server::interface_handler::get_part_picking_buttons;
use crate::server::packet_handler::TcpResponse;
use crate::server::server::Server;
use crate::texture_handler::TextureId;
use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::rect::Point;

pub(in crate::server) async fn handle_event(server: &mut Server<'_>, state: State, event: Event) {
    match state {
        State::Lobby => handle_lobby_event(server, event).await,
        State::PartPicking => handle_part_picking_event(server, event).await,
        _ => {}
    }
}

async fn handle_lobby_event(server: &mut Server<'_>, event: Event) {
    match event {
        Event::MouseButtonDown {
            x, y, mouse_btn, ..
        } => {
            if mouse_btn != MouseButton::Left {
                return;
            }

            let (window_w, window_h) = server.windowing.canvas.window().size();

            let start_button = server.texture_handler.get_texture(TextureId::StartButton).1;

            let start_button = crate::server::interface_handler::get_lobby_buttons(
                window_w,
                window_h,
                start_button,
            );

            if start_button.contains_point(Point::new(x, y)) {
                *server.game.state.write().await = State::PartPicking;

                let clients = server.game.clients.read().await.clone();
                *server.game.client_ids.write().await =
                    clients.into_iter().map(|(id, _)| id).collect();
                *server.game.picking_id_index.write().await = 0;

                server
                    .network_handler
                    .queue_tcp_response(TcpResponse::Broadcast {
                        packet: TcpPacket::ChangeState {
                            state: State::PartPicking,
                        },
                        sender_addr: None,
                    });

                server
                    .network_handler
                    .queue_tcp_response(TcpResponse::Broadcast {
                        packet: TcpPacket::YourTurn {
                            id: *server.game.client_ids.read().await.first().unwrap(),
                        },
                        sender_addr: None,
                    });
            }
        }
        _ => {}
    }
}

fn spawn_part(server: &mut Server<'_>) {
    const KINDS: &[ComponentKind] = &[
        ComponentKind::Piston,
        ComponentKind::Motor,
        ComponentKind::ArmLarge,
        ComponentKind::ArmMedium,
        ComponentKind::ArmSmall,
        ComponentKind::ArmTiny,
        ComponentKind::Screw,
    ];

    let i = rand::rng().random_range(0..KINDS.len());
    let kind = KINDS[i];
    let count = rand::rng().random_range(5..=10);

    server
        .network_handler
        .queue_tcp_response(TcpResponse::Broadcast {
            packet: TcpPacket::AddComponentListItem { kind, count },
            sender_addr: None,
        });
}

async fn handle_part_picking_event(server: &mut Server<'_>, event: Event) {
    match event {
        Event::KeyDown {
            keycode: Some(k), ..
        } => {
            if k == Keycode::SPACE {
                spawn_part(server);
            }
        }
        Event::MouseButtonDown {
            x, y, mouse_btn, ..
        } => {
            if mouse_btn != MouseButton::Left {
                return;
            }

            let mouse_pos = Point::new(x, y);

            let (window_w, window_h) = server.windowing.canvas.window().size();

            let start_button = server.texture_handler.get_texture(TextureId::StartButton);
            let next_part = server.texture_handler.get_texture(TextureId::NextPart);

            let (start_rect, next_part_rect) =
                get_part_picking_buttons(start_button.1, next_part.1);

            if start_rect.contains_point(mouse_pos) {
                *server.game.state.write().await = State::BuildingMenu;
                server
                    .network_handler
                    .queue_tcp_response(TcpResponse::Broadcast {
                        packet: TcpPacket::ChangeState {
                            state: State::BuildingMenu,
                        },
                        sender_addr: None,
                    });
            } else if next_part_rect.contains_point(mouse_pos) {
                spawn_part(server);
            }
        }
        _ => {}
    }
}
