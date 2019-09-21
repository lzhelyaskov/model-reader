extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

use super::{to_utf8, vec3_t, Error, Result};

pub const HEADER_IDENT: i32 = 1330660425;
pub const HEADER_VERSION: i32 = 6;

#[repr(C)]
#[derive(Debug)]
pub struct Header {
    pub ident: i32,   // must be equal to 1330660425 or to the string “IDPO”
    pub version: i32, // 6
    pub scale: vec3_t,
    pub translate: vec3_t,
    pub boundigradius: f32,
    pub eyeposition: vec3_t,

    pub num_skins: i32,
    pub skinwidth: i32,
    pub skinheight: i32,

    pub num_verices: i32,
    pub num_tris: i32,
    pub num_frames: i32,

    pub synctype: i32,
    pub flags: i32,
    pub size: f32,
}

pub struct Skin {
    pub group: i32, // 0
    pub data: Vec<u8>,
}

#[allow(dead_code)]
pub struct GroupSkin {
    pub group: i32, // 1
    pub nb: i32,
    pub time: Vec<f32>,
    pub data: Vec<u8>,
}

pub struct TexCoord {
    pub onseam: i32,
    pub s: i32,
    pub t: i32,
}

pub struct Triangle {
    pub facefront: i32, // 0-backface. 0<>frontface
    pub vertex: [i32; 3],
}

pub struct Vertex {
    pub v: [u8; 3],
    pub normal_index: u8,
}

pub struct SimpleFrame {
    pub bboxmin: Vertex,
    pub bboxmax: Vertex,
    pub name: String,
    pub verts: Vec<Vertex>,
}

pub struct Frame {
    pub type_: i32, // if 0
    pub frame: SimpleFrame,
}

#[allow(dead_code)]
pub struct GroupFrame {
    pub type_: i32, // if !0
    pub min: Vertex,
    pub max: Vertex,
    pub time: Vec<f32>,
    pub frames: Vec<SimpleFrame>,
}

pub struct Model {
    pub header: Header,
    pub skins: Vec<Skin>,
    pub texcoords: Vec<TexCoord>,
    pub triangles: Vec<Triangle>,
    pub frames: Vec<Frame>,
}

impl Model {
    fn read_header(reader: &mut dyn Read) -> Result<Header> {
        let header = {
            let mut buf = [0; std::mem::size_of::<Header>()];
            reader
                .read_exact(&mut buf)
                .map_err(|e| Error::io(e, "failed to read header"))?;
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

    fn read_skins(reader: &mut dyn Read, header: &Header) -> Result<Vec<Skin>> {
        let mut skins = Vec::<Skin>::new();
        let skin_width_x_height = (header.skinwidth * header.skinheight) as usize;
        for _ in 0..header.num_skins {
            let mut data = Vec::with_capacity(skin_width_x_height);
            unsafe {
                data.set_len(skin_width_x_height);
            }
            let group = reader
                .read_i32::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read skin group"))?;

            if group != 0 {
                return Err(Error::unsupported("skin groups are not supported."));
            }

            reader
                .read_exact(&mut data)
                .map_err(|e| Error::io(e, "failed to read skin data"))?;

            let skin = Skin {
                data: data,
                group: group,
            };
            skins.push(skin);
        }
        Ok(skins)
    }

    fn read_texcoords(reader: &mut dyn Read, header: &Header) -> Result<Vec<TexCoord>> {
        let mut texcoords = Vec::<TexCoord>::with_capacity(header.num_verices as usize);
        for _ in 0..header.num_verices {
            let onseam = reader
                .read_i32::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read texcoord"))?;
            let s = reader
                .read_i32::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read texcoord"))?;
            let t = reader
                .read_i32::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read texcoord"))?;
            let texcoord = TexCoord {
                onseam: onseam,
                s: s,
                t: t,
            };
            texcoords.push(texcoord);
        }
        Ok(texcoords)
    }

    fn read_triangles(reader: &mut dyn Read, header: &Header) -> Result<Vec<Triangle>> {
        let mut triangles = Vec::<Triangle>::with_capacity(header.num_tris as usize);
        for _ in 0..header.num_tris {
            let facefront = reader
                .read_i32::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read triangle"))?;
            let a = reader
                .read_i32::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read triangle"))?;
            let b = reader
                .read_i32::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read triangle"))?;
            let c = reader
                .read_i32::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read triangle"))?;
            let triangle = Triangle {
                facefront: facefront,
                vertex: [a, b, c],
            };
            triangles.push(triangle);
        }
        Ok(triangles)
    }

    fn read_frames(reader: &mut dyn Read, header: &Header) -> Result<Vec<Frame>> {
        let mut frames = Vec::<Frame>::with_capacity(header.num_frames as usize);
        let mut buf: [u8; 16] = [0; 16];
        for _ in 0..header.num_frames {
            let type_ = reader
                .read_i32::<LittleEndian>()
                .map_err(|e| Error::io(e, "failed to read frame type"))?;

            if type_ != 0 {
                return Err(Error::unsupported("group frames are not supported."));
            }
            let bboxmin = {
                let mut v: [u8; 3] = [0; 3];
                reader
                    .read_exact(&mut v)
                    .map_err(|e| Error::io(e, "failed to read bbox min"))?;
                let normal_index = reader
                    .read_u8()
                    .map_err(|e| Error::io(e, "failed to read bbox min"))?;
                Vertex {
                    v: v,
                    normal_index: normal_index,
                }
            };

            let bboxmax = {
                let mut v: [u8; 3] = [0; 3];
                reader
                    .read_exact(&mut v)
                    .map_err(|e| Error::io(e, "failed to read bbox max"))?;
                let normal_index = reader
                    .read_u8()
                    .map_err(|e| Error::io(e, "failed to read bbox max"))?;
                Vertex {
                    v: v,
                    normal_index: normal_index,
                }
            };

            reader
                .read_exact(&mut buf)
                .map_err(|e| Error::io(e, "failed to read frame name."))?;
            let name = to_utf8(&buf)
                .map_err(|e| Error::utf8(e, "failed to covert frame name to utf8."))?;

            let mut verts = Vec::<Vertex>::with_capacity(header.num_verices as usize);
            for _ in 0..header.num_verices {
                let mut v: [u8; 3] = [0; 3];
                reader
                    .read_exact(&mut v)
                    .map_err(|e| Error::io(e, "failed to read vertex"))?;
                let normal_index = reader
                    .read_u8()
                    .map_err(|e| Error::io(e, "failed to read vertex"))?;
                let vertex = Vertex {
                    v: v,
                    normal_index: normal_index,
                };
                verts.push(vertex);
            }
            let simple_frame = SimpleFrame {
                bboxmin: bboxmin,
                bboxmax: bboxmax,
                name: name,
                verts: verts,
            };
            let frame = Frame {
                type_: type_,
                frame: simple_frame,
            };
            frames.push(frame);
        }
        Ok(frames)
    }
    pub fn from_reader(reader: &mut dyn Read) -> Result<Self> {
        let header = Self::read_header(reader)?;

        let skins = Self::read_skins(reader, &header)?;
        let texcoords = Self::read_texcoords(reader, &header)?;
        let triangles = Self::read_triangles(reader, &header)?;
        let frames = Self::read_frames(reader, &header)?;

        Ok(Model {
            header: header,
            skins: skins,
            texcoords: texcoords,
            triangles: triangles,
            frames: frames,
        })
    }
}
