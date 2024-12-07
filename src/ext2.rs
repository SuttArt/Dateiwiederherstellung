// implementation template for convenience
// see https://www.nongnu.org/ext2-doc/ext2.html for documentation on ext2 fs

use std::{fs, io, u128};
use std::io::{Read, Seek, SeekFrom};
use crate::ext2;

#[derive(Debug)]
pub struct Ext2FS {
    super_block: Superblock,
    // The block group descriptor table is an array of block group descriptor, used to define parameters of all the block groups.
    block_group_descriptors: Vec<BlockGroupDescriptor>,
    block_bitmaps: Vec<Vec<u8>>, // A vector of block bitmaps for each block group
    data_blocks_offsets: Vec<u32>,
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
    inodes_per_group: u32,
    rev_level: u32,
    inode_size: u16,
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
    bg_reserved: [u8; 12],
}

impl Superblock {
    fn new(block: &[u8]) -> Self {
        // Parse relevant fields from the Superblock
        let inodes_count = u32::from_le_bytes(block[0..4].try_into().unwrap());
        let blocks_count = u32::from_le_bytes(block[4..8].try_into().unwrap());
        let log_block_size = u32::from_le_bytes(block[24..28].try_into().unwrap());
        let blocks_per_group = u32::from_le_bytes(block[32..36].try_into().unwrap());
        let inodes_per_group = u32::from_le_bytes(block[40..44].try_into().unwrap());
        let rev_level = u32::from_le_bytes(block[76..80].try_into().unwrap());
        let inode_size = u16::from_le_bytes(block[88..90].try_into().unwrap());

        // Calculate the block size#
        // The block size is computed using this 32bit value as the number of bits to shift left the value 1024. This value may only be non-negative.
        let block_size = 1024 << log_block_size;

        Superblock {
            inodes_count,
            blocks_count,
            block_size,
            blocks_per_group,
            inodes_per_group,
            rev_level,
            inode_size,
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

    fn blocks_per_group(&self) -> u32 { self.blocks_per_group }
    fn inodes_per_group(&self) -> u32 { self.inodes_per_group }
    fn rev_level(&self) -> u32 {
        self.rev_level
    }
    fn inode_size(&self) -> u16 { self.inode_size }

    fn print_parsed_info(&self) {
        // Print the parsed Superblock information
        println!("\x1b[32mPrint the parsed Superblock information:\x1b[0m");
        println!("Inodes Count: {}", self.inodes_count());
        println!("Blocks Count: {}", self.blocks_count());
        println!("Block Size: {} bytes", self.block_size());
        println!("Blocks per group: {}", self.blocks_per_group());
        println!("Inodes per group: {}", self.inodes_per_group());
        println!("Revision Level: {}", self.rev_level());
        println!("Inode size: {}", self.inode_size());
    }
}

impl BlockGroupDescriptor {
    // Temporary(?) solution, because I don't want to store invalid BGDs, and while
    // bg_block_bitmap == 0 is invalid - we will filter it out
    fn new(data: &[u8]) -> Option<Self> {
        let bg_block_bitmap = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let bg_inode_bitmap = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let bg_inode_table = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let bg_free_blocks_count = u16::from_le_bytes(data[12..14].try_into().unwrap());
        let bg_free_inodes_count = u16::from_le_bytes(data[14..16].try_into().unwrap());
        let bg_used_dirs_count = u16::from_le_bytes(data[16..18].try_into().unwrap());
        let bg_pad = u16::from_le_bytes(data[18..20].try_into().unwrap());
        let bg_reserved = data[20..32].try_into().unwrap();

        // Check if bg_block_bitmap is invalid
        if bg_block_bitmap == 0 {
            return None; // Return None if the block bitmap is invalid
        }

        Some(BlockGroupDescriptor {
            bg_block_bitmap,
            bg_inode_bitmap,
            bg_inode_table,
            bg_free_blocks_count,
            bg_free_inodes_count,
            bg_used_dirs_count,
            bg_pad,
            bg_reserved,
        })
    }

    fn bg_block_bitmap(&self) -> u32 { self.bg_block_bitmap }
    fn bg_inode_bitmap(&self) -> u32 { self.bg_inode_bitmap }
    fn bg_inode_table(&self) -> u32 { self.bg_inode_table }
    fn bg_free_blocks_count(&self) -> u16 { self.bg_free_blocks_count }
    fn bg_free_inodes_count(&self) -> u16 { self.bg_free_inodes_count }
    fn bg_used_dirs_count(&self) -> u16 { self.bg_used_dirs_count }
    fn bg_pad(&self) -> u16 { self.bg_pad }
    fn bg_reserved(&self) -> &[u8; 12] { &self.bg_reserved }

    pub fn print_parsed_info(&self, group_number: usize) {
        // Print the parsed Block Group Descriptor information
        // green text: "\x1b[32m ... \x1b[0m"
        println!("\x1b[32mPrint the parsed Block Group Descriptor information:\x1b[0m");
        println!("Block Group {}:", group_number);
        println!("Block Bitmap Block: {}", self.bg_block_bitmap());
        println!("Inode Bitmap Block: {}", self.bg_inode_bitmap());
        println!("Inode Table Block: {}", self.bg_inode_table());
        println!("Free Blocks Count: {}", self.bg_free_blocks_count());
        println!("Free Inodes Count: {}", self.bg_free_inodes_count());
        println!("Used Directories Count: {}", self.bg_used_dirs_count());
        println!("Padding: {}", self.bg_pad());
        println!("Reserved: {:?}", self.bg_reserved()); // Print reserved as a byte array
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

        superblock.print_parsed_info();

        // Get BlockGroupDescriptor
        let block_size = superblock.block_size();
        // The block group descriptor table starts on the first block following the superblock.
        // This would be the third block on a 1KiB block file system, or the second block for 2KiB and larger block file systems.
        let descriptor_table_offset = if block_size > 1024 {
            block_size // Descriptor table is in block 2 if block size > 1024
        } else {
            2 * block_size // Otherwise, it's in block 3
        };

        println!("Descriptor Table Offset: {}", descriptor_table_offset);
        // Depending on how many block groups are defined, this table can require multiple blocks of storage. Always refer to the superblock in case of doubt.
        // Calculate the number of block groups

        // For now, because I see this:
        // Block count:              2048
        // Blocks per group:         8192
        // And it's make no sense for me, how can I calculate block_groups_number
        // I assume, that Block Group Descriptor Table can not span more than 1 block (in fact, according to the documentation, that's not true.)
        let block_groups_number = block_size / 32; //superblock.blocks_count() / superblock.blocks_per_group();
        // Each Block Group Descriptor is 32 bytes in size.
        let mut buffer = vec![0u8; 32 * block_groups_number as usize];

        // Read the Block Group Descriptor Table
        _device.seek(SeekFrom::Start(descriptor_table_offset as u64))?;
        _device.read_exact(&mut buffer)?;

        let mut block_group_descriptors: Vec<BlockGroupDescriptor> = vec![];

        // Parse each Block Group Descriptor
        for i in 0..block_groups_number {
            let start = (i as usize) * 32;
            let end = start + 32;
            if let Some(descriptor) = BlockGroupDescriptor::new(&buffer[start..end]) {
                block_group_descriptors.push(descriptor);
            } else {
                // Red text: "\x1b[31m ...  \x1b[0m"
                println!("\x1b[31mInvalid Block Bitmap Block in Block Group {}\x1b[0m", i);
                break; // Exit the loop if an invalid descriptor is encountered
            }
        }

        for (i, descriptor) in block_group_descriptors.iter().enumerate() {
            descriptor.print_parsed_info(i);
        }



        // Calculate the Size of the Block Bitmap
        // The first block of this block group is represented by bit 0 of byte 0,
        // the second by bit 1 of byte 0. The 8th block is represented by bit 7 (most significant bit) of byte 0
        // while the 9th block is represented by bit 0 (least significant bit) of byte 1.
        // 1 byte = 8 bits, so each byte in the bitmap represents 8 blocks.
        let block_bitmap_size = superblock.blocks_per_group() / 8;

        let mut block_bitmaps: Vec<Vec<u8>> = Vec::new();

        for (_, descriptor) in block_group_descriptors.iter().enumerate() {
            let block_bitmap_offset = descriptor.bg_block_bitmap() * block_size;

            // Read the Block Bitmap
            let mut bitmap_buffer = vec![0u8; block_bitmap_size as usize];
            _device.seek(SeekFrom::Start(block_bitmap_offset as u64))?;
            _device.read_exact(&mut bitmap_buffer)?;

            // println!("Loaded Block Bitmap for Block Group {}: {:?}", i, bitmap_buffer);
            block_bitmaps.push(bitmap_buffer);
        }

        // green text: "\x1b[32m ... \x1b[0m"
        println!("\x1b[32mBlock bitmaps length: \x1b[0m{:?}", block_bitmaps.len());

        // We need data_blocks_offsets for each group, to get the data blocks
        // Inode Table Size (in bytes) = Inodes per Group * Inode Size
        // Or in blocks:
        let inode_table_size = superblock.inodes_per_group() / (block_size / superblock.inode_size() as u32);

        let mut data_blocks_offsets = Vec::new();

        for (_, descriptor) in block_group_descriptors.iter().enumerate() {
            // Calculate the data block start for this group
            let data_block_offset = inode_table_size + descriptor.bg_inode_table();

            data_blocks_offsets.push(data_block_offset);

        }

        // green text: "\x1b[32m ... \x1b[0m"
        println!("\x1b[32mData Blocks Offsets length: \x1b[0m{}", data_blocks_offsets.len());



        Ok(
            Ext2FS {
                super_block: superblock,
                block_group_descriptors,
                block_bitmaps,
                data_blocks_offsets
            }
        )
    }
}

impl Iterator for BlockIter {
    type Item = ();

    fn next(&mut self) -> Option<()> {
        // TODO: implement an iterator over unused blocks
        None
    }
}
