// Building Gadgets 2 for Horizon — the horizontal-row planner (pure).
//
// Ported from Building Gadgets 2 (Direwolf20-MC/BuildingGadgets2, MIT) for
// Minecraft 1.21.1 NeoForge. The upstream source was read for behaviour only;
// this is a clean-room Rust rewrite.
//
// Slice 1 (building gadget, horizontal row): a right click with the gadget
// builds a row of the selected block away from the player, starting at the
// clicked block and running along the clicked face's horizontal axis, bounded
// by the tool range and the player's supply. Each placed block consumes one
// item of that block's kind. Modes other than the row, the anchor, undo,
// energy and the GUI are later slices (see notes.md).
//
// This file is pure logic so it compiles and unit-tests as the crate's
// standalone library; the game-side glue lives in the bundle's support module.

pub const GADGET: &str = "buildinggadgets2:gadget_building";

/// Upstream's `GadgetNBT.getToolRange` default: one block.
pub const DEFAULT_RANGE: u32 = 1;
/// Upstream's range slider (`ModeRadialMenu`) allows 1..=15.
pub const MAX_RANGE: u32 = 15;

/// Vanilla's `server.properties` default for `spawn-protection` (blocks around the shared
/// spawn): the value the server uses when its file does not carry the key.
pub const VANILLA_SPAWN_PROTECTION: i32 = 16;

/// `Commands.LEVEL_ADMINS`: an operator bypasses spawn protection
/// (`DedicatedServer.isSpawnProtected`'s `player.hasPermissions(2)`).
pub const LEVEL_ADMINS: u8 = 2;

/// What the game answers about one player's right to build a gadget cell: the same gate
/// vanilla's `ServerLevel.mayInteract` applies to the player's own placement, which the
/// actor-free `Level.setBlock` write does not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditRights {
    /// `Player.mayBuild`: survival and creative may; a spectator may not; an adventure player
    /// depends on the stack (`may_use_at`).
    pub may_build: bool,
    /// `Player.mayUseItemAt`: the game's own "may this player use this stack here" check.
    pub may_use_at: bool,
    /// The level's world border as `[min_x, min_z, max_x, max_z]`, checked per cell.
    pub border: Option<[f64; 4]>,
    /// A dedicated server process (`Dist::DedicatedServer`); an integrated server never
    /// applies spawn protection.
    pub dedicated: bool,
    /// The player's permission level is at least [`LEVEL_ADMINS`].
    pub op: bool,
    /// The server's `spawn-protection` radius, 0 when it is off.
    pub spawn_protection: i32,
    /// The world's shared spawn (`LevelData.getRespawnData`).
    pub spawn: Option<[i32; 3]>,
    /// The destination is in the Overworld, the only dimension spawn protection covers.
    pub in_overworld: bool,
}

/// Whether the player may place a block at `pos`: vanilla's `ServerLevel.mayInteract` plus the
/// build permission its own placement would need.
#[must_use]
pub fn may_edit(pos: [i32; 3], rights: &EditRights) -> bool {
    rights.may_build
        && rights.may_use_at
        && inside_border(pos, rights.border)
        && !spawn_protected(pos, rights)
}

/// `WorldBorder.isWithinBounds`: the block's own x/z against the border's edges.
#[must_use]
pub fn inside_border(pos: [i32; 3], border: Option<[f64; 4]>) -> bool {
    let Some([min_x, min_z, max_x, max_z]) = border else {
        return true;
    };
    let x = f64::from(pos[0]);
    let z = f64::from(pos[2]);
    x >= min_x && x <= max_x && z >= min_z && z <= max_z
}

/// `DedicatedServer.isSpawnProtected(pos, player)`: a non-operator inside a positive
/// `spawn-protection` radius of the Overworld's shared spawn.
#[must_use]
pub fn spawn_protected(pos: [i32; 3], rights: &EditRights) -> bool {
    if !rights.dedicated || rights.op || rights.spawn_protection <= 0 || !rights.in_overworld {
        return false;
    }
    let Some(spawn) = rights.spawn else {
        return false;
    };
    let radius = rights.spawn_protection.max(1);
    (pos[0] - spawn[0]).abs() <= radius && (pos[2] - spawn[2]).abs() <= radius
}

/// `server.properties`'s `spawn-protection` (the first key of that name), or vanilla's default
/// when the file carries no number for it.
#[must_use]
pub fn spawn_protection_from(properties: &str) -> i32 {
    properties
        .lines()
        .filter_map(|line| line.trim().split_once('='))
        .find(|(key, _)| key.trim() == "spawn-protection")
        .and_then(|(_, value)| value.trim().parse::<i32>().ok())
        .unwrap_or(VANILLA_SPAWN_PROTECTION)
}

/// A wheel step on the gadget: upstream's slider moves one block per notch and
/// clamps to 1..=15. Positive `steps` (scroll up) increases the range.
#[must_use]
pub fn scroll_range(current: u32, steps: i32) -> u32 {
    (i64::from(current) + i64::from(steps)).clamp(1, i64::from(MAX_RANGE)) as u32
}

/// The horizontal direction a player faces, from their yaw
/// (`Direction.fromYRot`: 0° faces +Z, 90° faces −X).
#[must_use]
pub fn facing_from_yaw(yaw: f32) -> [i32; 3] {
    match (((yaw / 90.0) + 0.5).floor() as i64).rem_euclid(4) {
        0 => [0, 0, 1],
        1 => [-1, 0, 0],
        2 => [0, 0, -1],
        _ => [1, 0, 0],
    }
}

