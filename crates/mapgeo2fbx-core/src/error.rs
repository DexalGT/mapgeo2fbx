use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse mapgeo file {path}: {source}")]
    MapgeoParse {
        path: PathBuf,
        #[source]
        source: ritoshark::mapgeo::Error,
    },

    #[error("unsupported vertex element format for {element:?}: {format:?}")]
    UnsupportedVertexFormat {
        element: ritoshark::mapgeo::ElementName,
        format: ritoshark::mapgeo::ElementFormat,
    },

    #[error("model {model} references missing vertex buffer id {id}")]
    MissingVertexBuffer { model: String, id: i32 },

    #[error("model {model} references missing vertex description id {id}")]
    MissingVertexDescription { model: String, id: i32 },

    #[error("model {model} references missing index buffer id {id}")]
    MissingIndexBuffer { model: String, id: i32 },

    #[error("model {model} submesh index range [{start}..{end}) exceeds index buffer length {buffer_len}")]
    SubmeshIndexOutOfRange {
        model: String,
        start: usize,
        end: usize,
        buffer_len: usize,
    },

    #[error("model {model} vertex buffer too short: needed {needed} bytes at offset {offset}, buffer is {buffer_len} bytes")]
    VertexBufferTooShort {
        model: String,
        offset: usize,
        needed: usize,
        buffer_len: usize,
    },

    #[error("mesh {mesh} submesh references vertex index {index} but mesh has only {vertex_count} vertices")]
    VertexIndexOutOfRange {
        mesh: String,
        index: u32,
        vertex_count: usize,
    },

    #[error("failed to write fbx file {path}: {source}")]
    FbxWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("i/o error: {source}")]
    Write {
        #[from]
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
