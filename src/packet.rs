use std::collections::HashMap;
use crate::game::component::ComponentKind;
use crate::game::game_data::State;
use bincode::{Decode, Encode};

#[derive(Clone, Encode, Decode, Debug)]
pub enum TcpPacket {
    /* S <--- C */ JoinRequest { username: String },
    /* S ---> C */ JoinResponseSuccess { id: u64, clients: HashMap<u64, String>},
    /* S ---> C */ JoinResponseDeny { err: String },
    /* S ---> C */ PlayerJoined { user_id: u64, username: String },

    /* S <--- C */ LeaveRequest,
    /* S ---> C */ PlayerLeft { user_id: u64 },
    /* S ---> C */ Kick,
    /* S <--> S */ InternalClientDisconnect,

    /* S <--> C */ Chat { msg: String },

    /* S ---> C */ ChangeState { state: State },

    /* S ---> C */ AddComponentListItem { kind: ComponentKind, count: u64 },
    /* S ---> C */ YourTurn { id: u64 },
    /* S <--- C */ PickComponent { comp_id: u64 },
    /* S ---> C */ PickedComponent { user_id: u64, comp_id: u64 },
}

#[derive(Clone, Encode, Decode, Debug)]
pub enum UdpPacket {}
