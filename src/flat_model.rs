use super::vec3_t;
use std::io::Write;

pub struct FlatModel {
    pub vertices: Vec<Vec<vec3_t>>, // list of frames. each frame has same length is a list of vec3_t
    pub texcoords: Vec<(f32, f32)>, // should have the same langth as any of the frames
    pub indices: Vec<(usize, usize, usize)>, // basicaly a triangle
    // normals?
}

impl FlatModel {
    pub fn write_json(&self, writer: &mut dyn Write) -> Result<(), std::io::Error> {
        write!(writer, "{{\n")?;

        self.write_frames(writer)?;
        self.write_faces(writer)?;
        write!(writer, "\n}}")?;
        Ok(())
    }

    fn write_frames(&self, writer: &mut dyn Write) -> Result<(), std::io::Error> {
        if self.vertices.is_empty() {
            return Ok(());
        }

        write!(writer, "\n\t\"frames\": [")?;
        self.write_frame(writer, 0)?;
        for idx in 1..self.vertices.len() {
            write!(writer, ",")?;
            self.write_frame(writer, idx)?;
        }
        write!(writer, "\t],")?;
        Ok(())
    }

    fn write_frame(&self, writer: &mut dyn Write, idx: usize) -> Result<(), std::io::Error> {
        write!(writer, "{{\n\t\t\"vertices\": [\n")?;
        let vertices = &self.vertices[idx];

        let x: f32 = vertices[0][0];
        let y: f32 = vertices[0][1];
        let z: f32 = vertices[0][2];
        write!(writer, "\t\t\t{}, {}, {}", x, y, z)?;
        for i in 1..vertices.len() {
            let x: f32 = vertices[i][0];
            let y: f32 = vertices[i][1];
            let z: f32 = vertices[i][2];
            write!(writer, ",\n\t\t\t{}, {}, {}", x, y, z)?;
        }
        write!(writer, "\n\t\t]\n\t}}")?;
        Ok(())
    }

    fn write_faces(&self, writer: &mut dyn Write) -> Result<(), std::io::Error> {
        let indices = &self.indices;

        let (a, b, c) = indices[0];
        write!(writer, "\n\t\"indices\": [\n\t\t{}, {}, {}", a, b, c)?;
        for i in 1..indices.len() {
            let (a, b, c) = indices[i];
            write!(writer, ",\n\t\t{}, {}, {}", a, b, c)?;
        }
        write!(writer, "\n\t],\n")?;

        let texcoords = &self.texcoords;
        let (s, t) = texcoords[0];
        write!(writer, "\t\"texcoords\": [\n\t\t{}, {}", s, t)?;
        for i in 1..texcoords.len() {
            let (s, t) = texcoords[i];
            write!(writer, ",\n\t\t{}, {}", s, t)?;
        }
        write!(writer, "\n\t]\n")?;
        Ok(())
    }

