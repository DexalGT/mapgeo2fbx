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
