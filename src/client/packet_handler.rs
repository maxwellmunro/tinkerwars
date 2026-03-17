use rand::RngExt;

use crate::client::component_list::ComponentListItem;
use crate::constants;
use crate::game::game_data::{GameData, State};
use crate::packet::{TcpPacket, UdpPacket};

pub(in crate::client) async fn handle_tcp_packet(
    packet: TcpPacket,
    data: GameData,
) -> Vec<TcpPacket> {
    println!("Received TCP packet {:?}", packet);

    match packet {
        TcpPacket::JoinResponseSuccess { id, clients } => {
            *data.id.write().await = id;
            *data.state.write().await = State::Lobby;
            *data.clients.write().await = clients;

            vec![]
        }
        TcpPacket::JoinResponseDeny { err } => {
            *data.state.write().await = State::ConnectFailed;
            *data.connect_err.write().await = err;

            vec![]
        }
        TcpPacket::Chat { msg } => {
            let mut chat = data.chat.write().await;

            chat.push_back(msg);
            while chat.len() > constants::CHAT_MSG_LIMIT {
                chat.pop_front();
            }

            vec![]
        }
        TcpPacket::PlayerJoined { user_id, username } => {
            data.clients.write().await.insert(user_id, username);
            vec![]
        }
        TcpPacket::PlayerLeft { user_id } => {
            data.clients.write().await.remove(&user_id);
            vec![]
        }
        TcpPacket::AddComponentListItem { kind, count } => {
            let mut parts = data.component_list.write().await;
            let size = data.window_size.read().await;

            if size.0 == 0 || size.1 == 0 {
                return vec![];
            }

            let mut rng = rand::rng();

            let mut best = (0.0, 0.0, 0); // (x, y, min_dist)

            for _ in 0..constants::PART_SELECT_POS_ATTEMPTS {
                let target_x = rng.random_range(0.05..0.95);
                let target_y = rng.random_range(0.05..0.95);

                let dist = parts
                    .items()
                    .iter()
                    .map(|(_, c)| {
                        let dx = c.target_x() - target_x;
                        let dy = c.target_y() - target_y;

                        ((dx * dx + dy * dy).sqrt() * 1000.0) as i32
                    })
                    .min();

                let Some(dist) = dist else {
                    continue;
                };

                if dist > best.2 {
                    best = (target_x, target_y, dist);
                }
            }

            parts.insert(ComponentListItem::new(best.0, best.1, kind, count));

            vec![]
        }
        TcpPacket::ChangeState { state } => {
            *data.state.write().await = state;

            vec![]
        }
        TcpPacket::PickedComponent { comp_id, .. } => {
            data.component_list.write().await.remove(comp_id);

            vec![]
        }
        TcpPacket::YourTurn { id } => {
            if id == *data.id.read().await {
                *data.my_turn_picking.write().await = true;
            }

            vec![]
        }
        _ => vec![],
    }
}

pub(in crate::client) async fn handle_udp_packet(
    packet: UdpPacket,
    data: GameData,
) -> Vec<UdpPacket> {
    println!("Received UDP packet {:?}", packet);

    vec![]
}