/// The direction the row grows, exactly as `HorizontalRow.collectWorld`
/// computes it: a horizontal face uses the direction opposite the clicked
/// face (away from the player); a top or bottom face uses the player's facing.
#[must_use]
pub fn row_side(face: [i32; 3], facing: [i32; 3]) -> [i32; 3] {
    if face[1] != 0 {
        [facing[0], 0, facing[2]]
    } else {
        [-face[0], 0, -face[2]]
    }
}

/// The building gadget's modes, as `GadgetModes.setupDefaultModes` registers them
/// for `GadgetTarget.BUILDING`, in its order. The default is `build_to_me`
/// (`GadgetNBT.getMode`, which searches for that id first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Fill between the clicked block and the player (`BuildToMe`).
    BuildToMe,
    /// A wall across the clicked face's perpendicular horizontal axis (`VerticalWall`).
    VerticalWall,
    /// A column along the clicked face, or up and down on a side face (`VerticalColumn`).
    VerticalColumn,
    /// The clicked block's neighbours on the same surface plane (`Surface`).
    Surface,
    /// A wall one block wide running along the face ('HorizontalWall').
    HorizontalWall,
    /// A staircase (`Stairs`).
    Stairs,
    /// A row away from the player (`HorizontalRow`), the 0.1.0 slice.
    HorizontalRow,
    /// A lattice on the clicked block's plane (`Grid`).
    Grid,
}

/// The mode a fresh gadget uses, upstream's `build_to_me`.
pub const DEFAULT_MODE: Mode = Mode::BuildToMe;

impl Mode {
    /// Every mode the building gadget registers, in upstream's registration order.
    pub const ALL: [Mode; 8] = [
        Mode::BuildToMe,
        Mode::VerticalWall,
        Mode::VerticalColumn,
        Mode::Surface,
        Mode::HorizontalWall,
        Mode::Stairs,
        Mode::HorizontalRow,
        Mode::Grid,
    ];

    /// The mode's id path, upstream's `ResourceLocation` path (`build_to_me`).
    #[must_use]
    pub fn path(self) -> &'static str {
        match self {
            Mode::BuildToMe => "build_to_me",
            Mode::VerticalWall => "vertical_wall",
            Mode::VerticalColumn => "vertical_column",
            Mode::Surface => "surface",
            Mode::HorizontalWall => "horizontal_wall",
            Mode::Stairs => "stairs",
            Mode::HorizontalRow => "horizontal_row",
            Mode::Grid => "grid",
        }
    }

    /// The full id upstream stores in `GADGET_MODE` (`buildinggadgets2:build_to_me`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Mode::BuildToMe => "buildinggadgets2:build_to_me",
            Mode::VerticalWall => "buildinggadgets2:vertical_wall",
            Mode::VerticalColumn => "buildinggadgets2:vertical_column",
            Mode::Surface => "buildinggadgets2:surface",
            Mode::HorizontalWall => "buildinggadgets2:horizontal_wall",
            Mode::Stairs => "buildinggadgets2:stairs",
            Mode::HorizontalRow => "buildinggadgets2:horizontal_row",
            Mode::Grid => "buildinggadgets2:grid",
        }
    }

    /// The mode an id names: the full id or the bare path, as the command and a
    /// component written by an older package may carry it.
    #[must_use]
    pub fn from_id(text: &str) -> Option<Self> {
        let text = text.trim();
        let path = text.strip_prefix("buildinggadgets2:").unwrap_or(text);
        Mode::ALL.into_iter().find(|mode| mode.path() == path)
    }
}

/// What the planner reads from the plane of play: where the player's feet and eyes
/// are, which `BuildToMe` and `Stairs` measure against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerView {
    /// `player.blockPosition()`.
    pub block: [i32; 3],
    /// `player.getY()`: the feet's height.
    pub y: f64,
    /// `player.getEyeY()`: the feet plus the eye height.
    pub eye_y: f64,
}

/// What the planner reads from the world.
pub trait World {
    /// Whether a block may be placed at `pos`: replaceable, inside the build
    /// height, able to survive there and not blocked by protection. The glue
    /// decides.
    fn can_place(&mut self, pos: [i32; 3]) -> bool;

    /// Whether two cells hold the same block state, which `Surface` asks about the
    /// block behind a candidate and the clicked block.
    fn same_state(&mut self, a: [i32; 3], b: [i32; 3]) -> bool;
}

/// The cell `n` steps from `from` along `dir`.
const fn offset(from: [i32; 3], dir: [i32; 3], n: i32) -> [i32; 3] {
    [
        from[0] + dir[0] * n,
        from[1] + dir[1] * n,
        from[2] + dir[2] * n,
    ]
}

/// The axis a direction runs along: 0 for x, 1 for y, 2 for z.
const fn axis(dir: [i32; 3]) -> usize {
    if dir[0] != 0 {
        0
    } else if dir[1] != 0 {
        1
    } else {
        2
    }
}

/// `MagicHelpers.invertOnFace`: a value negated on the north, down and west faces.
const fn invert_on_face(face: [i32; 3], value: i32) -> i32 {
    if face[0] < 0 || face[1] < 0 || face[2] < 0 {
        -value
    } else {
        value
    }
}

