extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

use super::{to_utf8, vec3_t, Error, Result};
use std::io::{Read, Seek, SeekFrom};

pub const MAX_TRIANGLES: u16 = 4096;
pub const MAX_VERTICES: u16 = 2048;
pub const MAX_TEXCOORDS: u16 = 2048;
pub const MAX_FRAMES: u16 = 512;
pub const MAX_SKINS: u16 = 32;

pub const ANIMATIONS: [[u8; 3]; 21] = [
    // first, last, fps
    [0, 39, 9],     // STAND
    [40, 45, 10],   // RUN
    [46, 53, 10],   // ATTACK
    [54, 57, 7],    // PAIN_A
    [58, 61, 7],    // PAIN_B
    [62, 65, 7],    // PAIN_C
    [66, 71, 7],    // JUMP
    [72, 83, 7],    // FLIP
    [84, 94, 7],    // SALUTE
    [95, 111, 10],  // FALLBACK
    [112, 122, 7],  // WAVE
    [123, 134, 6],  // POINT
    [135, 153, 10], // CROUCH_STAND
    [154, 159, 7],  // CROUCH_WALK
    [160, 168, 10], // CROUCH_ATTACK
    [196, 172, 7],  // CROUCH_PAIN
    [173, 177, 5],  // CROUCH_DEATH
    [178, 183, 7],  // DEATH_FALLBACK
    [184, 189, 7],  // DEATH_FALLFORWARD
    [190, 197, 7],  // DEATH_FALLBACKSLOW
    [198, 198, 5],  // BOOM
];

#[allow(non_camel_case_types)]
pub enum Animation {
    STAND = 0,
    RUN,
    ATTACK,
    PAIN_A,
    PAIN_B,
    PAIN_C,
    JUMP,
    FLIP,
    SALUTE,
    FALLBACK,
    WAVE,
    POINT,
    CROUCH_STAND,
    CROUCH_WALK,
    CROUCH_ATTACK,
    CROUCH_PAIN,
    CROUCH_DEATH,
    DEATH_FALLBACK,
    DEATH_FALLFORWARD,
    DEATH_FALLBACKSLOW,
    BOOM,

    MAX_ANIMATIONS = 198,
}

#[allow(non_camel_case_types)]
type skin_name_t = [u8; 64];

#[derive(PartialEq, Debug)]
pub enum CommandType {
    Fan,
    Strip,
}

pub struct CommandPacket {
    pub s: f32,
    pub t: f32,
    pub i: i32,
}

pub struct Command {
    pub typ: CommandType,
    pub packets: Vec<CommandPacket>,
}

#[derive(Debug)]
enum NextCommand {
    Typ,
    S(CommandType, u32),
    T(CommandType, u32, f32),
    I(CommandType, u32, f32, f32),
}

pub const HEADER_IDENT: i32 = 844121161;
pub const HEADER_VERSION: i32 = 8;

#[repr(C)]
#[derive(Debug)]
pub struct Header {
    pub ident: i32,   // IDP2 / 844121161
    pub version: i32, // 8

    pub skin_width: i32,
    pub skin_height: i32,

    pub frame_size: i32,
    pub num_skins: i32,
    pub num_vertices: i32,
    pub num_texcoords: i32,
    pub num_faces: i32,
    pub num_gl_cmds: i32,
    pub num_frames: i32,

    pub offset_skins: i32,
    pub offset_texcoords: i32,
    pub offset_faces: i32,
    pub offset_frames: i32,
    pub offset_gl_cmds: i32,
    pub offset_end: i32,
}

pub struct TexCoord {
    pub s: i16,
    pub t: i16,
}

pub struct Triangle {
    pub vertex: [u16; 3],
    pub st_idx: [u16; 3],
}

pub struct Vertex {
    pub v: [u8; 3],
    pub normal_idx: u8,
}

pub struct Frame {
    pub scale: vec3_t,
    pub translate: vec3_t,
    pub name: String,
    pub vertices: Vec<Vertex>,
}

pub struct Model {
    pub header: Header,
    pub skin_names: Vec<String>,
    pub texcoords: Vec<TexCoord>,
    pub faces: Vec<Triangle>,
    pub frames: Vec<Frame>,
    pub commands: Vec<Command>,
}

