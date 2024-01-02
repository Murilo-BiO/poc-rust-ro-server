use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Cursor, Read, Seek},
    path::{Path, PathBuf},
};

const HEADER_SIZE: usize = 46;
const HEADER_MAGIC_STRING: &str = "Master of Magic\0";
const SUPPORTED_VERSION: u32 = 0x200;

#[derive(Debug)]
pub enum GrfError {
    InvalidData(String),
    DecompressionError(String),
    UnsupportedVersion(u32),
    Other(io::Error),
}

impl From<io::Error> for GrfError {
    fn from(error: io::Error) -> Self {
        GrfError::Other(error)
    }
}

impl From<yazi::Error> for GrfError {
    fn from(error: yazi::Error) -> Self {
        GrfError::DecompressionError(format!("Failed decompression: {:?}", error))
    }
}

#[derive(Default, Debug)]
pub struct GrfHeader {
    pub encription_key: String,
    pub file_table_offset: u32,
    pub seed: u32,
    pub files_count: u32,
    pub version: u32,
}

impl GrfHeader {
    pub fn from_bytes(fd: &mut File) -> Result<Self, GrfError> {
        let mut header = Self::default();

        fd.by_ref().take(14).read_to_string(&mut header.encription_key)?;

        header.file_table_offset = fd.read_u32::<LittleEndian>()?;
        header.seed = fd.read_u32::<LittleEndian>()?;
        header.files_count = fd.read_u32::<LittleEndian>()?;
        header.version = fd.read_u32::<LittleEndian>()?;

        if header.version != SUPPORTED_VERSION {
            return Err(GrfError::UnsupportedVersion(header.version));
        }

        Ok(header)
    }
}

#[derive(Default, Debug)]
pub struct GrfFileEntry {
    pub file_name: String,
    pub file_name_bytes: Vec<u8>,
    pub compressed_size: u32,
    pub compressed_size_aligned: u32,
    pub uncompressed_size: u32,
    pub flags: u8,
    pub offset: u32,
}

#[derive(Default, Debug)]
pub struct GrfFileTable {
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub files: HashMap<PathBuf, GrfFileEntry>,
}

impl GrfFileTable {
    pub fn from_bytes(fd: &mut File, header: &GrfHeader) -> Result<Self, GrfError> {
        let mut files_table = Self::default();

        fd.seek(io::SeekFrom::Current(header.file_table_offset as i64))?;

        files_table.compressed_size = fd.read_u32::<LittleEndian>()?;
        files_table.uncompressed_size = fd.read_u32::<LittleEndian>()?;

        let mut rdr = {
            let mut buf = vec![0_u8; files_table.compressed_size as usize];
            fd.read_exact(&mut buf)?;

            let (buf, _) = yazi::decompress(&buf, yazi::Format::Zlib)?;

            Cursor::new(buf)
        };

        while rdr.get_ref().len() >= rdr.position() as usize {
            let file_name_opt = Self::read_file_name(&mut rdr);

            if let None = file_name_opt {
                break;
            }

            let mut file_entry = GrfFileEntry::default();

            let (file_name, file_name_bytes) = file_name_opt.unwrap();
            file_entry.file_name = file_name;
            file_entry.file_name_bytes = file_name_bytes;

            file_entry.compressed_size = rdr.read_u32::<LittleEndian>()?;
            file_entry.compressed_size_aligned = rdr.read_u32::<LittleEndian>()?;
            file_entry.uncompressed_size = rdr.read_u32::<LittleEndian>()?;
            file_entry.flags = rdr.read_u8()?;
            file_entry.offset = rdr.read_u32::<LittleEndian>()?;

            let file_path = file_entry.file_name.replace("\\", "/");

            files_table
                .files
                .insert(Path::new(&file_path).to_path_buf(), file_entry);
        }

        Ok(files_table)
    }

    fn read_file_name<R: Read>(reader: &mut R) -> Option<(String, Vec<u8>)> {
        let mut string_bytes = Vec::new();

        reader
            .by_ref()
            .bytes()
            .take_while(|byte| match byte {
                Ok(byte) => *byte != 0x00_u8,
                Err(_) => false,
            })
            .for_each(|byte| string_bytes.push(byte.unwrap()));

        let file_name: String = string_bytes.iter().map(|&c| c as char).collect();

        if file_name == "" {
            return None;
        }

        Some((file_name, string_bytes))
    }
}

#[derive(Debug)]
pub struct Grf {
    pub file_handle: File,
    pub header: GrfHeader,
    pub files_table: GrfFileTable,
}

impl Grf {
    pub fn from_path(path: &Path) -> Result<Self, GrfError> {
        let mut fd = File::open(path)?;

        if HEADER_SIZE >= fd.metadata()?.len() as usize  {
            return Err(GrfError::InvalidData(
                "Not a valid GRF file. File is smaller than the header size.".to_string(),
            ));
        }

        Self::validate_magic_header(&mut fd)?;

        let header = GrfHeader::from_bytes(&mut fd)?;
        let files_table = GrfFileTable::from_bytes(&mut fd, &header)?;

        Ok(Self {
            file_handle: fd,
            header,
            files_table,
        })
    }

    fn validate_magic_header(fd: &mut File) -> Result<(), GrfError> {
        let mut buf = [0_u8; HEADER_MAGIC_STRING.len()];
        fd.read_exact(&mut buf)?;

        if buf != HEADER_MAGIC_STRING.as_bytes() {
            return Err(GrfError::InvalidData(
                "Invalid GRF File. Magic Header not present.".to_string(),
            ));
        }

        Ok(())
    }

    pub fn get_file_from_path(&mut self, path: &Path) -> Result<(&GrfFileEntry, Cursor<Vec<u8>>), GrfError> {
        let file_entry = self.files_table
            .files
            .get(&path.to_path_buf())
            .ok_or_else(|| GrfError::Other(io::Error::new(
                io::ErrorKind::Other,
                format!("File not found in the file table '{}'", path.display()),
            )))?;

        self.file_handle.seek(io::SeekFrom::Start(file_entry.offset as u64 + HEADER_SIZE as u64))?;
        let reader = {
            let mut buf = vec![0_u8; file_entry.compressed_size as usize];
            self.file_handle.read_exact(&mut buf)?;

            let (buf, _) = yazi::decompress(&buf, yazi::Format::Zlib)?;

            Cursor::new(buf)
        };

        Ok((&file_entry, reader))
    }
}