/// `Stairs.collectWorld`'s own sign: `-1` on the east and south faces, `+1`
/// elsewhere. It is the opposite of [`invert_on_face`], which is why the two
/// modes shift in opposite directions from the same click.
const fn stairs_sign(dir: [i32; 3]) -> i32 {
    if dir[0] > 0 || dir[2] > 0 { -1 } else { 1 }
}

/// Keep a candidate only where the world allows it.
fn push(world: &mut impl World, cells: &mut Vec<[i32; 3]>, pos: [i32; 3]) {
    if world.can_place(pos) {
        cells.push(pos);
    }
}

/// One click's inputs: what was clicked, from where, with which settings.
#[derive(Clone, Copy, Debug)]
pub struct BuildRequest {
    /// The block that was right-clicked.
    pub clicked: [i32; 3],
    /// Its clicked face normal.
    pub face: [i32; 3],
    /// The player's horizontal facing.
    pub facing: [i32; 3],
    /// The player's feet and eyes.
    pub player: PlayerView,
    /// Upstream's `PLACE_ON_TOP` toggle.
    pub place_on_top: bool,
    /// The tool range, 1 to [`MAX_RANGE`].
    pub range: u32,
}

/// The cells one use of the gadget places, in placement order. `mode` picks the
/// shape; invalid cells are skipped, not stopped at, as upstream does.
#[must_use]
pub fn plan(world: &mut impl World, mode: Mode, request: &BuildRequest) -> Vec<[i32; 3]> {
    if request.range == 0 {
        return Vec::new();
    }
    let mut cells = Vec::new();
    match mode {
        Mode::BuildToMe => build_to_me(world, &mut cells, request),
        Mode::VerticalWall => vertical_wall(world, &mut cells, request),
        Mode::VerticalColumn => vertical_column(world, &mut cells, request),
        Mode::Surface => surface(world, &mut cells, request),
        Mode::HorizontalWall => horizontal_wall(world, &mut cells, request),
        Mode::Stairs => stairs(world, &mut cells, request),
        Mode::HorizontalRow => horizontal_row(world, &mut cells, request),
        Mode::Grid => grid(world, &mut cells, request),
    }
    cells
}

/// `BuildToMe.collectWorld`: every cell between the clicked block and the player
/// along the clicked face's axis, the clicked block and the player's own cell
/// excluded.
fn build_to_me(world: &mut impl World, cells: &mut Vec<[i32; 3]>, request: &BuildRequest) {
    let axis = axis(request.face);
    let start = request.clicked[axis];
    let height = if request.place_on_top {
        request.player.eye_y
    } else {
        f64::from(request.player.block[1])
    };
    let player_coord = if axis == 1 {
        height as i32
    } else {
        request.player.block[axis]
    };
    let difference = (i64::from(start) - i64::from(player_coord)).abs().min(32) as i32;
    for i in 1..difference {
        push(world, cells, offset(request.clicked, request.face, i));
    }
}

/// `VerticalWall.collectWorld`: a wall spanning the face's perpendicular
/// horizontal axis and the vertical range, centred on the clicked block.
fn vertical_wall(world: &mut impl World, cells: &mut Vec<[i32; 3]>, request: &BuildRequest) {
    let clicked = request.clicked;
    let face = request.face;
    let size = (request.range / 2) as i32;
    let side = if face[1] != 0 {
        axis(request.facing)
    } else {
        axis(face)
    };
    let (start_y, end_y) = if face[1] != 0 {
        let direction = if face[1] < 0 { -1 } else { 1 };
        (
            clicked[1] + 1,
            clicked[1] + (request.range as i32 - 1) * direction + 1,
        )
    } else {
        (clicked[1] - size, clicked[1] + size)
    };
    let (start_x, end_x) = if side == 2 {
        (clicked[0] - size, clicked[0] + size)
    } else {
        (clicked[0], clicked[0])
    };
    let (start_z, end_z) = if side == 0 {
        (clicked[2] - size, clicked[2] + size)
    } else {
        (clicked[2], clicked[2])
    };
    for x in start_x..=end_x {
        for y in start_y..=end_y {
            for z in start_z..=end_z {
                push(world, cells, [x, y, z]);
            }
        }
    }
}

/// `VerticalColumn.collectWorld`: along the clicked face on a top or bottom face,
/// and up and down around the clicked block on a side face (the clicked cell
/// included, so a solid one is skipped by the planner's validity test).
fn vertical_column(world: &mut impl World, cells: &mut Vec<[i32; 3]>, request: &BuildRequest) {
    let clicked = request.clicked;
    if request.face[1] != 0 {
        for i in 1..=request.range as i32 {
            push(world, cells, offset(clicked, request.face, i));
        }
    } else {
        let half = (request.range / 2) as i32;
        for i in -half..=half {
            push(world, cells, offset(clicked, [0, 1, 0], i));
        }
    }
}

/// `Surface.collectWorld`: the square in the plane one block past the clicked
/// face, kept where the block behind each cell is the clicked block's state.
fn surface(world: &mut impl World, cells: &mut Vec<[i32; 3]>, request: &BuildRequest) {
    let face = request.face;
    let clicked = request.clicked;
    let bound = (request.range / 2) as i32;
    let start = offset(clicked, face, 1);
    let behind = [-face[0], -face[1], -face[2]];
    let (xs, ys, zs) = match axis(face) {
        1 => (
            start[0] - bound..=start[0] + bound,
            start[1]..=start[1],
            start[2] - bound..=start[2] + bound,
        ),
        2 => (
            start[0] - bound..=start[0] + bound,
            start[1] - bound..=start[1] + bound,
            start[2]..=start[2],
        ),
        _ => (
            start[0]..=start[0],
            start[1] - bound..=start[1] + bound,
            start[2] - bound..=start[2] + bound,
        ),
    };
    for x in xs {
        for y in ys.clone() {
            for z in zs.clone() {
                let pos = [x, y, z];
                if world.can_place(pos) && world.same_state(offset(pos, behind, 1), clicked) {
                    cells.push(pos);
                }
            }
        }
    }
}

