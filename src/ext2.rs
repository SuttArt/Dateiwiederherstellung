// implementation template for convenience
// see https://www.nongnu.org/ext2-doc/ext2.html for documentation on ext2 fs

use std::{fs, io};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug)]
pub struct Ext2FS {
    pub super_block: Superblock,
    // The block group descriptor table is an array of block group descriptor, used to define parameters of all the block groups.
    #[allow(dead_code)]
    block_group_descriptors: Vec<BlockGroupDescriptor>,
    inode_table: Vec<Inode>,
    block_bitmaps: Vec<Vec<u8>>, // A vector of block bitmaps for each block group
    data_blocks_offsets: Vec<u32>,
}

pub struct BlockIter<'a> {
    ext2_fs: &'a Ext2FS,
    current_group: usize,
    current_byte: usize,
    current_bit: usize,
    block_number: usize,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Superblock {
    inodes_count: u32,        // Total number of inodes
    blocks_count: u32,        // Total number of blocks
    block_size: u32,          // Block size
    blocks_per_group: u32,    // Number of blocks per group
    inodes_per_group: u32,    // Number of inodes per group
    rev_level: u32,           // Revision level
    inode_size: u16,          // Size of inode structure
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct BlockGroupDescriptor {
    bg_block_bitmap: u32,      // Block ID of the block bitmap
    bg_inode_bitmap: u32,      // Block ID of the inode bitmap
    bg_inode_table: u32,       // Starting block ID of the inode table
    bg_free_blocks_count: u16, // Number of free blocks in the group
    bg_free_inodes_count: u16, // Number of free inodes in the group
    bg_used_dirs_count: u16,   // Number of used directories in the group
    bg_pad: u16,               // Padding to align the structure
    bg_reserved: [u8; 12],     // Reserved space for future use
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct Inode {
    i_mode: u16,              // File mode (type and permissions)
    i_uid: u16,               // User ID of owner
    i_size: u32,              // Size of file in bytes
    i_atime: u32,             // Access time (UNIX timestamp)
    i_ctime: u32,             // Creation time (UNIX timestamp)
    i_mtime: u32,             // Modification time (UNIX timestamp)
    i_dtime: u32,             // Deletion time (UNIX timestamp)
    i_gid: u16,               // Group ID of owner
    i_links_count: u16,       // Hard link count
    i_blocks: u32,            // Number of 512-byte blocks allocated
    i_flags: u32,             // File flags
    i_osd1: u32,              // OS-dependent value
    i_block: [u32; 15],       // Pointers to data blocks
    i_generation: u32,        // File version (for NFS)
    i_file_acl: u32,          // File ACL (Access Control List)
    i_dir_acl: u32,           // Directory ACL or file size high bits
    i_faddr: u32,             // Fragment address
    i_osd2: [u8; 12],         // OS-dependent value
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

    pub fn block_size(&self) -> u32 {
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

    pub fn get_all_info(&self) -> String {
        format!(
            "Superblock Information:\n\
            Inodes Count: {}\n\
            Blocks Count: {}\n\
            Block Size: {} bytes\n\
            Blocks per Group: {}\n\
            Inodes per Group: {}\n\
            Revision Level: {}\n\
            Inode Size: {} bytes\n",
            self.inodes_count(),
            self.blocks_count(),
            self.block_size(),
            self.blocks_per_group(),
            self.inodes_per_group(),
            self.rev_level(),
            self.inode_size(),
        )
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
    pub fn get_all_info(&self, group_number: usize) -> String {
        format!(
            "Block Group Descriptor Information:\n\
            Block Group: {}\n\
            Block Bitmap: {}\n\
            Inode Bitmap: {}\n\
            Inode Table: {}\n\
            Free Blocks Count: {}\n\
            Free Inodes Count: {}\n\
            Used Directories Count: {}\n\
            Padding: {}\n\
            Reserved: {:?}\n\
            \n\n\n",
            group_number,
            self.bg_block_bitmap(),
            self.bg_inode_bitmap(),
            self.bg_inode_table(),
            self.bg_free_blocks_count(),
            self.bg_free_inodes_count(),
            self.bg_used_dirs_count(),
            self.bg_pad(),
            self.bg_reserved(),
        )
    }
}
impl Inode {
    fn new(data: &[u8]) -> Option<Self> {
        let i_mode = u16::from_le_bytes(data[0..2].try_into().unwrap());

        // Validate fields
        if i_mode == 0 {
            return None; // Return None if the inode is unused
        }

        let i_uid = u16::from_le_bytes(data[2..4].try_into().unwrap());
        let i_size = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let i_atime = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let i_ctime = u32::from_le_bytes(data[12..16].try_into().unwrap());
        let i_mtime = u32::from_le_bytes(data[16..20].try_into().unwrap());
        let i_dtime = u32::from_le_bytes(data[20..24].try_into().unwrap());
        let i_gid = u16::from_le_bytes(data[24..26].try_into().unwrap());
        let i_links_count = u16::from_le_bytes(data[26..28].try_into().unwrap());
        let i_blocks = u32::from_le_bytes(data[28..32].try_into().unwrap());
        let i_flags = u32::from_le_bytes(data[32..36].try_into().unwrap());
        let i_osd1 = u32::from_le_bytes(data[36..40].try_into().unwrap());
        let i_block: [u32; 15] = data[40..100]
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let i_generation = u32::from_le_bytes(data[100..104].try_into().unwrap());
        let i_file_acl = u32::from_le_bytes(data[104..108].try_into().unwrap());
        let i_dir_acl = u32::from_le_bytes(data[108..112].try_into().unwrap());
        let i_faddr = u32::from_le_bytes(data[112..116].try_into().unwrap());
        let i_osd2: [u8; 12] = data[116..128].try_into().unwrap();

        Some(Inode {
            i_mode,
            i_uid,
            i_size,
            i_atime,
            i_ctime,
            i_mtime,
            i_dtime,
            i_gid,
            i_links_count,
            i_blocks,
            i_flags,
            i_osd1,
            i_block,
            i_generation,
            i_file_acl,
            i_dir_acl,
            i_faddr,
            i_osd2,
        })
    }

    fn i_mode(&self) -> u16 { self.i_mode}
    fn i_uid(&self) -> u16 { self.i_uid}
    fn i_size(&self) -> u32 { self.i_size}
    fn i_atime(&self) -> u32 { self.i_atime}
    fn i_ctime(&self) -> u32 { self.i_ctime}
    fn i_mtime(&self) -> u32 { self.i_mtime}
    fn i_dtime(&self) -> u32 { self.i_dtime}
    fn i_gid(&self) -> u16 { self.i_gid}
    fn i_links_count(&self) -> u16 { self.i_links_count}
    fn i_blocks(&self) -> u32 { self.i_blocks}
    fn i_flags(&self) -> u32 { self.i_flags}
    fn i_osd1(&self) -> u32 { self.i_osd1}
    fn i_block(&self) -> [u32; 15] { self.i_block}
    fn i_generation(&self) -> u32 { self.i_generation}
    fn i_file_acl(&self) -> u32 { self.i_file_acl}
    fn i_dir_acl(&self) -> u32 { self.i_dir_acl}
    fn i_faddr(&self) -> u32 { self.i_faddr}
    fn i_osd2(&self) -> [u8; 12] { self.i_osd2}

    fn print_parsed_info(&self) {
        println!("\x1b[32mParsed Inode Information:\x1b[0m");
        println!("Mode: {:#o}", self.i_mode());
        println!("User ID (UID): {}", self.i_uid());
        println!("File Size: {} bytes", self.i_size());
        println!("Access Time: {}", self.i_atime());
        println!("Creation Time: {}", self.i_ctime());
        println!("Modification Time: {}", self.i_mtime());
        println!("Deletion Time: {}", self.i_dtime());
        println!("Group ID (GID): {}", self.i_gid());
        println!("Hard Link Count: {}", self.i_links_count());
        println!("Blocks Allocated: {}", self.i_blocks());
        println!("Flags: {:#x}", self.i_flags());
        println!("OSD1: {:#x}", self.i_osd1());
        println!("Block Pointers: {:?}", self.i_block());
        println!("Generation (Version): {}", self.i_generation());
        println!("File ACL: {:#x}", self.i_file_acl());
        println!("Directory ACL: {:#x}", self.i_dir_acl());
        println!("Fragment Address: {:#x}", self.i_faddr());
        println!("OSD2: {:?}", self.i_osd2());
    }
    pub fn get_all_info(&self, inode_number: usize) -> String {
        format!(
            "Inode Information:\n\
            Inode Number: {} \n\
            File Mode: {:o}\n\
            User ID (UID): {}\n\
            File Size: {} bytes\n\
            Access Time: {}\n\
            Creation Time: {}\n\
            Modification Time: {}\n\
            Deletion Time: {}\n\
            Group ID (GID): {}\n\
            Hard Link Count: {}\n\
            Blocks Allocated: {}\n\
            File Flags: {:x}\n\
            OS-Dependent Value: {}\n\
            Block Pointers: {:?}\n\
            File Generation: {}\n\
            File ACL: {}\n\
            Directory ACL: {}\n\
            Fragment Address: {}\n\
            OS-Dependent Value: {:?}\n\
            \n\n\n",
            inode_number,
            self.i_mode(),
            self.i_uid(),
            self.i_size(),
            self.i_atime(),
            self.i_ctime(),
            self.i_mtime(),
            self.i_dtime(),
            self.i_gid(),
            self.i_links_count(),
            self.i_blocks(),
            self.i_flags(),
            self.i_osd1(),
            self.i_block(),
            self.i_generation(),
            self.i_file_acl(),
            self.i_dir_acl(),
            self.i_faddr(),
            self.i_osd2(),
        )
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

        // We need data_blocks_offsets for each group, to get the data blocks, that are located after Inode Table.
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

        // Save Inode Table
        let mut inode_table: Vec<Inode> = vec![];
        // Read the Inode Table
        for (_, descriptor) in block_group_descriptors.iter().enumerate() {
            let inode_table_offset = descriptor.bg_inode_table() * superblock.block_size();

            // Iterate through each inode in the inode table
            for inode_index in 0..superblock.inodes_per_group() {
                // Inode Structure - 128 bytes
                let inode_offset = inode_table_offset + inode_index * 128;
                // Buffer for a single inode
                let mut buffer = [0u8; 128];
                _device.seek(SeekFrom::Start(inode_offset as u64))?;
                _device.read_exact(&mut buffer)?;

                if let Some(inode) = Inode::new(&buffer) {
                    inode_table.push(inode);
                }
            }
        }

        println!("inode table len: {}", inode_table.len());

        for (_, inode) in inode_table.iter().enumerate() {
            inode.print_parsed_info();
        }

        Ok(
            Ext2FS {
                super_block: superblock,
                block_group_descriptors,
                inode_table,
                block_bitmaps,
                data_blocks_offsets
            }
        )
    }

    /// Creates the debug_os_info folder and generates the .txt files
    pub fn create_debug_os_info(&self) -> io::Result<()> {
        // Create debug_os_info directory if it doesn't exist
        let folder_path = "debug_os_info";

        // Check if the folder exists
        if fs::metadata(folder_path).is_ok() {
            // Remove the folder and its contents
            fs::remove_dir_all(folder_path)?;
        }

        fs::create_dir(&folder_path)?;

        // superblock_info

        // Create a .txt file in the folder and write data to it
        let file_path = format!("{}/superblock_info.txt", folder_path);
        let mut file = File::create(&file_path)?;

        // Write data to the file
        file.write_all(self.super_block.get_all_info().as_bytes())?;



        // block_group_descriptors_info

        // Create a .txt file in the folder and write data to it
        let file_path = format!("{}/block_group_descriptors_info.txt", folder_path);
        // Open the file in append mode, creating it if it doesn't exist
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;
        for (i, descriptor) in self.block_group_descriptors.iter().enumerate() {
            let info = descriptor.get_all_info(i); // Get the descriptor information
            file.write_all(info.as_bytes())?;      // Write to the file
        }


        // inode_table_info

        // Create a .txt file in the folder and write data to it
        let file_path = format!("{}/inode_table_info.txt", folder_path);
        // Open the file in append mode, creating it if it doesn't exist
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;
        for (i, inode) in self.inode_table.iter().enumerate() {
            let info = inode.get_all_info(i); // Get the descriptor information
            file.write_all(info.as_bytes())?;      // Write to the file
        }

        Ok(())
    }
}

impl<'a> BlockIter<'a> {
    pub fn new(ext2_fs: &'a Ext2FS) -> Self {
        BlockIter {
            ext2_fs,
            current_group: 0,
            current_byte: 0,
            current_bit: 0,
            block_number: 0
        }
    }
}
impl Iterator for BlockIter<'_> {
    type Item = (usize, usize, bool); // (Group Number, Block Number, Is Used)

    fn next(&mut self) -> Option<Self::Item> {
        // Check every group in block_bitmaps, we need every unused block
        while self.current_group < self.ext2_fs.block_bitmaps.len() {
            // Extract group from vector
            let group_bitmap = &self.ext2_fs.block_bitmaps[self.current_group];
            // We can skip block, what are used for metadata:
            // superblock;
            // block group descriptor table;
            // block bitmap;
            // inode bitmap;
            // inode table;
            // We need -> data blocks
            let blocks_to_skip = self.ext2_fs.data_blocks_offsets[self.current_group];

            // Check every byte in group
            while self.current_byte < group_bitmap.len() {
                // Extract byte from vector
                let group_byte = group_bitmap[self.current_byte];

                // Extract every bit from byte
                while self.current_bit < 8 {
                    // Calculate current block number
                    let current_block_number = self.block_number + 1;


                    // Skip blocks with metadata
                    if current_block_number <= blocks_to_skip as usize {
                        self.block_number += 1;
                        self.current_bit += 1;
                        continue;
                    }

                    // Extract the bit at current_bit
                    let bit = (group_byte >> self.current_bit) & 1;


                    // Next bit
                    self.block_number += 1;
                    self.current_bit += 1;

                    // Return the current block info
                    return Some((
                        self.current_group + 1,    // Block group number (1-based)
                        current_block_number,     // Block number
                        bit == 1,                 // Is Used
                        ));
                }

                // Next byte
                self.current_bit = 0;
                self.current_byte += 1;
            }

            // Next group
            self.current_byte = 0;
            self.current_group += 1;
        }
        None
    }
}
