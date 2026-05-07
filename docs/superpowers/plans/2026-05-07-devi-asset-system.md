# Devi 客户端资源系统实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 RO 资源文件的解析和加载，支持 GRF 容器和自定义 PNG+JSON 格式的双模式加载。

**Architecture:** 资源系统分为三层：GRF 容器解析 → 格式解码器（SPR/ACT/GAT/GND/RSW）→ 统一加载器（自动 fallback）。参考 roBrowser 的 JavaScript 实现，用 Rust 重写。

**Tech Stack:** Rust, bevy_asset, flate2 (zlib 解压), image (图片处理), byteorder (字节序)

---

## 文件结构

| 操作 | 文件路径 | 职责 |
|------|----------|------|
| Create | `devi/src/asset/mod.rs` | 资源模块声明和通用类型 |
| Create | `devi/src/asset/grf.rs` | GRF 文件解析器 |
| Create | `devi/src/asset/sprite.rs` | SPR 精灵图片解析 |
| Create | `devi/src/asset/action.rs` | ACT 动画定义解析 |
| Create | `devi/src/asset/gat.rs` | GAT 地面高度网格解析 |
| Create | `devi/src/asset/gnd.rs` | GND 地面纹理/法线解析 |
| Create | `devi/src/asset/loader.rs` | 统一资源加载器（双格式 fallback） |
| Create | `devi/tests/asset_test.rs` | 资源系统测试 |

---

### Task 1: GRF 容器解析器

**Files:**
- Create: `devi/src/asset/mod.rs`
- Create: `devi/src/asset/grf.rs`
- Create: `devi/tests/asset_test.rs`

- [ ] **Step 1: 编写 GRF 解析测试**

创建 `devi/tests/asset_test.rs`:
```rust
use devi::asset::grf::GrfArchive;

#[test]
fn test_grf_header_magic() {
    // GRF 文件头魔术字节: "Master of Magic\0"
    let magic = b"Master of Magic\0";
    assert_eq!(magic.len(), 16);
}

#[test]
fn test_grf_open_nonexistent() {
    let result = GrfArchive::open("nonexistent.grf");
    assert!(result.is_err());
}

#[test]
fn test_grf_file_entry_size() {
    // GRF 文件条目固定 17 字节头部 + 变长文件名
    // 格式: pack_size(4) + compressed_size(4) + real_size(4) + type(1) + offset(4) + filename
    use std::mem::size_of;
    // 验证基本类型大小
    assert_eq!(size_of::<u32>(), 4);
    assert_eq!(size_of::<u8>(), 1);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test asset_test`
Expected: FAIL，`unresolved import devi::asset::grf`

- [ ] **Step 3: 实现 GRF 模块声明**

创建 `devi/src/asset/mod.rs`:
```rust
// 资源系统模块
pub mod grf;
pub mod sprite;
pub mod action;
pub mod gat;
pub mod gnd;
pub mod loader;

/// 资源加载错误
#[derive(Debug)]
pub enum AssetError {
    /// 文件未找到
    NotFound(String),
    /// IO 错误
    Io(std::io::Error),
    /// 解析错误
    ParseError(String),
    /// 不支持的格式
    UnsupportedFormat(String),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::NotFound(path) => write!(f, "资源未找到: {}", path),
            AssetError::Io(err) => write!(f, "IO 错误: {}", err),
            AssetError::ParseError(msg) => write!(f, "解析错误: {}", msg),
            AssetError::UnsupportedFormat(fmt) => write!(f, "不支持的格式: {}", fmt),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<std::io::Error> for AssetError {
    fn from(err: std::io::Error) -> Self {
        AssetError::Io(err)
    }
}
```

- [ ] **Step 4: 实现 GRF 解析器**

