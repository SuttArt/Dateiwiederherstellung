use std::{io, fs};

mod ext2;


fn recover_files(_device: fs::File, _path: &str) -> io::Result<()> {
	// TODO: read superblock, iterate over unused blocks, find jpg start and end patterns, copy data

	// read superblock, BlockGroupDescriptor, some usefully data
	let ext2_fs = ext2::Ext2FS::new(_device)?;

	// iterate over unused blocks
	let block_iter = ext2::BlockIter::new(ext2_fs);

	// Use the iterator to process blocks
	for (group_number, block_number, is_used) in block_iter {
		println!(
			"Block Group {}, Block {}: {}",
			group_number,
			block_number,
			if is_used { "Used" } else { "Free" }
		);
	}

	Ok(())
}

fn main() -> io::Result<()> {
	use std::env::args;
	let device_path = args().nth(1).unwrap_or("examples/small.img".to_string());
	let target_path = args().nth(2).unwrap_or("restored".to_string());
	
	fs::create_dir_all(&target_path)?;
	recover_files(fs::File::open(&device_path)?, &target_path)
}