impl Model {
    fn read_header(reader: &mut dyn Read) -> Result<Header> {
        let header = {
            let mut buf = [0; std::mem::size_of::<Header>()];
            if let Err(e) = reader.read_exact(&mut buf) {
                return Err(Error::io(e, "failed to read header"));
            };
            let header: Header = unsafe { std::mem::transmute(buf) };
            header
        };

        if header.ident != HEADER_IDENT {
            return Err(Error::ident(header.ident, HEADER_IDENT));
        }

        if header.version != HEADER_VERSION {
            return Err(Error::version(header.version, HEADER_VERSION));
        }

        Ok(header)
    }

    fn read_skin_names<T: Read + Seek>(reader: &mut T, header: &Header) -> Result<Vec<String>> {
        let mut skin_names = Vec::<String>::new();
        reader
            .seek(SeekFrom::Start(header.offset_skins as u64))
            .map_err(|e| Error::io(e, "offset_skins failed."))?;
        let mut buf: skin_name_t = [0; 64];
        for _ in 0..header.num_skins {
            reader
                .read_exact(&mut buf)
                .map_err(|e| Error::io(e, "skin_name: read_exact failed."))?;
            let name =
                to_utf8(&buf).map_err(|e| Error::utf8(e, "failed to convert skin name to utf8"))?;
            skin_names.push(name);
        }

        Ok(skin_names)
    }

    fn read_texcoords<T: Read + Seek>(reader: &mut T, header: &Header) -> Result<Vec<TexCoord>> {
        let mut texcoords = Vec::<TexCoord>::new();
        reader
            .seek(SeekFrom::Start(header.offset_texcoords as u64))
            .map_err(|e| Error::io(e, "offset_texcoords failed."))?;
        for _ in 0..header.num_texcoords {
            let s: i16 = reader
                .read_i16::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read 's'."))?;
            let t: i16 = reader
                .read_i16::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read 't'."))?;

            let st = TexCoord { s: s, t: t };
            texcoords.push(st);
        }

        Ok(texcoords)
    }

    fn read_faces<T: Read + Seek>(reader: &mut T, header: &Header) -> Result<Vec<Triangle>> {
        let mut faces = Vec::<Triangle>::new();
        reader
            .seek(SeekFrom::Start(header.offset_faces as u64))
            .map_err(|e| Error::io(e, "offset_faces failed."))?;

        for _ in 0..header.num_faces {
            let x = reader
                .read_u16::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read 'x'."))?;
            let y = reader
                .read_u16::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read 'y'."))?;
            let z = reader
                .read_u16::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read 'z'."))?;
            let i = reader
                .read_u16::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read 'i'."))?;
            let j = reader
                .read_u16::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read 'j'."))?;
            let k = reader
                .read_u16::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read 'k'."))?;

            let triangle = Triangle {
                vertex: [x, y, z],
                st_idx: [i, j, k],
            };
            faces.push(triangle);
        }

        Ok(faces)
    }

    fn read_commands<T: Read + Seek>(reader: &mut T, header: &Header) -> Result<Vec<Command>> {
        let mut commands = Vec::<Command>::new();
        reader
            .seek(SeekFrom::Start(header.offset_gl_cmds as u64))
            .map_err(|e| Error::io(e, "offset_gl_cmds failed."))?;
        let mut state = NextCommand::Typ;
        let mut packets = Vec::new();
        for _ in 0..header.num_gl_cmds {
            match state {
                NextCommand::Typ => {
                    let n = reader
                        .read_i32::<LittleEndian>()
                        .map_err(|e| Error::io(e, "failed to read 'n'."))?;
                    if n == 0 {
                        break;
                    }
                    state = if n > 0 {
                        NextCommand::S(CommandType::Fan, n.abs() as u32)
                    } else {
                        NextCommand::S(CommandType::Strip, n.abs() as u32)
                    };
                }
                NextCommand::S(typ, n) => {
                    let s = reader
                        .read_f32::<LittleEndian>()
                        .map_err(|e| Error::io(e, "failed to read 's'."))?;
                    state = NextCommand::T(typ, n, s);
                }
                NextCommand::T(typ, n, s) => {
                    let t = reader
                        .read_f32::<LittleEndian>()
                        .map_err(|e| Error::io(e, "failed to read 't'."))?;
                    state = NextCommand::I(typ, n, s, t);
                }
                NextCommand::I(typ, n, s, t) => {
                    let i = reader
                        .read_i32::<LittleEndian>()
                        .map_err(|e| Error::io(e, "failed to read 'i'."))?;
                    let cmd = CommandPacket { s: s, t: t, i: i };
                    packets.push(cmd);

                    state = if n - 1 == 0 {
                        let command = Command {
                            typ: typ,
                            packets: std::mem::replace(&mut packets, Vec::<CommandPacket>::new()),
                        };
                        commands.push(command);
                        NextCommand::Typ
                    } else {
                        NextCommand::S(typ, n - 1)
                    };
                }
            }
        }

        Ok(commands)
    }