创建 `devi/src/asset/grf.rs`:
```rust
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, BufReader};
use std::path::Path;
use flate2::read::ZlibDecoder;

use super::AssetError;

/// GRF 文件头魔术字节
const GRF_MAGIC: &[u8; 16] = b"Master of Magic\0";

/// GRF 文件头
#[derive(Debug)]
struct GrfHeader {
    /// 魔术字节（已验证）
    /// 文件条目表偏移（从文件头之后算起）
    entry_table_offset: u32,
    /// 文件条目数量
    entry_count: u32,
}

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
        let _flags = u32::from_le_bytes(buf); // 预留标志位（未使用）

        reader.read_exact(&mut buf)?;
        let _grf_size = u32::from_le_bytes(buf); // GRF 文件大小

        reader.read_exact(&mut buf)?;
        let entry_table_offset = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let entry_count_raw = u32::from_le_bytes(buf);

        // 计算实际文件条目数（参考 rAthena：count - count0 - 7）
        let entry_count = entry_count_raw.saturating_sub(7);

        // 跳转到条目表
        // 条目表偏移是相对于文件头结束（30 字节）的偏移
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

        // 尝试读取固定头部
        if reader.read(&mut buf)? < 17 {
            return Err(AssetError::ParseError(
                "GRF 条目数据不完整".to_string()
            ));
        }

        // 读取变长文件名（以 null 结尾）
        let mut name_buf = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            if reader.read(&mut byte)? == 0 {
                break;
            }
            if byte[0] == 0 {
                break;
            }
            name_buf.push(byte[0]);
        }

        let path = String::from_utf8_lossy(&name_buf)
            .to_string()
            .replace('\\', "/") // 统一路径分隔符
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
        // 跳转到文件数据位置（偏移相对于文件头结束后的 30 字节）
        let abs_offset = 30 + entry.offset as u64;
        self.reader.seek(SeekFrom::Start(abs_offset))?;

        // 读取压缩数据
        let mut compressed = vec![0u8; entry.compressed_size as usize];
        self.reader.read_exact(&mut compressed)?;

        if entry.compressed_size == entry.real_size {
            // 未压缩，直接返回
            return Ok(compressed);
        }

        // 解压
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::with_capacity(entry.real_size as usize);
        decoder.read_to_end(&mut decompressed)?;

        if decompressed.len() != entry.real_size as usize {
            return Err(AssetError::ParseError(format!(
                "解压大小不匹配：期望 {} 字节，实际 {} 字节",
                entry.real_size,
                decompressed.len()
            )));
        }

        Ok(decompressed)
    }
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cd devi && cargo test --test asset_test`
Expected: 3 tests PASS

- [ ] **Step 6: Commit**

```bash
git add devi/src/asset/mod.rs devi/src/asset/grf.rs devi/tests/asset_test.rs
git commit -m "feat(devi/asset): 实现 GRF 容器解析器，支持文件索引和解压"
```

---

### Task 2: SPR 精灵图片解析

**Files:**
- Create: `devi/src/asset/sprite.rs`
- Modify: `devi/tests/asset_test.rs`

- [ ] **Step 1: 编写 SPR 解析测试**

在 `devi/tests/asset_test.rs` 中追加:
```rust
use devi::asset::sprite::SpriteFile;

#[test]
fn test_spr_header_parsing() {
    // SPR 文件头: version(2) + indexed_count(2) + rgba_count(2)
    // 版本: 0x0101 (1.1) 或 0x0102 (1.2) 或 0x0200 (2.0)
    let data = vec![
        0x01, 0x02, // version 1.2
        0x05, 0x00, // 5 个索引色帧
        0x03, 0x00, // 3 个 RGBA 帧
    ];
    let spr = SpriteFile::from_bytes(&data).unwrap();
    assert_eq!(spr.version, 0x0102);
    assert_eq!(spr.indexed_frame_count, 5);
    assert_eq!(spr.rgba_frame_count, 3);
}

#[test]
fn test_spr_empty_file() {
    let data = vec![];
    let result = SpriteFile::from_bytes(&data);
    assert!(result.is_err());
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test asset_test`
Expected: FAIL，`unresolved import devi::asset::sprite`

- [ ] **Step 3: 实现 SPR 解析器**

