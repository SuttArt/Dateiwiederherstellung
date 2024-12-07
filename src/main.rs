use std::{io, fs};
use std::io::{Read, Seek, Write};

mod ext2;


fn recover_files(mut _device: fs::File, _path: &str) -> io::Result<()> {
	// TODO: read superblock, iterate over unused blocks, find jpg start and end patterns, copy data

	// read superblock, BlockGroupDescriptor, some usefully data
	let ext2_fs = ext2::Ext2FS::new(_device.try_clone()?)?;

	// iterate over unused blocks
	let block_iter = ext2::BlockIter::new(&ext2_fs);

	let block_size = ext2_fs.super_block.block_size();
	// Buffer to store current JPEG data
	let mut current_jpeg: Option<Vec<u8>> = None;
	// Start block of JPEG
	let mut jpeg_start_block: Option<usize> = None;

	// Use the iterator to process blocks
	for (group_number, block_number, is_used) in block_iter {
		if is_used {
			continue; // Skip used blocks
		}

		// Calculate the block offset in the FS
		let block_offset = (block_number as u64) * (block_size as u64);

		// Read the block data
		_device.seek(io::SeekFrom::Start(block_offset))?;
		let mut block_data = vec![0; block_size as usize];
		_device.read_exact(&mut block_data)?;

		// Search for JPEG SOI (FFD8) and EOI (FFD9) markers
		// For test, how it works:
		// let block_data = [0x12, 0x34, 0xFF, 0xD8, 0x99];
		//     if let Some(pos) = block_data.windows(2).position(|w| w == [0xFF, 0xD8]) {
		//         println!("JPEG SOI marker found at index: {}", pos);
		//     }
		if let Some(pos) = block_data.windows(2).position(|w| w == [0xFF, 0xD8]) {
			// Found JPEG Start
			jpeg_start_block = Some(block_number);
			// Start saving JPEG data to Vector
			current_jpeg = Some(block_data[pos..].to_vec());
			println!("JPEG Start found in Block Group {}, Block {}", group_number, block_number);
		} else if let Some(pos) = block_data.windows(2).position(|w| w == [0xFF, 0xD9]) {
			// Found JPEG End
			// Check if we hat some data before
			if let Some(ref mut jpeg_data) = current_jpeg {
				// Add the block up to EOI
				// create slice of block_data from the beginning of the block up to and including the pos + 1 index.
				jpeg_data.extend_from_slice(&block_data[..=pos + 1]);

				// Save the JPEG data to a file.
				let filename = format!("{}/recovered_{}_{}.jpg", _path, group_number, jpeg_start_block.unwrap());
				let mut file = fs::File::create(&filename)?;
				file.write_all(jpeg_data)?;

				println!("JPEG saved to {}", filename);

				// Reset current JPEG buffer
				current_jpeg = None;
				jpeg_start_block = None;
			}
		} else if let Some(ref mut jpeg_data) = current_jpeg {
			// Continue buffering JPEG data
			jpeg_data.extend_from_slice(&block_data);
		}
	}

	Ok(())
}

fn main() -> io::Result<()> {
	use std::env::args;
	let device_path = args().nth(1).unwrap_or("examples/medium.img".to_string());
	let target_path = args().nth(2).unwrap_or("restored".to_string());

	fs::create_dir_all(&target_path)?;
	recover_files(fs::File::open(&device_path)?, &target_path)
}
