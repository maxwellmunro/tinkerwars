use crate::game::game_data::GameData;
use crate::packet::{TcpPacket, UdpPacket};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub(in crate::server) enum TcpResponse {
    Broadcast {
        packet: TcpPacket,
        sender_addr: Option<SocketAddr>,
    },
    Reply {
        packet: TcpPacket,
        addr: SocketAddr,
    },
}

pub(in crate::server) enum UdpResponse {
    Broadcast {
        packet: UdpPacket,
        sender_addr: SocketAddr,
    },
    Reply {
        packet: UdpPacket,
        addr: SocketAddr,
    },
}

pub(in crate::server) async fn handle_tcp_packet(
    packet: TcpPacket,
    data: GameData,
    id: u64,
    addr: SocketAddr,
    tcp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
    udp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
) -> Vec<TcpResponse> {
    println!("Received TCP packet {:?}", packet);

    match packet {
        TcpPacket::JoinRequest { username } => {
            let contained = data.clients.read().await.iter().any(|c| c.1 == &username);
            if contained {
                return vec![TcpResponse::Reply {
                    packet: TcpPacket::JoinResponseDeny {
                        err: String::from(
                            "A player with the same username is already in the game :/",
                        ),
                    },
                    addr,
                }];
            }

            tcp_addresses.write().await.insert(addr);
            udp_addresses.write().await.insert(addr);

            let mut clients = data.clients.write().await;
            clients.insert(id, username.clone());

            vec![
                TcpResponse::Reply {
                    packet: TcpPacket::JoinResponseSuccess {
                        id,
                        clients: clients.clone(),
                    },
                    addr,
                },
                TcpResponse::Broadcast {
                    packet: TcpPacket::PlayerJoined {
                        user_id: id,
                        username,
                    },
                    sender_addr: Some(addr),
                },
            ]
        }
        TcpPacket::Chat { msg } => {
            println!("Msg: {} from {}", msg, addr);

            vec![TcpResponse::Broadcast {
                packet: TcpPacket::Chat { msg },
                sender_addr: Some(addr),
            }]
        }
        TcpPacket::InternalClientDisconnect | TcpPacket::LeaveRequest => {
            println!("Disconnecting client {}", addr);

            data.clients.write().await.remove(&id);

            vec![TcpResponse::Broadcast {
                packet: TcpPacket::PlayerLeft { user_id: id },
                sender_addr: Some(addr),
            }]
        }
        TcpPacket::PickComponent { comp_id } => {
            let len = data.client_ids.read().await.len();
            let mut cur_id_index = data.picking_id_index.write().await;
            *cur_id_index += 1;

            if *cur_id_index == len as u64 {
                *cur_id_index = 0;
            }

            let index = cur_id_index.clone();
            drop(cur_id_index);

            let new_picking_id = data.client_ids.read().await[index as usize].clone();

            vec![
                TcpResponse::Broadcast {
                    packet: TcpPacket::PickedComponent {
                        user_id: id,
                        comp_id,
                    },
                    sender_addr: Some(addr),
                },
                TcpResponse::Broadcast {
                    packet: TcpPacket::YourTurn { id: new_picking_id },
                    sender_addr: None,
                },
            ]
        }
        _ => vec![],
    }
}

pub(in crate::server) async fn handle_udp_packet(
    packet: UdpPacket,
    data: GameData,
    addr: SocketAddr,
) -> Vec<UdpResponse> {
    println!("Received UDP packet {:?}", packet);

    vec![]
}