/// `HorizontalWall.collectWorld`: a square on a top or bottom face, and on a side
/// face a wall along the face with half the range to each side. Upstream gates a
/// side-face column on its centre cell, so a blocked centre skips the whole column.
fn horizontal_wall(world: &mut impl World, cells: &mut Vec<[i32; 3]>, request: &BuildRequest) {
    let clicked = request.clicked;
    let face = request.face;
    let half = (request.range / 2) as i32;
    if face[1] != 0 {
        let y = clicked[1] + i32::from(request.place_on_top);
        for i in -half..=half {
            for j in -half..=half {
                push(world, cells, [clicked[0] - i, y, clicked[2] + j]);
            }
        }
        return;
    }
    let along_x = axis(face) == 0;
    for i in 1..=request.range as i32 {
        let value = invert_on_face(face, i);
        let centre = if along_x {
            [clicked[0] + value, clicked[1], clicked[2]]
        } else {
            [clicked[0], clicked[1], clicked[2] + value]
        };
        if !world.can_place(centre) {
            continue;
        }
        for j in -half..=half {
            let pos = if along_x {
                [clicked[0] + value, clicked[1], clicked[2] + j]
            } else {
                [clicked[0] + j, clicked[1], clicked[2] + value]
            };
            push(world, cells, pos);
        }
    }
}

/// `Stairs.collectWorld`: a staircase stepping away from the clicked face (or the
/// player's facing on a top or bottom face), climbing when the clicked block is
/// below the player and descending when it is above.
fn stairs(world: &mut impl World, cells: &mut Vec<[i32; 3]>, request: &BuildRequest) {
    let clicked = request.clicked;
    let face = request.face;
    let player = &request.player;
    let side = if face[1] != 0 {
        [-request.facing[0], 0, -request.facing[2]]
    } else {
        face
    };
    let on_top = i32::from(request.place_on_top);
    for i in 0..request.range as i32 {
        if face[1] == 0 && f64::from(clicked[1]) < player.y {
            let shift = (i + 1) * invert_on_face(face, 1);
            let pos = [
                clicked[0] + if face[0] != 0 { shift } else { 0 },
                clicked[1] + if request.place_on_top { -i } else { -(i + 1) },
                clicked[2] + if face[2] != 0 { shift } else { 0 },
            ];
            push(world, cells, pos);
            continue;
        }
        let mut shift = i * stairs_sign(side);
        if f64::from(clicked[1]) < player.y - 2.0 {
            shift = -shift;
        }
        let climb = if f64::from(clicked[1]) > player.y + 1.0 {
            -i
        } else {
            i
        };
        let pos = [
            clicked[0] + if side[0] != 0 { shift } else { 0 },
            clicked[1] + climb + on_top,
            clicked[2] + if side[2] != 0 { shift } else { 0 },
        ];
        push(world, cells, pos);
    }
}

/// `HorizontalRow.collectWorld`: a row starting at the clicked block (or above it
/// with `place_on_top`), running away from the player along the clicked face.
fn horizontal_row(world: &mut impl World, cells: &mut Vec<[i32; 3]>, request: &BuildRequest) {
    let side = row_side(request.face, request.facing);
    if side == [0, 0, 0] {
        return;
    }
    let start = if request.place_on_top {
        [
            request.clicked[0],
            request.clicked[1] + 1,
            request.clicked[2],
        ]
    } else {
        request.clicked
    };
    for step in 0..request.range as i32 {
        push(world, cells, offset(start, side, step));
    }
}

/// `Grid.collectWorld`: a lattice on the clicked block's horizontal plane, its
/// spacing from the range (two blocks plus one per two of range past the first).
fn grid(world: &mut impl World, cells: &mut Vec<[i32; 3]>, request: &BuildRequest) {
    let range = request.range as i32 + 1;
    let step = ((range - 2) % 6) + 2;
    let bound = range * 7 / 5;
    let start = if request.place_on_top {
        [
            request.clicked[0],
            request.clicked[1] + 1,
            request.clicked[2],
        ]
    } else {
        request.clicked
    };
    for x in -bound..=bound {
        for z in -bound..=bound {
            if x % step != 0 || z % step != 0 {
                continue;
            }
            push(world, cells, [start[0] + x, start[1], start[2] + z]);
        }
    }
}

/// A parsed `/buildinggadgets2` command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GadgetCommand {
    /// `/buildinggadgets2`, `range` or `mode` alone: report the settings.
    Status,
    /// `/buildinggadgets2 range <n>`: set the range.
    SetRange(u32),
    /// `/buildinggadgets2 mode <name>`: select the mode.
    SetMode(Mode),
    /// `/buildinggadgets2 placeontop <on|off>`: the `PLACE_ON_TOP` setting.
    SetPlaceOnTop(bool),
    /// A command in this namespace that the slice does not implement.
    Invalid,
}

