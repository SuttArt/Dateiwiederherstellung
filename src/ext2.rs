
// implementation template for convenience
// see https://www.nongnu.org/ext2-doc/ext2.html for documentation on ext2 fs

#[derive(Debug)]
pub struct Ext2FS {
	// TODO: fill this structure
}

pub struct BlockIter {
	// TODO: fill this structure
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct Superblock {
	// TODO: fill this structure
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct BlockGroupDescriptor {
	// TODO: fill this structure
}

impl Superblock {
	// TODO: add useful methods
}

impl BlockGroupDescriptor {
	// TODO: add useful methods
}

impl Ext2FS {
	// TODO: add useful methods
}

impl Iterator for BlockIter {
	type Item = (); 
	
	fn next(&mut self) -> Option<()> {
		// TODO: implement an iterator over unused blocks
		None
	}
}
