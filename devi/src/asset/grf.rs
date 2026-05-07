use std::fs::File;
use std::io::{Read, Seek, SeekFrom, BufReader};
use std::path::Path;
use flate2::read::ZlibDecoder;

use super::AssetError;

/// GRF 文件头魔术字节
const GRF_MAGIC: &[u8; 16] = b"Master of Magic\0";

/// GRF 文件条目
#[derive(Debug, Clone)]
pub struct GrfEntry {
    /// 压缩后大小（字节）
    pub compressed_size: u32,
    /// 压缩前大小（字节）
    pub real_size: u32,
    /// 文件类型（0=普通文件，1=目录）
    pub entry_type: u8,
    /// 文件在 GRF 中的偏移（从 GRF 头部算起）
    pub offset: u32,
    /// 文件路径（小写，用 '\\' 分隔）
    pub path: String,
}

/// GRF 文件归档
pub struct GrfArchive {
    reader: BufReader<File>,
    entries: Vec<GrfEntry>,
}

impl GrfArchive {
    /// 打开 GRF 文件
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, AssetError> {
        let file = File::open(path.as_ref()).map_err(|_| {
            AssetError::NotFound(path.as_ref().to_string_lossy().to_string())
        })?;
        let mut reader = BufReader::new(file);

        // 读取魔术字节
        let mut magic = [0u8; 16];
        reader.read_exact(&mut magic)?;
        if &magic != GRF_MAGIC {
            return Err(AssetError::ParseError(
                "无效的 GRF 文件：魔术字节不匹配".to_string()
            ));
        }

        // 读取文件头
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        let _flags = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let _grf_size = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let entry_table_offset = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let entry_count_raw = u32::from_le_bytes(buf);

        // 计算实际文件条目数（参考 rAthena：count - count0 - 7）
        let entry_count = entry_count_raw.saturating_sub(7);

        // 跳转到条目表（偏移相对于文件头结束后的 30 字节）
        reader.seek(SeekFrom::Start(30 + entry_table_offset as u64))?;

        // 读取所有条目
        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let entry = Self::read_entry(&mut reader)?;
            entries.push(entry);
        }

        Ok(Self { reader, entries })
    }

    /// 读取单个文件条目
    fn read_entry(reader: &mut BufReader<File>) -> Result<GrfEntry, AssetError> {
        let mut buf = [0u8; 17];
        if reader.read(&mut buf)? < 17 {
            return Err(AssetError::ParseError("GRF 条目数据不完整".to_string()));
        }

        // 读取变长文件名（以 null 结尾）
        let mut name_buf = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            if reader.read(&mut byte)? == 0 { break; }
            if byte[0] == 0 { break; }
            name_buf.push(byte[0]);
        }

        let path = String::from_utf8_lossy(&name_buf)
            .to_string()
            .replace('\\', "/")
            .to_lowercase();

        Ok(GrfEntry {
            compressed_size: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
            real_size: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
            entry_type: buf[8],
            offset: u32::from_le_bytes([buf[9], buf[10], buf[11], buf[12]]),
            path,
        })
    }

    /// 获取文件条目列表
    pub fn entries(&self) -> &[GrfEntry] {
        &self.entries
    }

    /// 按路径查找文件条目
    pub fn find_entry(&self, path: &str) -> Option<&GrfEntry> {
        let normalized = path.replace('\\', "/").to_lowercase();
        self.entries.iter().find(|e| e.path == normalized)
    }

    /// 读取文件内容（自动解压）
    pub fn read_file(&mut self, entry: &GrfEntry) -> Result<Vec<u8>, AssetError> {
        let abs_offset = 30 + entry.offset as u64;
        self.reader.seek(SeekFrom::Start(abs_offset))?;

        let mut compressed = vec![0u8; entry.compressed_size as usize];
        self.reader.read_exact(&mut compressed)?;

        if entry.compressed_size == entry.real_size {
            return Ok(compressed);
        }

        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::with_capacity(entry.real_size as usize);
        decoder.read_to_end(&mut decompressed)?;

        if decompressed.len() != entry.real_size as usize {
            return Err(AssetError::ParseError(format!(
                "解压大小不匹配：期望 {} 字节，实际 {} 字节",
                entry.real_size, decompressed.len()
            )));
        }

        Ok(decompressed)
    }
}
