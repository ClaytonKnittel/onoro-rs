use std::{env, error::Error, fs};

fn main() -> Result<(), Box<dyn Error>> {
  let proto_dir = env::current_dir()?.parent().unwrap().join("proto");
  prost_build::compile_protos(
    &fs::read_dir(&proto_dir)?
      .map(|proto_file| proto_file.map(|file| file.path()))
      .collect::<Result<Vec<_>, _>>()?[..],
    &[&proto_dir],
  )?;
  Ok(())
}