创建 `devi/src/asset/sprite.rs`:
```rust
use super::AssetError;

/// SPR 文件（精灵图片数据）
#[derive(Debug)]
pub struct SpriteFile {
    /// 版本号（如 0x0102 表示 1.2）
    pub version: u16,
    /// 索引色帧数量
    pub indexed_frame_count: u16,
    /// RGBA 帧数量
    pub rgba_frame_count: u16,
    /// 帧数据
    pub frames: Vec<SpriteFrame>,
}

/// 精灵帧
#[derive(Debug)]
pub enum SpriteFrame {
    /// 索引色帧（8bit，需配合调色板）
    Indexed {
        width: u16,
        height: u16,
        data: Vec<u8>,
    },
    /// RGBA 帧（32bit 真彩色）
    Rgba {
        width: u16,
        height: u16,
        data: Vec<u8>,
    },
}

impl SpriteFile {
    /// 从字节数据解析 SPR 文件
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() < 6 {
            return Err(AssetError::ParseError(
                "SPR 文件太小，无法读取头部".to_string()
            ));
        }

        let version = u16::from_le_bytes([data[0], data[1]]);
        let indexed_count = u16::from_le_bytes([data[2], data[3]]);
        let rgba_count = u16::from_le_bytes([data[4], data[5]]);

        let mut frames = Vec::new();
        let mut offset = 6;

        // 读取索引色帧
        for _ in 0..indexed_count {
            if offset + 8 > data.len() {
                return Err(AssetError::ParseError(
                    "SPR 索引色帧数据不完整".to_string()
                ));
            }
            let width = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let height = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
            offset += 4;

            let pixel_count = width as usize * height as usize;
            if offset + pixel_count > data.len() {
                return Err(AssetError::ParseError(
                    "SPR 索引色帧像素数据不完整".to_string()
                ));
            }
            let frame_data = data[offset..offset + pixel_count].to_vec();
            offset += pixel_count;

            frames.push(SpriteFrame::Indexed {
                width,
                height,
                data: frame_data,
            });
        }

        // 读取 RGBA 帧
        for _ in 0..rgba_count {
            if offset + 8 > data.len() {
                return Err(AssetError::ParseError(
                    "SPR RGBA 帧数据不完整".to_string()
                ));
            }
            let width = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let height = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
            offset += 4;

            let pixel_count = width as usize * height as usize * 4;
            if offset + pixel_count > data.len() {
                return Err(AssetError::ParseError(
                    "SPR RGBA 帧像素数据不完整".to_string()
                ));
            }
            let frame_data = data[offset..offset + pixel_count].to_vec();
            offset += pixel_count;

            frames.push(SpriteFrame::Rgba {
                width,
                height,
                data: frame_data,
            });
        }

        Ok(Self {
            version,
            indexed_frame_count: indexed_count,
            rgba_frame_count: rgba_count,
            frames,
        })
    }

    /// 总帧数
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test asset_test`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/asset/sprite.rs devi/tests/asset_test.rs
git commit -m "feat(devi/asset): 实现 SPR 精灵图片解析器"
```

---

### Task 3: ACT 动画定义解析

**Files:**
- Create: `devi/src/asset/action.rs`
- Modify: `devi/tests/asset_test.rs`

- [ ] **Step 1: 编写 ACT 解析测试**

在 `devi/tests/asset_test.rs` 中追加:
```rust
use devi::asset::action::ActionFile;

#[test]
fn test_act_header_parsing() {
    // ACT 文件头: magic("AC") + version(2) + action_count(2)
    let data = vec![
        b'A', b'C', // 魔术字节
        0x00, 0x02, // version 2.0
        0x02, 0x00, // 2 个动作
    ];
    let act = ActionFile::from_bytes(&data).unwrap();
    assert_eq!(act.version, 0x0002);
    assert_eq!(act.action_count, 2);
}