/// Parse a command line without its leading slash. `None` when the line is
/// not ours; `Some(GadgetCommand::Invalid)` when it is ours but malformed.
#[must_use]
pub fn parse_command(line: &str) -> Option<GadgetCommand> {
    let line = line.trim().trim_start_matches('/');
    let mut words = line.split_whitespace();
    if words.next()? != "buildinggadgets2" {
        return None;
    }
    let Some(verb) = words.next() else {
        return Some(GadgetCommand::Status);
    };
    let Some(value) = words.next() else {
        return match verb {
            "range" | "mode" | "placeontop" => Some(GadgetCommand::Status),
            _ => Some(GadgetCommand::Invalid),
        };
    };
    if words.next().is_some() {
        return Some(GadgetCommand::Invalid);
    }
    match verb {
        "range" => match value.parse::<u32>() {
            Ok(range) if (1..=MAX_RANGE).contains(&range) => Some(GadgetCommand::SetRange(range)),
            _ => Some(GadgetCommand::Invalid),
        },
        "mode" => Mode::from_id(value).map_or(Some(GadgetCommand::Invalid), |mode| {
            Some(GadgetCommand::SetMode(mode))
        }),
        "placeontop" => match value {
            "on" | "true" => Some(GadgetCommand::SetPlaceOnTop(true)),
            "off" | "false" => Some(GadgetCommand::SetPlaceOnTop(false)),
            _ => Some(GadgetCommand::Invalid),
        },
        _ => Some(GadgetCommand::Invalid),
    }
}

/// The action-bar reply to a status request.
#[must_use]
pub fn status_reply(mode: Mode, range: u32, selected: Option<&str>, place_on_top: bool) -> String {
    let selected = selected.unwrap_or("none");
    let on_top = if place_on_top { "on" } else { "off" };
    format!(
        "Building Gadget: mode {}, range {range}, block {selected}, place-on-top {on_top}",
        mode.path()
    )
}

/// The action-bar reply after a range change.
#[must_use]
pub fn set_range_reply(range: u32) -> String {
    format!("Building Gadget range set to {range}")
}

/// The action-bar reply after a mode change.
#[must_use]
pub fn set_mode_reply(mode: Mode) -> String {
    format!("Building Gadget mode set to {}", mode.path())
}

