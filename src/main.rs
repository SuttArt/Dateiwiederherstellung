use std::{io, fs};
use std::io::{Read, Seek, SeekFrom};

mod ext2;


fn recover_files(_device: fs::File, _path: &str) -> io::Result<()> {

	let ext2_fs = ext2::Ext2FS::new(_device)?;

	Ok(())
}

fn main() -> io::Result<()> {
	use std::env::args;
	let device_path = args().nth(1).unwrap_or("examples/small.img".to_string());
	let target_path = args().nth(2).unwrap_or("restored".to_string());
	
	fs::create_dir_all(&target_path)?;
	recover_files(fs::File::open(&device_path)?, &target_path)
}
