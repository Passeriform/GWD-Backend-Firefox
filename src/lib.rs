pub mod errors;

use crate::errors::LZ4Error;
use dirs;
use glob::glob;
use godwit_daemon::config;
use godwit_daemon::core::{Backend, BackendArgs, Registrar};
use godwit_daemon::errors::TraceError;
use godwit_daemon::export_backend;
use lz4::block::decompress;
use serde_json::{self, Value};
use std::fs::File;
use std::io::{prelude::*, Seek, SeekFrom};
use std::str;

fn parse_jsonlz4(lz4file: &mut File) -> Result<Value, LZ4Error> {
    // Skip 8 bytes for jsonlz4 type
    lz4file.seek(SeekFrom::Start(8))?;

    let mut sizebuffer = [0; 4];
    lz4file.read(&mut sizebuffer)?;
    let size = u32::from_le_bytes(sizebuffer);

    let mut lz4buffer = Vec::new();
    lz4file.read_to_end(&mut lz4buffer)?;

    let json_str = decompress(&lz4buffer, Some(size as i32))?;

    let parsed: Value = serde_json::from_str(str::from_utf8(&json_str)?)?;
    Ok(parsed)
}

pub fn trace(refresh: bool) -> Result<(), TraceError> {
    let mozilla_profile_path = dirs::home_dir().unwrap().as_path().join(".mozilla/firefox");

    for lz4path in glob(&format!(
        "{}{}",
        mozilla_profile_path
            .to_str()
            .expect("Path couldn't be converted to string."),
        "/*.default*/sessionstore-backups/*.*lz4*"
    ))? {
        let lz4path = lz4path?;
        let mut lz4file = File::open(&lz4path)?;
        let parsed_json = parse_jsonlz4(&mut lz4file)?;

        if refresh {
            config::purge_base_file("Firefox", &lz4path)?;
        }

        config::update_patches("Firefox", &lz4path, parsed_json)?;
    }
    Ok(())
}

pub struct Firefox;

impl Backend for Firefox {
    fn trace(&self, _args: BackendArgs) -> Result<(), TraceError> {
        trace(_args.refresh)
    }
}

export_backend!(register);

extern "C" fn register(registrar: &mut dyn Registrar) {
    registrar.register_backend("Firefox", Box::new(Firefox));
}
