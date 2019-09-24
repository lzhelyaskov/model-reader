extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Write};

use super::{to_utf8, vec3_t, Error, Result};

pub const MAX_TRIANGLES: u16 = 2048;
pub const MAX_VERTICES: u16 = 1024;
pub const MAX_TEXCOORDS: u16 = 1024;
pub const MAX_FRAMES: u16 = 256;

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
    pub skin_width: i32,
    pub skin_height: i32,

    pub num_verices: i32,
    pub num_faces: i32,
    pub num_frames: i32,

    pub synctype: i32, // 0 synchron. 1 random
    pub flags: i32,
    pub size: f32,
}

/// basicaly a bitmap
/// width and height are stored in header
/// each item of data vector is an index to
/// color map super::COLORMAP
pub struct Skin {
    pub group: i32, // 0
    pub data: Vec<u8>,
}

// TODO: implement this
#[allow(dead_code)]
pub struct GroupSkin {
    pub group: i32, // 1
    pub nb: i32,
    pub time: Vec<f32>,
    pub data: Vec<u8>, // nb * skin_width * skin_height
}

/// onseam > 0 means the coordinate is on the edge
/// between front and back parts of the texture
/// if the triangle is on the back (facefront = 0)
/// half of thr texture width must be added to 's' value
pub struct TexCoord {
    pub onseam: i32,
    pub s: i32,
    pub t: i32,
}

pub struct Triangle {
    pub facefront: i32,   // 0-backface. 0<>frontface
    pub vertex: [i32; 3], // index to SimpleFrame::verts
}

