use crate::client::building::Robot;
use crate::client::component_list::ComponentListSet;
use crate::game::component::ComponentKind;
use crate::game::world::World;
use bincode::{Decode, Encode};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Encode, Decode)]
pub(crate) enum State {
    #[default]
    MainMenu,
    JoiningMenu,
    Connecting,
    ConnectFailed,
    Lobby,
    PartPicking,
    BuildingMenu,
    InGame,
}

#[derive(Clone, Default)]
pub(crate) struct GameData {
    pub(crate) clients: Arc<RwLock<HashMap<u64, String>>>,
    pub(crate) client_ids: Arc<RwLock<Vec<u64>>>,
    pub(crate) id: Arc<RwLock<u64>>,
    pub(crate) state: Arc<RwLock<State>>,
    pub(crate) chat: Arc<RwLock<VecDeque<String>>>,
    pub(crate) connect_err: Arc<RwLock<String>>,
    pub(crate) window_size: Arc<RwLock<(u32, u32)>>,
    pub(crate) my_turn_picking: Arc<RwLock<bool>>,
    pub(crate) picking_id_index: Arc<RwLock<u64>>,
    pub(crate) component_list: Arc<RwLock<ComponentListSet>>,
    pub(crate) building_components: Arc<RwLock<HashMap<ComponentKind, u64>>>,
    pub(crate) robot: Arc<RwLock<Robot>>,
    pub(crate) part_picking_scoll: i32,
}