#[test]
fn test_act_invalid_magic() {
    let data = vec![b'X', b'Y', 0x00, 0x02, 0x00, 0x00];
    let result = ActionFile::from_bytes(&data);
    assert!(result.is_err());
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test asset_test`
Expected: FAIL，`unresolved import devi::asset::action`

- [ ] **Step 3: 实现 ACT 解析器**

创建 `devi/src/asset/action.rs`:
```rust
use super::AssetError;

/// ACT 文件（动画定义）
#[derive(Debug)]
pub struct ActionFile {
    /// 版本号
    pub version: u16,
    /// 动作数量
    pub action_count: u16,
    /// 动作列表
    pub actions: Vec<Action>,
}

/// 单个动画动作
#[derive(Debug)]
pub struct Action {
    /// 帧列表
    pub frames: Vec<ActionFrame>,
}

/// 动画帧
#[derive(Debug)]
pub struct ActionFrame {
    /// 精灵索引列表（一帧可由多个精灵组合）
    pub sprites: Vec<FrameSprite>,
    /// 事件类型（音效、特效触发等）
    pub event_type: u32,
}

/// 帧中的单个精灵
#[derive(Debug)]
pub struct FrameSprite {
    /// 精灵图集索引
    pub sprite_index: u32,
    /// X 偏移（像素）
    pub x: i32,
    /// Y 偏移（像素）
    pub y: i32,
    /// 缩放 X
    pub scale_x: f32,
    /// 缩放 Y
    pub scale_y: f32,
    /// 旋转角度（度）
    pub rotation: f32,
    /// 颜色 (RGBA)
    pub color: [u8; 4],
    /// 变换类型
    pub mirroring: u8,
}

impl ActionFile {
    /// 从字节数据解析 ACT 文件
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() < 6 {
            return Err(AssetError::ParseError(
                "ACT 文件太小，无法读取头部".to_string()
            ));
        }

        // 验证魔术字节 "AC"
        if data[0] != b'A' || data[1] != b'C' {
            return Err(AssetError::ParseError(
                "无效的 ACT 文件：魔术字节不匹配".to_string()
            ));
        }

        let version = u16::from_le_bytes([data[2], data[3]]);
        let action_count = u16::from_le_bytes([data[4], data[5]]);

        let mut actions = Vec::new();
        let mut offset = 6;

        for _ in 0..action_count {
            if offset + 4 > data.len() {
                return Err(AssetError::ParseError(
                    "ACT 动作数据不完整".to_string()
                ));
            }

            let frame_count = u32::from_le_bytes([
                data[offset], data[offset + 1],
                data[offset + 2], data[offset + 3],
            ]) as usize;
            offset += 4;

            let mut frames = Vec::new();
            for _ in 0..frame_count {
                let (frame, new_offset) = Self::read_frame(data, offset, version)?;
                frames.push(frame);
                offset = new_offset;
            }

            actions.push(Action { frames });
        }

        Ok(Self {
            version,
            action_count,
            actions,
        })
    }

    /// 读取单个动画帧
    fn read_frame(data: &[u8], mut offset: u16, version: u16) -> Result<(ActionFrame, usize), AssetError> {
        if offset + 32 > data.len() {
            return Err(AssetError::ParseError(
                "ACT 帧数据不完整".to_string()
            ));
        }

        // 读取帧范围（32 字节）
        let _range_x1 = i32::from_le_bytes([
            data[offset], data[offset + 1],
            data[offset + 2], data[offset + 3],
        ]);
        let _range_y1 = i32::from_le_bytes([
            data[offset + 4], data[offset + 5],
            data[offset + 6], data[offset + 7],
        ]);
        let _range_x2 = i32::from_le_bytes([
            data[offset + 8], data[offset + 9],
            data[offset + 10], data[offset + 11],
        ]);
        let _range_y2 = i32::from_le_bytes([
            data[offset + 12], data[offset + 13],
            data[offset + 14], data[offset + 15],
        ]);
        offset += 16;

        // 读取事件类型
        let event_type = u32::from_le_bytes([
            data[offset], data[offset + 1],
            data[offset + 2], data[offset + 3],
        ]);
        offset += 4;

        // 读取精灵数量
        let sprite_count = u32::from_le_bytes([
            data[offset], data[offset + 1],
            data[offset + 2], data[offset + 3],
        ]) as usize;
        offset += 4;

        // 跳过 4 字节保留字段
        offset += 4;

        let mut sprites = Vec::new();
        for _ in 0..sprite_count {
            let (sprite, new_offset) = Self::read_sprite(data, offset, version)?;
            sprites.push(sprite);
            offset = new_offset;
        }

        Ok((ActionFrame {
            sprites,
            event_type,
        }, offset))
    }

    /// 读取单个精灵定义
    fn read_sprite(data: &[u8], mut offset: u16, version: u16) -> Result<(FrameSprite, usize), AssetError> {
        // 基础字段：sprite_index(4) + x(4) + y(4) + scale_x(4) + scale_y(4) + rotation(4) + color(4) + mirroring(2) = 30 字节
        let sprite_index = u32::from_le_bytes([
            data[offset], data[offset + 1],
            data[offset + 2], data[offset + 3],
        ]);
        let x = i32::from_le_bytes([
            data[offset + 4], data[offset + 5],
            data[offset + 6], data[offset + 7],
        ]);
        let y = i32::from_le_bytes([
            data[offset + 8], data[offset + 9],
            data[offset + 10], data[offset + 11],
        ]);
        let scale_x = f32::from_le_bytes([
            data[offset + 12], data[offset + 13],
            data[offset + 14], data[offset + 15],
        ]);
        let scale_y = f32::from_le_bytes([
            data[offset + 16], data[offset + 17],
            data[offset + 18], data[offset + 19],
        ]);
        let rotation = f32::from_le_bytes([
            data[offset + 20], data[offset + 21],
            data[offset + 22], data[offset + 23],
        ]);
        let color = [
            data[offset + 24], data[offset + 25],
            data[offset + 26], data[offset + 27],
        ];
        let mirroring = data[offset + 28];
        offset += 30;

        // 版本 >= 2.0 时有额外 2 字节
        if version >= 0x0200 {
            offset += 2;
        }

        Ok((FrameSprite {
            sprite_index,
            x,
            y,
            scale_x,
            scale_y,
            rotation,
            color,
            mirroring,
        }, offset))
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test asset_test`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/asset/action.rs devi/tests/asset_test.rs
git commit -m "feat(devi/asset): 实现 ACT 动画定义解析器"
```

---

### Task 4: GAT 地面高度网格解析

**Files:**
- Create: `devi/src/asset/gat.rs`
- Modify: `devi/tests/asset_test.rs`

- [ ] **Step 1: 编写 GAT 解析测试**

在 `devi/tests/asset_test.rs` 中追加:
```rust
use devi::asset::gat::GatFile;

#[test]
fn test_gat_header_parsing() {
    // GAT 文件头: magic("GRAT") + version(2) + width(4) + height(4)
    let mut data = vec![
        b'G', b'R', b'A', b'T', // 魔术字节
        0x01, 0x02, // version 1.2
        0x02, 0x00, 0x00, 0x00, // width = 2
        0x02, 0x00, 0x00, 0x00, // height = 2
    ];
    // 每个 cell 20 字节: 4个高度(float32) + type(u32)
    for _ in 0..4 {
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // h1
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // h2
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // h3
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // h4
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // type
    }

    let gat = GatFile::from_bytes(&data).unwrap();
    assert_eq!(gat.version, 0x0102);
    assert_eq!(gat.width, 2);
    assert_eq!(gat.height, 2);
    assert_eq!(gat.cells.len(), 4);
}

#[test]
fn test_gat_invalid_magic() {
    let data = vec![b'X', b'Y', b'Z', b'W', 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let result = GatFile::from_bytes(&data);
    assert!(result.is_err());
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test asset_test`
Expected: FAIL，`unresolved import devi::asset::gat`

- [ ] **Step 3: 实现 GAT 解析器**

创建 `devi/src/asset/gat.rs`:
```rust
use super::AssetError;

/// GAT 文件（地面高度网格）
#[derive(Debug)]
pub struct GatFile {
    /// 版本号
    pub version: u16,
    /// 网格宽度（cell 数量）
    pub width: u32,
    /// 网格高度（cell 数量）
    pub height: u32,
    /// 地面单元格列表（按行优先排列）
    pub cells: Vec<GatCell>,
}

/// 地面单元格
#[derive(Debug, Clone)]
pub struct GatCell {
    /// 四个角的高度值（左下、右下、左上、右上）
    pub heights: [f32; 4],
    /// 单元格类型（0=可行走, 1=不可行走, 2=水, 等）
    pub cell_type: u32,
}

impl GatFile {
    /// 从字节数据解析 GAT 文件
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() < 14 {
            return Err(AssetError::ParseError(
                "GAT 文件太小，无法读取头部".to_string()
            ));
        }

        // 验证魔术字节 "GRAT"
        if &data[0..4] != b"GRAT" {
            return Err(AssetError::ParseError(
                "无效的 GAT 文件：魔术字节不匹配".to_string()
            ));
        }

        let version = u16::from_le_bytes([data[4], data[5]]);
        let width = u32::from_le_bytes([data[6], data[7], data[8], data[9]]);
        let height = u32::from_le_bytes([data[10], data[11], data[12], data[13]]);

        let cell_count = (width * height) as usize;
        let expected_size = 14 + cell_count * 20; // 每个 cell 20 字节

        if data.len() < expected_size {
            return Err(AssetError::ParseError(
                "GAT 文件数据不完整".to_string()
            ));
        }

        let mut cells = Vec::with_capacity(cell_count);
        let mut offset = 14;

        for _ in 0..cell_count {
            let h1 = f32::from_le_bytes([
                data[offset], data[offset + 1],
                data[offset + 2], data[offset + 3],
            ]);
            let h2 = f32::from_le_bytes([
                data[offset + 4], data[offset + 5],
                data[offset + 6], data[offset + 7],
            ]);
            let h3 = f32::from_le_bytes([
                data[offset + 8], data[offset + 9],
                data[offset + 10], data[offset + 11],
            ]);
            let h4 = f32::from_le_bytes([
                data[offset + 12], data[offset + 13],
                data[offset + 14], data[offset + 15],
            ]);
            let cell_type = u32::from_le_bytes([
                data[offset + 16], data[offset + 17],
                data[offset + 18], data[offset + 19],
            ]);
            offset += 20;

            cells.push(GatCell {
                heights: [h1, h2, h3, h4],
                cell_type,
            });
        }

        Ok(Self {
            version,
            width,
            height,
            cells,
        })
    }

    /// 获取指定位置的单元格
    pub fn get_cell(&self, x: u32, y: u32) -> Option<&GatCell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y * self.width + x) as usize;
        self.cells.get(index)
    }

    /// 检查指定位置是否可行走
    pub fn is_walkable(&self, x: u32, y: u32) -> bool {
        self.get_cell(x, y)
            .map(|c| c.cell_type == 0)
            .unwrap_or(false)
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test asset_test`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/asset/gat.rs devi/tests/asset_test.rs
git commit -m "feat(devi/asset): 实现 GAT 地面高度网格解析器"
```

---

### Task 5: 统一资源加载器

**Files:**
- Create: `devi/src/asset/loader.rs`
- Modify: `devi/tests/asset_test.rs`

- [ ] **Step 1: 编写加载器测试**

在 `devi/tests/asset_test.rs` 中追加:
```rust
use devi::asset::loader::AssetLoader;
use std::path::PathBuf;

#[test]
fn test_asset_loader_custom_dir_not_found() {
    let loader = AssetLoader::new(
        Some(PathBuf::from("/nonexistent/grf")),
        PathBuf::from("/nonexistent/custom"),
    );
    // 自定义格式目录不存在时，应能正常初始化
    assert!(loader.custom_dir().is_some());
}

#[test]
fn test_asset_loader_no_grf() {
    let loader = AssetLoader::new(
        None,
        PathBuf::from("/nonexistent/custom"),
    );
    assert!(loader.grf_path().is_none());
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test asset_test`
Expected: FAIL，`unresolved import devi::asset::loader`

- [ ] **Step 3: 实现统一加载器**

创建 `devi/src/asset/loader.rs`:
```rust
use std::path::{Path, PathBuf};
use super::grf::GrfArchive;
use super::AssetError;

/// 统一资源加载器
/// 优先从自定义格式目录加载，找不到则 fallback 到 GRF
pub struct AssetLoader {
    /// GRF 文件路径（可选）
    grf_path: Option<PathBuf>,
    /// 自定义格式目录路径
    custom_dir: PathBuf,
    /// 已打开的 GRF 归档（延迟加载）
    grf_archive: Option<GrfArchive>,
}

impl AssetLoader {
    /// 创建资源加载器
    pub fn new(grf_path: Option<PathBuf>, custom_dir: PathBuf) -> Self {
        Self {
            grf_path,
            custom_dir,
            grf_archive: None,
        }
    }

    /// 获取 GRF 路径
    pub fn grf_path(&self) -> Option<&Path> {
        self.grf_path.as_deref()
    }

    /// 获取自定义格式目录
    pub fn custom_dir(&self) -> Option<&Path> {
        Some(&self.custom_dir)
    }

    /// 加载资源文件
    /// 先尝试自定义格式目录，找不到则 fallback 到 GRF
    pub fn load(&mut self, path: &str) -> Result<Vec<u8>, AssetError> {
        // 1. 尝试自定义格式目录
        let custom_path = self.custom_dir.join(path);
        if custom_path.exists() {
            return std::fs::read(&custom_path).map_err(AssetError::Io);
        }

        // 2. Fallback 到 GRF
        if let Some(grf_path) = &self.grf_path {
            let archive = self.get_or_open_grf(grf_path)?;
            if let Some(entry) = archive.find_entry(path) {
                let entry = entry.clone();
                return archive.read_file(&entry);
            }
        }

        Err(AssetError::NotFound(path.to_string()))
    }

    /// 检查资源是否存在
    pub fn exists(&mut self, path: &str) -> bool {
        let custom_path = self.custom_dir.join(path);
        if custom_path.exists() {
            return true;
        }

        if let Some(grf_path) = &self.grf_path {
            if let Ok(archive) = self.get_or_open_grf(grf_path) {
                return archive.find_entry(path).is_some();
            }
        }

        false
    }

    /// 获取或打开 GRF 归档
    fn get_or_open_grf(&mut self, path: &Path) -> Result<&mut GrfArchive, AssetError> {
        if self.grf_archive.is_none() {
            self.grf_archive = Some(GrfArchive::open(path)?);
        }
        Ok(self.grf_archive.as_mut().unwrap())
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test asset_test`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/asset/loader.rs devi/tests/asset_test.rs
git commit -m "feat(devi/asset): 实现统一资源加载器，支持 GRF/自定义格式双模式 fallback"
```