pub struct Vertex {
    pub v: [u8; 3], // to uncompress: real[i] = (scale[i] * vertex[i]) + translate[i];
    pub normal_idx: u8, // index to super::NORMALS
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

// TODO: implement this
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
        let skin_width_x_height = (header.skin_width * header.skin_height) as usize;
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
        let mut triangles = Vec::<Triangle>::with_capacity(header.num_faces as usize);
        for _ in 0..header.num_faces {
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
                    normal_idx: normal_index,
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
                    normal_idx: normal_index,
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
                    normal_idx: normal_index,
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

    /// writes model as json to writer
    /// back and front faces (trinagles) are written in separate vecs
    pub fn write_json(&self, writer: &mut dyn Write) -> std::result::Result<(), std::io::Error> {
        write!(writer, "{{")?;

        self.write_frames(writer)?; // and normals(?)
        self.write_triangles(writer)?; // and texcoords

        write!(writer, "}}")?;
        Ok(())
    }

    fn write_frames(&self, writer: &mut dyn Write) -> std::result::Result<(), std::io::Error> {
        if self.frames.is_empty() {
            return Ok(());
        }

        write!(writer, "\n\t\"frames\": [")?;

        self.write_frame(writer, 0)?;
        for idx in 1..self.frames.len() {
            write!(writer, ",")?;
            self.write_frame(writer, idx)?;
        }
        write!(writer, "\t],")?;
        Ok(())
    }

    fn write_frame(
        &self,
        writer: &mut dyn Write,
        idx: usize,
    ) -> std::result::Result<(), std::io::Error> {
        let frame = &self.frames[idx];
        let scale = self.header.scale;
        let translate = self.header.translate;
        
        write!(
            writer,
            "{{\n\t\t\"name\": \"{}\",\n\t\t\"vertices\": [\n",
            &frame.frame.name
        )?;

        let vertices = &frame.frame.verts;
        let x = ((vertices[0].v[0] as f32) * scale[0]) + translate[0];
        let y = ((vertices[0].v[1] as f32) * scale[1]) + translate[1];
        let z = ((vertices[0].v[2] as f32) * scale[2]) + translate[2];
        write!(writer, "\t\t\t{}, {}, {}", x, y, z)?;

        for i in 1..vertices.len() {
            let vert = &vertices[i];
            let x = ((vert.v[0] as f32) * scale[0]) + translate[0];
            let y = ((vert.v[1] as f32) * scale[1]) + translate[1];
            let z = ((vert.v[2] as f32) * scale[2]) + translate[2];
            write!(writer, ",\n\t\t\t{}, {}, {}", x, y, z)?;

            // let nx = NORMALS[vert.normal_index as usize][0];
            // let ny = NORMALS[vert.normal_index as usize][1];
            // let nz = NORMALS[vert.normal_index as usize][2];
        }
        write!(writer, "\n\t\t]\n\t}}")?;
        Ok(())
    }

    fn write_triangles(&self, writer: &mut dyn Write) -> std::result::Result<(), std::io::Error> {
        let w = self.header.skin_width as f32;
        let h = self.header.skin_height as f32;

        let mut texcoords_front = vec![(0f32, 0f32); self.header.num_verices as usize];
        let mut texcoords_back = vec![(0f32, 0f32); self.header.num_verices as usize];
        let mut indices_front = Vec::<i32>::new();
        let mut indices_back = Vec::<i32>::new();

        for triangle in &self.triangles {
            if triangle.facefront != 0 {
                for v in triangle.vertex.iter() {
                    let idx = *v as usize;
                    let s = (self.texcoords[idx].s as f32 + 0.5) / w;
                    let t = (self.texcoords[idx].t as f32 + 0.5) / h;
                    texcoords_front[idx] = (s, t);
                    indices_front.push(*v);
                }
            } else {
                for v in triangle.vertex.iter() {
                    let idx = *v as usize;
                    let s = if self.texcoords[idx].onseam > 0 {
                        (self.texcoords[idx].s as f32 + (w * 0.5f32) + 0.5) / w
                    } else {
                        (self.texcoords[idx].s as f32 + 0.5) / w
                    };
                    let t = (self.texcoords[idx].t as f32 + 0.5) / h;
                    texcoords_back[idx] = (s, t);
                    indices_back.push(*v);
                }
            }
        }
        // write indices front
        write!(
            writer,
            "\n\t\"indices_front\": [\n\t\t{}, {}, {}",
            indices_front[0], indices_front[1], indices_front[2]
        )?;
        for i in 1..(indices_front.len() / 3) {
            write!(
                writer,
                ",\n\t\t{}, {}, {}",
                indices_front[i * 3 + 0],
                indices_front[i * 3 + 1],
                indices_front[i * 3 + 2]
            )?;
        }
        write!(writer, "\n\t],\n")?;

        // back
        write!(
            writer,
            "\t\"indices_back\": [\n\t\t{}, {}, {}",
            indices_back[0], indices_back[1], indices_back[2]
        )?;
        for i in 1..(indices_back.len() / 3) {
            write!(
                writer,
                ",\n\t\t{}, {}, {}",
                indices_back[i * 3 + 0],
                indices_back[i * 3 + 1],
                indices_back[i * 3 + 2]
            )?;
        }
        write!(writer, "\n\t],\n")?;

        // write texture coordinates front
        write!(
            writer,
            "\t\"texcoords_front\": [\n\t\t{}, {}",
            texcoords_front[0].0, texcoords_front[0].1
        )?;
        for i in 1..texcoords_front.len() {
            write!(
                writer,
                ",\n\t\t{}, {}",
                texcoords_front[i].0, texcoords_front[i].1
            )?;
        }
        write!(writer, "\n\t],\n")?;

        // back
        write!(
            writer,
            "\t\"texcoords_back\": [\n\t\t{}, {}",
            texcoords_back[0].0, texcoords_back[0].1
        )?;
        for i in 1..texcoords_back.len() {
            write!(
                writer,
                ",\n\t\t{}, {}",
                texcoords_back[i].0, texcoords_back[i].1
            )?;
        }
        write!(writer, "\n\t]\n")?;
        Ok(())
    }
}