/// The action-bar reply after the place-on-top setting changes.
#[must_use]
pub fn set_place_on_top_reply(enabled: bool) -> String {
    format!(
        "Building Gadget place-on-top {}",
        if enabled { "on" } else { "off" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestWorld {
        states: HashMap<[i32; 3], &'static str>,
    }

    impl TestWorld {
        fn new() -> Self {
            TestWorld {
                states: HashMap::new(),
            }
        }
        fn put(&mut self, pos: [i32; 3], state: &'static str) {
            self.states.insert(pos, state);
        }
        fn block(&mut self, pos: [i32; 3]) {
            self.put(pos, "minecraft:stone");
        }
    }

    impl World for TestWorld {
        fn can_place(&mut self, pos: [i32; 3]) -> bool {
            !self.states.contains_key(&pos)
        }
        fn same_state(&mut self, a: [i32; 3], b: [i32; 3]) -> bool {
            match (self.states.get(&a), self.states.get(&b)) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
    }

    /// The e2e's geometry: a stone block at `(0, 100, 0)` whose south face is
    /// clicked by a player standing at `(0, 100, 4)` and facing north.
    fn clicked() -> [i32; 3] {
        [0, 100, 0]
    }

    fn south() -> [i32; 3] {
        [0, 0, 1]
    }

    fn north() -> [i32; 3] {
        [0, 0, -1]
    }

    fn player() -> PlayerView {
        PlayerView {
            block: [0, 100, 4],
            y: 100.0,
            eye_y: 101.62,
        }
    }

    /// One click in the e2e's geometry: the block at `(0, 100, 0)`, the given
    /// face and player facing, the player at `(0, 100, 4)` facing north.
    fn request(face: [i32; 3], facing: [i32; 3], place_on_top: bool, range: u32) -> BuildRequest {
        BuildRequest {
            clicked: clicked(),
            face,
            facing,
            player: player(),
            place_on_top,
            range,
        }
    }

    /// One use of `mode` in the e2e's geometry, with the clicked block solid.
    fn mode_cells(mode: Mode, face: [i32; 3], place_on_top: bool, range: u32) -> Vec<[i32; 3]> {
        let mut world = TestWorld::new();
        world.block(clicked());
        plan(
            &mut world,
            mode,
            &request(face, north(), place_on_top, range),
        )
    }

    #[test]
    fn facing_snaps_to_the_four_horizontals() {
        assert_eq!(facing_from_yaw(0.0), [0, 0, 1], "south");
        assert_eq!(facing_from_yaw(90.0), [-1, 0, 0], "west");
        assert_eq!(facing_from_yaw(180.0), [0, 0, -1], "north");
        assert_eq!(facing_from_yaw(270.0), [1, 0, 0], "east");
        assert_eq!(facing_from_yaw(-90.0), [1, 0, 0], "east");
        assert_eq!(facing_from_yaw(359.0), [0, 0, 1], "wraps to south");
        assert_eq!(facing_from_yaw(45.0), [-1, 0, 0], "boundary to west");
    }

    #[test]
    fn a_side_face_builds_away_from_the_player() {
        // Click the south face of a block at z=0 while standing south of it:
        // the row runs north, through the clicked block and beyond.
        let mut world = TestWorld::new();
        world.block([0, 100, 0]);
        let cells = plan(
            &mut world,
            Mode::HorizontalRow,
            &request([0, 0, 1], [0, 0, 1], false, 6),
        );
        assert_eq!(
            cells,
            vec![
                [0, 100, -1],
                [0, 100, -2],
                [0, 100, -3],
                [0, 100, -4],
                [0, 100, -5]
            ]
        );
    }

    #[test]
    fn a_top_face_uses_the_player_facing() {
        let mut world = TestWorld::new();
        let cells = plan(
            &mut world,
            Mode::HorizontalRow,
            &request([0, 1, 0], [1, 0, 0], false, 4),
        );
        assert_eq!(
            cells,
            vec![[0, 100, 0], [1, 100, 0], [2, 100, 0], [3, 100, 0]]
        );
    }

    #[test]
    fn a_bottom_face_uses_the_player_facing() {
        let mut world = TestWorld::new();
        let cells = plan(
            &mut world,
            Mode::HorizontalRow,
            &request([0, -1, 0], [0, 0, -1], false, 3),
        );
        assert_eq!(cells, vec![[0, 100, 0], [0, 100, -1], [0, 100, -2]]);
    }

    #[test]
    fn place_on_top_starts_above_the_clicked_block() {
        let mut world = TestWorld::new();
        let cells = plan(
            &mut world,
            Mode::HorizontalRow,
            &request([0, 1, 0], [0, 0, 1], true, 3),
        );
        assert_eq!(cells, vec![[0, 101, 0], [0, 101, 1], [0, 101, 2]]);
    }

    #[test]
    fn the_range_caps_the_row() {
        let mut world = TestWorld::new();
        let mut row = |range| {
            plan(
                &mut world,
                Mode::HorizontalRow,
                &request([0, 0, 1], [0, 0, 1], false, range),
            )
        };
        assert_eq!(row(1).len(), 1);
        assert_eq!(row(15).len(), 15);
        assert!(row(0).is_empty());
    }

    #[test]
    fn an_invalid_cell_is_skipped_not_stopped_at() {
        let mut world = TestWorld::new();
        world.block([0, 100, -2]);
        let cells = plan(
            &mut world,
            Mode::HorizontalRow,
            &request([0, 0, 1], [0, 0, 1], false, 5),
        );
        assert_eq!(
            cells,
            vec![[0, 100, 0], [0, 100, -1], [0, 100, -3], [0, 100, -4]]
        );
    }

    #[test]
    fn the_default_range_is_one_and_the_slider_stops_at_fifteen() {
        assert_eq!(DEFAULT_RANGE, 1);
        assert_eq!(MAX_RANGE, 15);
    }

    /// Upstream's `GadgetNBT.getMode` returns `build_to_me` for a fresh gadget, and
    /// the id it stores is the full `buildinggadgets2:<path>`.
    #[test]
    fn a_fresh_gadget_is_build_to_me() {
        assert_eq!(DEFAULT_MODE, Mode::BuildToMe);
        assert_eq!(Mode::BuildToMe.id(), "buildinggadgets2:build_to_me");
    }

    #[test]
    fn mode_ids_round_trip_from_the_full_id_or_the_path() {
        for mode in Mode::ALL {
            assert_eq!(Mode::from_id(mode.id()), Some(mode), "{}", mode.id());
            assert_eq!(Mode::from_id(mode.path()), Some(mode), "{}", mode.path());
        }
        assert_eq!(Mode::from_id(" buildinggadgets2:grid "), Some(Mode::Grid));
        assert_eq!(Mode::from_id("nope"), None);
        assert_eq!(Mode::from_id("buildinggadgets2:"), None);
        assert_eq!(Mode::ALL.len(), 8);
    }

    /// `BuildToMe.collectWorld`: the cells strictly between the clicked block and
    /// the player, along the clicked face's axis.
    #[test]
    fn build_to_me_fills_between_the_player_and_the_clicked_block() {
        assert_eq!(
            mode_cells(Mode::BuildToMe, south(), false, 5),
            vec![[0, 100, 1], [0, 100, 2], [0, 100, 3]]
        );
    }

    /// `VerticalColumn.collectWorld` on a side face: half the range up and down
    /// from the clicked block, the clicked cell included (and skipped when solid).
    #[test]
    fn vertical_column_on_a_side_face_centres_on_the_clicked_block() {
        assert_eq!(
            mode_cells(Mode::VerticalColumn, south(), false, 5),
            vec![[0, 98, 0], [0, 99, 0], [0, 101, 0], [0, 102, 0]]
        );
    }

    /// On a top or bottom face it runs along the face instead.
    #[test]
    fn vertical_column_on_a_top_face_runs_along_the_clicked_face() {
        let mut world = TestWorld::new();
        assert_eq!(
            plan(
                &mut world,
                Mode::VerticalColumn,
                &request([0, 1, 0], north(), false, 3)
            ),
            vec![[0, 101, 0], [0, 102, 0], [0, 103, 0]]
        );
    }

    /// `VerticalWall.collectWorld`: a wall across the face's perpendicular
    /// horizontal axis and the vertical range.
    #[test]
    fn vertical_wall_stands_across_the_clicked_face() {
        let cells = mode_cells(Mode::VerticalWall, south(), false, 5);
        assert_eq!(cells.len(), 24, "25 cells minus the clicked block");
        assert_eq!(cells[0], [-2, 98, 0]);
        assert!(
            !cells.contains(&[0, 100, 0]),
            "the clicked block is skipped"
        );
        assert_eq!(cells.last(), Some(&[2, 102, 0]));
    }

    /// `Surface.collectWorld`: only the cells whose block behind them is the
    /// clicked block's state survive.
    #[test]
    fn surface_copies_only_the_cells_backed_by_the_clicked_block() {
        assert_eq!(
            mode_cells(Mode::Surface, south(), false, 5),
            vec![[0, 100, 1]]
        );
    }

    /// `HorizontalWall.collectWorld` on a side face: the range along the face,
    /// half the range to each side.
    #[test]
    fn horizontal_wall_on_a_side_face_runs_along_the_face() {
        let cells = mode_cells(Mode::HorizontalWall, south(), false, 5);
        assert_eq!(cells.len(), 25);
        assert_eq!(cells[0], [-2, 100, 1]);
        assert_eq!(cells.last(), Some(&[2, 100, 5]));
    }

    /// Upstream gates a side-face column on its centre cell: a blocked centre
    /// skips the whole column, which is what its two validity checks amount to.
    #[test]
    fn horizontal_wall_skips_a_column_whose_centre_is_blocked() {
        let mut world = TestWorld::new();
        world.block(clicked());
        world.block([0, 100, 2]);
        let cells = plan(
            &mut world,
            Mode::HorizontalWall,
            &request(south(), north(), false, 5),
        );
        assert_eq!(cells.len(), 20, "the z=2 column is skipped");
        assert!(cells.iter().all(|pos| pos[2] != 2));
    }

    /// On a top face it squares around the clicked block, one block up with the
    /// toggle on.
    #[test]
    fn horizontal_wall_on_a_top_face_squares_around_the_click() {
        let mut world = TestWorld::new();
        world.block(clicked());
        let flat = plan(
            &mut world,
            Mode::HorizontalWall,
            &request([0, 1, 0], north(), false, 5),
        );
        assert_eq!(flat.len(), 24, "25 cells minus the clicked block");
        let raised = plan(
            &mut world,
            Mode::HorizontalWall,
            &request([0, 1, 0], north(), true, 5),
        );
        assert_eq!(raised.len(), 25);
        assert!(raised.iter().all(|pos| pos[1] == 101));
    }

    /// `Stairs.collectWorld`: steps up and away from the player, the clicked
    /// block itself skipped as solid.
    #[test]
    fn stairs_climb_away_from_the_player() {
        assert_eq!(
            mode_cells(Mode::Stairs, south(), false, 5),
            vec![[0, 101, -1], [0, 102, -2], [0, 103, -3], [0, 104, -4]]
        );
    }

    /// `Grid.collectWorld`: a lattice on the clicked block's plane, its spacing
    /// two blocks plus one per two of range past the first.
    #[test]
    fn grid_lays_a_lattice_on_the_clicked_plane() {
        assert_eq!(
            mode_cells(Mode::Grid, south(), false, 5),
            vec![
                [-6, 100, -6],
                [-6, 100, 0],
                [-6, 100, 6],
                [0, 100, -6],
                [0, 100, 6],
                [6, 100, -6],
                [6, 100, 0],
                [6, 100, 6]
            ]
        );
    }

    /// `place_on_top` is the setting `Grid`, `Stairs`, `HorizontalWall` and
    /// `BuildToMe` read; the grid moves one block up with it.
    #[test]
    fn place_on_top_moves_the_grid_up_one_block() {
        let cells = mode_cells(Mode::Grid, south(), true, 5);
        assert_eq!(cells.len(), 9);
        assert!(cells.iter().all(|pos| pos[1] == 101));
        assert!(cells.contains(&[0, 101, 0]), "above the clicked block");
    }

    #[test]
    fn commands_are_parsed_in_the_mod_namespace() {
        assert_eq!(
            parse_command("buildinggadgets2"),
            Some(GadgetCommand::Status)
        );
        assert_eq!(
            parse_command("/buildinggadgets2 range"),
            Some(GadgetCommand::Status)
        );
        assert_eq!(
            parse_command("buildinggadgets2 range 6"),
            Some(GadgetCommand::SetRange(6))
        );
        assert_eq!(
            parse_command("  /buildinggadgets2   range   15 "),
            Some(GadgetCommand::SetRange(15))
        );
        assert_eq!(
            parse_command("buildinggadgets2 range 0"),
            Some(GadgetCommand::Invalid)
        );
        assert_eq!(
            parse_command("buildinggadgets2 range 16"),
            Some(GadgetCommand::Invalid)
        );
        assert_eq!(
            parse_command("buildinggadgets2 range many"),
            Some(GadgetCommand::Invalid)
        );
        assert_eq!(
            parse_command("buildinggadgets2 range 2 extra"),
            Some(GadgetCommand::Invalid)
        );
        assert_eq!(parse_command("ultimine shape"), None);
        assert_eq!(parse_command(""), None);
    }

    #[test]
    fn commands_select_the_mode_and_the_place_on_top_setting() {
        assert_eq!(
            parse_command("buildinggadgets2 mode stairs"),
            Some(GadgetCommand::SetMode(Mode::Stairs))
        );
        assert_eq!(
            parse_command("buildinggadgets2 mode buildinggadgets2:vertical_wall"),
            Some(GadgetCommand::SetMode(Mode::VerticalWall))
        );
        assert_eq!(
            parse_command("buildinggadgets2 mode build_to_me"),
            Some(GadgetCommand::SetMode(Mode::BuildToMe))
        );
        assert_eq!(
            parse_command("buildinggadgets2 mode nope"),
            Some(GadgetCommand::Invalid)
        );
        assert_eq!(
            parse_command("buildinggadgets2 mode"),
            Some(GadgetCommand::Status)
        );
        assert_eq!(
            parse_command("buildinggadgets2 placeontop on"),
            Some(GadgetCommand::SetPlaceOnTop(true))
        );
        assert_eq!(
            parse_command("buildinggadgets2 placeontop false"),
            Some(GadgetCommand::SetPlaceOnTop(false))
        );
        assert_eq!(
            parse_command("buildinggadgets2 placeontop maybe"),
            Some(GadgetCommand::Invalid)
        );
        assert_eq!(
            parse_command("buildinggadgets2 placeontop"),
            Some(GadgetCommand::Status)
        );
    }

    #[test]
    fn wheel_steps_clamp_to_the_slider() {
        assert_eq!(scroll_range(6, 1), 7);
        assert_eq!(scroll_range(6, -1), 5);
        assert_eq!(scroll_range(1, -1), 1);
        assert_eq!(scroll_range(15, 1), 15);
        assert_eq!(scroll_range(1, 0), 1);
    }

    #[test]
    fn replies_name_the_mode_range_and_selection() {
        assert_eq!(
            status_reply(Mode::Stairs, 5, Some("minecraft:stone"), false),
            "Building Gadget: mode stairs, range 5, block minecraft:stone, place-on-top off"
        );
        assert_eq!(
            status_reply(Mode::BuildToMe, 1, None, true),
            "Building Gadget: mode build_to_me, range 1, block none, place-on-top on"
        );
        assert_eq!(set_range_reply(5), "Building Gadget range set to 5");
        assert_eq!(
            set_mode_reply(Mode::VerticalWall),
            "Building Gadget mode set to vertical_wall"
        );
        assert_eq!(
            set_place_on_top_reply(true),
            "Building Gadget place-on-top on"
        );
    }

    fn rights() -> EditRights {
        EditRights {
            may_build: true,
            may_use_at: true,
            border: None,
            dedicated: false,
            op: false,
            spawn_protection: 0,
            spawn: Some([0, 64, 0]),
            in_overworld: true,
        }
    }

    /// The control case: a survival or creative player outside every protection may build.
    #[test]
    fn an_unprotected_row_cell_is_buildable() {
        assert!(may_edit([1000, 100, 1000], &rights()));
    }

    /// A spectator's or an adventure player's gadget must not place: the actor-free write
    /// never asks the game, so the player's build permission is checked here.
    #[test]
    fn a_player_who_may_not_build_is_refused() {
        let mut spectator = rights();
        spectator.may_build = false;
        assert!(!may_edit([1000, 100, 1000], &spectator));

        let mut adventure = rights();
        adventure.may_use_at = false;
        assert!(!may_edit([1000, 100, 1000], &adventure));
    }

    /// A row cell outside the world border is refused, as `Level.mayInteract` refuses it: the
    /// border is checked per destination, so a row that starts inside and runs out is cut.
    #[test]
    fn a_cell_outside_the_border_is_refused() {
        let mut bordered = rights();
        bordered.border = Some([-100.0, -100.0, 100.0, 100.0]);
        assert!(may_edit([0, 100, 0], &bordered));
        assert!(may_edit([100, 100, 100], &bordered), "on the edge");
        assert!(!may_edit([101, 100, 0], &bordered), "one block past x");
        assert!(!may_edit([0, 100, -101], &bordered), "one block past z");
    }

    /// The review finding: a row may not be placed across a dedicated server's spawn-protection
    /// boundary, even though the clicked block outside it was permitted. An operator is exempt,
    /// the radius is the server's own, and only the Overworld is covered.
    #[test]
    fn a_dedicated_servers_spawn_protection_is_enforced_per_cell() {
        let mut server = rights();
        server.dedicated = true;
        server.spawn_protection = 16;
        assert!(!may_edit([4, 100, 4], &server), "inside the radius");
        assert!(
            may_edit([17, 100, 4], &server),
            "one block outside the radius"
        );
        assert!(may_edit([4, 100, 17], &server), "z outside the radius");

        let mut op = server;
        op.op = true;
        assert!(may_edit([4, 100, 4], &op), "an operator is exempt");

        let mut off = server;
        off.spawn_protection = 0;
        assert!(may_edit([4, 100, 4], &off), "spawn protection off");

        let mut nether = server;
        nether.in_overworld = false;
        assert!(may_edit([4, 100, 4], &nether), "another dimension");

        let mut integrated = server;
        integrated.dedicated = false;
        assert!(
            may_edit([4, 100, 4], &integrated),
            "an integrated server never applies spawn protection"
        );
    }

    /// The radius comes from the server's own file; a missing or malformed key is vanilla's
    /// default, never a silent zero.
    #[test]
    fn the_spawn_protection_radius_is_the_servers_own() {
        assert_eq!(
            spawn_protection_from("spawn-protection=8\nmax-players=20\n"),
            8
        );
        assert_eq!(spawn_protection_from("spawn-protection=0\n"), 0);
        assert_eq!(
            spawn_protection_from("spawn-protection=abc\n"),
            VANILLA_SPAWN_PROTECTION
        );
        assert_eq!(
            spawn_protection_from("max-players=20\n"),
            VANILLA_SPAWN_PROTECTION
        );
        assert_eq!(spawn_protection_from("  spawn-protection = 4  \n"), 4);
    }
}
