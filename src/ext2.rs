
// implementation template for convenience
// see https://www.nongnu.org/ext2-doc/ext2.html for documentation on ext2 fs

use std::{fs, io, u128};
use std::io::{Read, Seek, SeekFrom};
use crate::ext2;

#[derive(Debug)]
pub struct Ext2FS {
	super_block: Superblock,
    // The block group descriptor table is an array of block group descriptor, used to define parameters of all the block groups.
    // block_group_descriptors: Vec<BlockGroupDescriptor>,
}

pub struct BlockIter {
	// TODO: fill this structure
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct Superblock {
	inodes_count: u32,
	blocks_count: u32,
	block_size: u32,
    blocks_per_group: u32,
	rev_level: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct BlockGroupDescriptor {
    bg_block_bitmap: u32,
    bg_inode_bitmap: u32,
    bg_inode_table: u32,
    bg_free_blocks_count: u16,
    bg_free_inodes_count: u16,
    bg_used_dirs_count: u16,
    bg_pad: u16,
    bg_reserved: u128,
}

impl Superblock {
	fn new(block: &[u8]) -> Self {
		// Parse relevant fields from the Superblock
		let inodes_count = u32::from_le_bytes(block[0..4].try_into().unwrap());
		let blocks_count = u32::from_le_bytes(block[4..8].try_into().unwrap());
		let log_block_size = u32::from_le_bytes(block[24..28].try_into().unwrap());
        let blocks_per_group = u32::from_le_bytes(block[32..36].try_into().unwrap());
		let rev_level = u32::from_le_bytes(block[76..80].try_into().unwrap());

		// Calculate the block size#
		// The block size is computed using this 32bit value as the number of bits to shift left the value 1024. This value may only be non-negative.
		let block_size = 1024 << log_block_size;

		Superblock {
			inodes_count,
			blocks_count,
			block_size,
            blocks_per_group,
			rev_level
		}
	}

    // Add getter methods to safely access fields
    fn inodes_count(&self) -> u32 {
        self.inodes_count
    }

    fn blocks_count(&self) -> u32 {
        self.blocks_count
    }

    fn block_size(&self) -> u32 {
        self.block_size
    }

    fn rev_level(&self) -> u32 {
        self.rev_level
    }
}

impl BlockGroupDescriptor {
	fn new(data: &[u8]) -> Self {
        let bg_block_bitmap =  u32::from_le_bytes(data[0..4].try_into().unwrap());
        let bg_inode_bitmap =  u32::from_le_bytes(data[4..8].try_into().unwrap());
        let bg_inode_table =  u32::from_le_bytes(data[8..12].try_into().unwrap());
        let bg_free_blocks_count =  u16::from_le_bytes(data[12..14].try_into().unwrap());
        let bg_free_inodes_count =  u16::from_le_bytes(data[14..16].try_into().unwrap());
        let bg_used_dirs_count =  u16::from_le_bytes(data[16..18].try_into().unwrap());
        let bg_pad =  u16::from_le_bytes(data[18..20].try_into().unwrap());
        let bg_reserved =  u128::from_le_bytes(data[20..32].try_into().unwrap());

        BlockGroupDescriptor {
            bg_block_bitmap,
            bg_inode_bitmap,
            bg_inode_table,
            bg_free_blocks_count,
            bg_free_inodes_count,
            bg_used_dirs_count,
            bg_pad,
            bg_reserved,
        }
    }
}

impl Ext2FS {
	pub fn new(mut _device: fs::File) -> io::Result<Ext2FS> {
		// Seek to the Superblock location (offset 1024 bytes)
		_device.seek(SeekFrom::Start(1024))?;

		// Read 1024 bytes (size of the Superblock)
		let mut buffer = [0u8; 1024];
		_device.read_exact(&mut buffer)?;
		let superblock = Superblock::new(&buffer);

        // Print the parsed Superblock information
        println!("Inodes Count: {}", superblock.inodes_count());
        println!("Blocks Count: {}", superblock.blocks_count());
        println!("Block Size: {} bytes", superblock.block_size());
        println!("Revision Level: {}", superblock.rev_level());

        // Get BlockGroupDescriptor
        let block_size = superblock.block_size();
        // The block group descriptor table starts on the first block following the superblock.
        // This would be the third block on a 1KiB block file system, or the second block for 2KiB and larger block file systems.
        let descriptor_table_offset = if block_size > 1024 {
            block_size // Descriptor table is in block 1 if block size > 1024
        } else {
            1024 + block_size // Otherwise, it's in block 2
        };
        println!("block_size: {}", block_size);

		println!("Descriptor Table Offset: {}", descriptor_table_offset);
        // Depending on how many block groups are defined, this table can require multiple blocks of storage. Always refer to the superblock in case of doubt.
        // Calculate the number of block groups
        let blocks_per_group = superblock.blocks_count / superblock.blocks_per_group;
        // Each Block Group Descriptor is 32 bytes in size.
        let mut buffer = vec![0u8; 32 * blocks_per_group as usize];

        // Read the Block Group Descriptor Table
        _device.seek(SeekFrom::Start(descriptor_table_offset as u64))?;
        _device.read_exact(&mut buffer)?;

		Ok(
            Ext2FS {
                super_block: superblock,
		}
        )
	}

    pub fn read_block_group_descriptors(&mut self, file: &mut std::fs::File) -> std::io::Result<()> {
        let block_size = self.super_block.block_size() as u64;
        // The block group descriptor table starts on the first block following the superblock.
        // This would be the third block on a 1KiB block file system, or the second block for 2KiB and larger block file systems.
        let descriptor_table_offset = if block_size > 1024 {
            block_size // Descriptor table is in block 1 if block size > 1024
        } else {
            1024 + block_size// Otherwise, it's in block 2
        };

        // Depending on how many block groups are defined, this table can require multiple blocks of storage.
        // Calculate the number of block groups
        let blocks_per_group = self.super_block.blocks_count / self.super_block.blocks_per_group;
        let mut buffer = vec![0u8; 32 * blocks_per_group as usize];

        // Read the Block Group Descriptor Table
        file.seek(SeekFrom::Start(descriptor_table_offset))?;
        file.read_exact(&mut buffer)?;

        // Parse each Block Group Descriptor
        for i in 0..blocks_per_group {
            let start = (i as usize) * 32;
            let end = start + 32;
            let descriptor = BlockGroupDescriptor::new(&buffer[start..end]);
            // self.block_group_descriptors.push(descriptor);
        }

        Ok(())
    }
}

impl Iterator for BlockIter {
	type Item = (); 
	
	fn next(&mut self) -> Option<()> {
		// TODO: implement an iterator over unused blocks
		None
	}
}