    fn read_frames<T: Read + Seek>(reader: &mut T, header: &Header) -> Result<Vec<Frame>> {
        let mut frames = Vec::<Frame>::new();
        reader
            .seek(SeekFrom::Start(header.offset_frames as u64))
            .map_err(|e| Error::io(e, "offset_frames failed."))?;
        let mut buf = [0; 16];
        for _ in 0..header.num_frames {
            let scale = {
                let x = reader
                    .read_f32::<LittleEndian>()
                    .map_err(|e| Error::io(e, "failed to read 'scale x'."))?;
                let y = reader
                    .read_f32::<LittleEndian>()
                    .map_err(|e| Error::io(e, "failed to read 'scale y'."))?;
                let z = reader
                    .read_f32::<LittleEndian>()
                    .map_err(|e| Error::io(e, "failed to read 'scale z'."))?;
                [x, y, z]
            };
            let translate = {
                let x = reader
                    .read_f32::<LittleEndian>()
                    .map_err(|e| Error::io(e, "failed to read 'translate x'."))?;
                let y = reader
                    .read_f32::<LittleEndian>()
                    .map_err(|e| Error::io(e, "failed to read 'translate y'."))?;
                let z = reader
                    .read_f32::<LittleEndian>()
                    .map_err(|e| Error::io(e, "failed to read 'translate z'."))?;
                [x, y, z]
            };

            reader
                .read_exact(&mut buf)
                .map_err(|e| Error::io(e, "failed to read 'frame name'."))?;
            let name = to_utf8(&buf)
                .map_err(|e| Error::utf8(e, "failed to convert frame name to utf8"))?;
            let mut vertices = Vec::<Vertex>::with_capacity(header.num_vertices as usize);
            for _ in 0..header.num_vertices {
                let v = {
                    let x = reader
                        .read_u8()
                        .map_err(|e| Error::io(e, "failed to read 'vec x'."))?;
                    let y = reader
                        .read_u8()
                        .map_err(|e| Error::io(e, "failed to read 'vec y'."))?;
                    let z = reader
                        .read_u8()
                        .map_err(|e| Error::io(e, "failed to read 'vec z'."))?;
                    [x, y, z]
                };

                let normal_idx = reader
                    .read_u8()
                    .map_err(|e| Error::io(e, "failed to read 'vec normal_idx'."))?;

                let vertex = Vertex {
                    v: v,
                    normal_idx: normal_idx,
                };
                vertices.push(vertex);
            }

            let frame = Frame {
                scale: scale,
                translate: translate,
                name: name,
                vertices: vertices,
            };
            frames.push(frame);
        }

        Ok(frames)
    }

    pub fn from_reader<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let header = Self::read_header(reader)?;
        let skin_names = Self::read_skin_names(reader, &header)?;
        let texcoords = Self::read_texcoords(reader, &header)?;
        let faces = Self::read_faces(reader, &header)?;
        let commands = Self::read_commands(reader, &header)?;
        let frames = Self::read_frames(reader, &header)?;

        Ok(Model {
            header: header,
            skin_names: skin_names,
            texcoords: texcoords,
            faces: faces,
            frames: frames,
            commands: commands,
        })
    }    
}
