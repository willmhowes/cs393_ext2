use ext2::ext2::structs::TypePerm;
use ext2::ext2::Ext2;
use rustyline::{DefaultEditor, Result};
use std::f32::consts::E;

fn main() -> Result<()> {
    let disk = include_bytes!("../myfs.ext2");
    let start_addr: usize = disk.as_ptr() as usize;
    let ext2 = Ext2::new(&disk[..], start_addr);

    let mut current_working_inode: usize = 2;

    let mut rl = DefaultEditor::new()?;
    loop {
        // fetch the children of the current working directory
        let dirs = match ext2.read_dir_inode(current_working_inode) {
            Ok(dir_listing) => dir_listing,
            Err(_) => {
                println!("unable to read cwd, top");
                break;
            }
        };

        let buffer = rl.readline(":> ");
        if let Ok(line) = buffer {
            if line.starts_with("ls") {
                // `ls` prints our cwd's children
                // TODO: support arguments to ls (print that directory's children instead)
                let elts: Vec<&str> = line.split(' ').collect();
                if elts.len() == 1 {
                    for dir in &dirs {
                        print!("{}\t", dir.1);
                    }
                    println!();
                } else {
                    let desired_dir = match ext2.follow_path(elts[1], dirs) {
                        Ok(dir_listing) => dir_listing,
                        Err(_) => {
                            println!("unable to read dir_listing");
                            break;
                        }
                    };
                    // TODO: maybe don't just assume this is a directory
                    let dirs = match ext2.read_dir_inode(desired_dir) {
                        Ok(dir_listing) => dir_listing,
                        Err(_) => {
                            println!("unable to read cwd");
                            break;
                        }
                    };
                    for dir in &dirs {
                        print!("{}\t", dir.1);
                    }
                    println!();
                }
            } else if line.starts_with("cd") {
                // `cd` with no arguments, cd goes back to root
                // `cd dir_name` moves cwd to that directory
                let elts: Vec<&str> = line.split(' ').collect();
                if elts.len() == 1 {
                    current_working_inode = 2;
                } else {
                    // TODO: if the argument is a path, follow the path
                    // e.g., cd dir_1/dir_2 should move you down 2 directories
                    // deeper into dir_2
                    let to_dir = elts[1];
                    let mut found = false;
                    for dir in &dirs {
                        if dir.1.to_string().eq(to_dir) {
                            // TODO: maybe don't just assume this is a directory
                            found = true;
                            let candidate_inode = ext2.get_inode(dir.0);
                            if candidate_inode.type_perm & TypePerm::DIRECTORY
                                != TypePerm::DIRECTORY
                            {
                                println!("cannot cd into a file");
                            } else {
                                current_working_inode = dir.0;
                            }
                        }
                    }
                    if !found {
                        println!("unable to locate {}, cwd unchanged", to_dir);
                    }
                }
            } else if line.starts_with("mkdir") {
                // `mkdir childname`
                // create a directory with the given name, add a link to cwd
                // consider supporting `-p path/to_file` to create a path of directories
                println!("mkdir not yet implemented");
            } else if line.starts_with("cat") {
                // TODO: Need to finish
                // `cat filename`
                // print the contents of filename to stdout
                // if it's a directory, print a nice error
                let elts: Vec<&str> = line.split(' ').collect();
                if elts.len() == 1 {
                    println!("no argument provided");
                } else {
                    // TODO: if the argument is a path, follow the path
                    // e.g., cd dir_1/dir_2 should move you down 2 directories
                    // deeper into dir_2
                    let filename = elts[1];
                    let mut found = false;
                    println!("DIRS: {:?}", dirs);
                    for dir in &dirs {
                        if dir.1.to_string().trim_end().eq(filename) {
                            // TODO: maybe don't just assume this is a directory
                            found = true;
                            let arg_inode = dir.0;
                            let file = ext2.get_inode(arg_inode);
                            let file_size = file.size_low;

                            let mut running_file_size: u32 = 0;
                            // Scan through direct block pointers
                            let mut i: usize = 0;
                            while i < 12 && running_file_size < file_size {
                                if file.direct_pointer[i] != 0 {
                                    // Locate data block
                                    let block = ext2.blocks
                                        [file.direct_pointer[i] as usize - ext2.block_offset];
                                    // Print data block as utf8
                                    print!("{}", std::str::from_utf8(block).unwrap());
                                } else {
                                    print!("...");
                                };
                                running_file_size += ext2.block_size as u32;
                                i += 1;
                            }

                            // Scan through indirect block pointers
                            let mut i: usize = 0;
                            if file.indirect_pointer != 0 {
                                // Locate indirect block
                                let mut indirect_block =
                                    ext2.blocks[file.indirect_pointer as usize - ext2.block_offset];
                                // We divide by 4 because each entry is 32 bits, or 4 bytes
                                let num_ptrs_in_indirect = ext2.block_size / 4;
                                while i < num_ptrs_in_indirect && running_file_size < file_size {
                                    // grab four bytes that represent the pointer and check to see
                                    // if it is zero or not
                                    let (int_bytes, rest) =
                                        indirect_block.split_at(std::mem::size_of::<u32>());
                                    indirect_block = rest;
                                    let file_pointer: u32 =
                                        u32::from_le_bytes(int_bytes.try_into().unwrap());
                                    if file_pointer != 0 {
                                        let block =
                                            ext2.blocks[file_pointer as usize - ext2.block_offset];
                                        print!("{}", std::str::from_utf8(block).unwrap());
                                    } else {
                                        print!("...");
                                    };
                                    running_file_size += ext2.block_size as u32;
                                    i += 1;
                                } // Locate indirect block
                            }

                            // Scan through doubly-indirect block pointers
                            let mut i: usize = 0;
                            if file.doubly_indirect != 0 {
                                // Locate doubly indirect block
                                let mut doubly_indirect =
                                    ext2.blocks[file.doubly_indirect as usize - ext2.block_offset];
                                let num_ptrs_in_doubly_indirect = ext2.block_size / 4;
                                while i < num_ptrs_in_doubly_indirect
                                    && running_file_size < file_size
                                {
                                    // grab four bytes that represent the pointer and check to see
                                    // if it is zero or not
                                    let (int_bytes, rest) =
                                        doubly_indirect.split_at(std::mem::size_of::<u32>());
                                    doubly_indirect = rest;

                                    let indirect_block_pointer: u32 =
                                        u32::from_le_bytes(int_bytes.try_into().unwrap());

                                    // navigate singly indirect blocks
                                    let mut j: usize = 0;
                                    if indirect_block_pointer != 0 {
                                        let mut indirect_block = ext2.blocks
                                            [indirect_block_pointer as usize - ext2.block_offset];
                                        // We divide by 4 because each entry is 32 bits, or 4 bytes
                                        let num_ptrs_in_indirect = ext2.block_size / 4;
                                        while j < num_ptrs_in_indirect
                                            && running_file_size < file_size
                                        {
                                            // grab four bytes that represent the pointer and check to see
                                            // if it is zero or not
                                            let (int_bytes, rest) =
                                                indirect_block.split_at(std::mem::size_of::<u32>());
                                            indirect_block = rest;
                                            let file_pointer: u32 =
                                                u32::from_le_bytes(int_bytes.try_into().unwrap());
                                            if file_pointer != 0 {
                                                let block = ext2.blocks
                                                    [file_pointer as usize - ext2.block_offset];
                                                print!("{}", std::str::from_utf8(block).unwrap());
                                            } else {
                                                print!("...");
                                            };
                                            running_file_size += ext2.block_size as u32;
                                            j += 1;
                                        }
                                    } else {
                                        print!("...");
                                    };
                                    i += 1;
                                }
                            }

                            // Scan through triply indirect block pointers
                            let mut z: usize = 0;
                            if file.triply_indirect != 0 {
                                // Locate triply indirect block
                                let mut triply_indirect =
                                    ext2.blocks[file.triply_indirect as usize - ext2.block_offset];
                                // We divide by 4 because each entry is 32 bits, or 4 bytes
                                let num_ptrs_in_triply_indirect = ext2.block_size / 4;
                                while z < num_ptrs_in_triply_indirect
                                    && running_file_size < file_size
                                {
                                    // grab four bytes that represent the pointer and check to see
                                    // if it is zero or not
                                    let (int_bytes, rest) =
                                        triply_indirect.split_at(std::mem::size_of::<u32>());
                                    triply_indirect = rest;
                                    let doubly_indirect_file_pointer: u32 =
                                        u32::from_le_bytes(int_bytes.try_into().unwrap());

                                    // Scan through doubly-indirect block pointers
                                    let mut i: usize = 0;
                                    if doubly_indirect_file_pointer != 0 {
                                        // Locate doubly indirect block
                                        let mut doubly_indirect = ext2.blocks
                                            [doubly_indirect_file_pointer as usize
                                                - ext2.block_offset];
                                        let num_ptrs_in_doubly_indirect = ext2.block_size / 4;
                                        while i < num_ptrs_in_doubly_indirect
                                            && running_file_size < file_size
                                        {
                                            // grab four bytes that represent the pointer and check to see
                                            // if it is zero or not
                                            let (int_bytes, rest) = doubly_indirect
                                                .split_at(std::mem::size_of::<u32>());
                                            doubly_indirect = rest;

                                            let indirect_block_pointer: u32 =
                                                u32::from_le_bytes(int_bytes.try_into().unwrap());

                                            // navigate singly indirect blocks
                                            let mut j: usize = 0;
                                            if indirect_block_pointer != 0 {
                                                let mut indirect_block = ext2.blocks
                                                    [indirect_block_pointer as usize
                                                        - ext2.block_offset];
                                                // We divide by 4 because each entry is 32 bits, or 4 bytes
                                                let num_ptrs_in_indirect = ext2.block_size / 4;
                                                while j < num_ptrs_in_indirect
                                                    && running_file_size < file_size
                                                {
                                                    // grab four bytes that represent the pointer and check to see
                                                    // if it is zero or not
                                                    let (int_bytes, rest) = indirect_block
                                                        .split_at(std::mem::size_of::<u32>());
                                                    indirect_block = rest;
                                                    let file_pointer: u32 = u32::from_le_bytes(
                                                        int_bytes.try_into().unwrap(),
                                                    );
                                                    if file_pointer != 0 {
                                                        let block = ext2.blocks[file_pointer
                                                            as usize
                                                            - ext2.block_offset];
                                                        print!(
                                                            "{}",
                                                            std::str::from_utf8(block).unwrap()
                                                        );
                                                    } else {
                                                        print!("...");
                                                    };
                                                    running_file_size += ext2.block_size as u32;
                                                    j += 1;
                                                }
                                            } else {
                                                print!("...");
                                            };
                                            i += 1;
                                        }
                                    } else {
                                        print!("...");
                                    };
                                    z += 1;
                                }
                            }

                            println!("\nrunning_file_size = {running_file_size}");
                            println!("FILESIZE = {}", file_size);
                        }
                    }
                    if !found {
                        println!("unable to locate {}", filename);
                    }
                }
            } else if line.starts_with("rm") {
                // `rm target`
                // unlink a file or empty directory
                println!("rm not yet implemented");
            } else if line.starts_with("mount") {
                // `mount host_filename mountpoint`
                // mount an ext2 filesystem over an existing empty directory
                println!("mount not yet implemented");
            } else if line.starts_with("link") {
                // `link arg_1 arg_2`
                // create a hard link from arg_1 to arg_2
                // consider what to do if arg2 does- or does-not end in "/"
                // and/or if arg2 is an existing directory name
                println!("link not yet implemented");
            } else if line.starts_with("quit") || line.starts_with("exit") {
                break;
            }
        } else {
            println!("bye!");
            break;
        }
    }
    Ok(())
}
