use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TileKind {
    Grass  = 0,
    Wall   = 1,
    Tree   = 2,
    Water  = 3,
    Floor  = 4,
    Road   = 5,
    Sand   = 6,
    Lava   = 7,  // impassable, volcanic
    Ice    = 8,  // passable, frozen ground
    Pillar = 9,  // impassable, stone column
    Bramble= 10, // impassable, thorns
    Ruins  = 11, // passable, crumbled stone
    Mud    = 12, // passable, swamp mud
}

impl TileKind {
    pub fn passable(self) -> bool {
        matches!(self,
            Self::Grass | Self::Floor | Self::Road | Self::Sand |
            Self::Ice   | Self::Ruins | Self::Mud
        )
    }
    pub fn symbol(self) -> char {
        match self {
            Self::Grass  => '.',
            Self::Wall   => '#',
            Self::Tree   => 'T',
            Self::Water  => '~',
            Self::Floor  => '.',
            Self::Road   => '+',
            Self::Sand   => ',',
            Self::Lava   => '^',
            Self::Ice    => '_',
            Self::Pillar => 'O',
            Self::Bramble=> '*',
            Self::Ruins  => ':',
            Self::Mud    => ';',
        }
    }
    pub fn from_u8(v: u8) -> Self {
        match v {
            0  => Self::Grass,
            1  => Self::Wall,
            2  => Self::Tree,
            3  => Self::Water,
            4  => Self::Floor,
            5  => Self::Road,
            6  => Self::Sand,
            7  => Self::Lava,
            8  => Self::Ice,
            9  => Self::Pillar,
            10 => Self::Bramble,
            11 => Self::Ruins,
            12 => Self::Mud,
            _  => Self::Grass,
        }
    }
}