    pub fn from_md2(model: &super::md2::Model) -> Self {
        let w = model.header.skin_width as f32;
        let h = model.header.skin_height as f32;

        let mut vertices = Vec::<Vec<vec3_t>>::with_capacity(model.frames.len());
        for frame in &model.frames {
            let scale = frame.scale;
            let translate = frame.translate;

            let mut temp = Vec::<vec3_t>::new();

            for vertex in &frame.vertices {
                let x = (vertex.v[0] as f32 * scale[0]) + translate[0];
                let y = (vertex.v[1] as f32 * scale[1]) + translate[1];
                let z = (vertex.v[2] as f32 * scale[2]) + translate[2];
                temp.push([x, y, z]);
            }

            vertices.push(temp);
        }
        use std::collections::HashMap;
        let mut set = HashMap::<usize, HashMap<usize, usize>>::new();
        let mut indices = Vec::<usize>::new();
        let mut texcoords = vec![(0f32, 0f32); vertices[0].len() * 2];

        for face in &model.faces {
            for i in 0..3 {
                let vec_idx = face.vertex[i] as usize;
                let tex_idx = face.st_idx[i] as usize;
                let st = {
                    let s = model.texcoords[tex_idx].s as f32 / w;
                    let t = model.texcoords[tex_idx].t as f32 / h;
                    (s, t)
                };
                /*
                1) if the vertex (vec_idx) is new:
                store vec_idx in indices
                store texcoords (s, t) at the vec_idx index
                in texcoords

                2) if we have seen the vertex already
                check if it has same texcoords.
                    is this the case: store previously used index in indices
                    if not: 3) copy vertex and push it in new position
                    store this position in indices and texcoords at this new position

                */
                if !set.contains_key(&vec_idx) {
                    // 1)
                    indices.push(vec_idx);
                    texcoords[vec_idx] = st;
                    let mut new_map = HashMap::new();
                    new_map.insert(tex_idx, vec_idx);
                    set.insert(vec_idx, new_map);
                } else {
                    if set[&vec_idx].contains_key(&tex_idx) {
                        // 2)
                        let idx = set[&vec_idx][&tex_idx];
                        indices.push(idx);
                    } else {
                        // 3)
                        for frame in &mut vertices {
                            let vertex = frame[vec_idx];
                            frame.push(vertex);
                        }

                        let new_idx = vertices[0].len() - 1;
                        indices.push(new_idx);
                        texcoords[new_idx] = st;
                        set.get_mut(&vec_idx).unwrap().insert(tex_idx, new_idx);
                    }
                }
            }
        }

        let mut fi = Vec::new();
        for i in 0..indices.len() / 3 {
            let a = indices[i * 3 + 0];
            let b = indices[i * 3 + 1];
            let c = indices[i * 3 + 2];
            fi.push((a, b, c));
        }

        texcoords.truncate(vertices[0].len());
        FlatModel {
            vertices: vertices,
            indices: fi,
            texcoords: texcoords,
        }
    }

    pub fn from_mdl(model: &super::mdl::Model) -> Self {
        let scale = model.header.scale;
        let translate = model.header.translate;
        let w = model.header.skin_width as f32;
        let h = model.header.skin_height as f32;

        let mut vertices = Vec::<Vec<vec3_t>>::new();

        for frame in &model.frames {
            let mut temp = Vec::<vec3_t>::with_capacity(model.header.num_verices as usize);
            for vertex in &frame.frame.verts {
                let x = ((vertex.v[0] as f32) * scale[0]) + translate[0];
                let y = ((vertex.v[1] as f32) * scale[1]) + translate[1];
                let z = ((vertex.v[2] as f32) * scale[2]) + translate[2];

                temp.push([x, y, z]);
            }
            vertices.push(temp);
        }
        let mut texcoords = vec![(0f32, 0f32); vertices[0].len() * 3];
        let mut indices = Vec::<usize>::new();

        for face in &model.triangles {
            let is_back = face.facefront == 0;
            for v in face.vertex.iter() {
                let idx = *v as usize;
                let onseam = model.texcoords[idx].onseam > 0;
                if is_back && onseam {
                    let s = (model.texcoords[idx].s as f32 + (w * 0.5f32) + 0.5) / w;
                    let t = (model.texcoords[idx].t as f32 + 0.5) / h;
                    for vertex in &mut vertices {
                        let new_vertex = vertex[idx];
                        vertex.push(new_vertex);
                    }
                    let new_idx = vertices[0].len() - 1;
                    indices.push(new_idx);
                    texcoords[new_idx] = (s, t);
                } else {
                    let s = (model.texcoords[idx].s as f32 + 0.5) / w;
                    let t = (model.texcoords[idx].t as f32 + 0.5) / h;
                    texcoords[idx] = (s, t);
                    indices.push(idx);
                }
            }
        }

        let mut fi = Vec::new();
        for i in 0..indices.len() / 3 {
            let a = indices[i * 3 + 0];
            let b = indices[i * 3 + 1];
            let c = indices[i * 3 + 2];
            fi.push((a, b, c));
        }

        texcoords.truncate(vertices[0].len());
        FlatModel {
            vertices: vertices,
            texcoords: texcoords,
            indices: fi,
        }
    }
}
