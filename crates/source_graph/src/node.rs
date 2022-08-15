use std::path::PathBuf;

use module_attributes2::Module;
use syn::{File, Type};

use crate::parser::Uses;

#[derive(Clone, Debug)]
pub enum Node {
    CyclerInstance { instance: String },
    CyclerModule { module: String, path: PathBuf },
    HardwareInterface,
    Module { module: Module },
    ParsedRustFile { file: File },
    RustFilePath { path: PathBuf },
    Struct { name: String },
    StructField { data_type: Type },
    Uses { uses: Uses },
}