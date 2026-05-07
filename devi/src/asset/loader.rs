use std::path::{Path, PathBuf};
use super::grf::GrfArchive;
use super::AssetError;

/// 统一资源加载器
/// 实现双模式 fallback 策略：优先从自定义格式目录加载，找不到则 fallback 到 GRF 归档。
/// 这样可以支持自定义资源覆盖，同时保留原始 GRF 数据作为后备。
pub struct AssetLoader {
    /// GRF 文件路径（可选）
    grf_path: Option<PathBuf>,
    /// 自定义格式目录路径
    custom_dir: PathBuf,
    /// 已打开的 GRF 归档（延迟加载，避免不必要的文件打开）
    grf_archive: Option<GrfArchive>,
}

impl AssetLoader {
    /// 创建资源加载器
    pub fn new(grf_path: Option<PathBuf>, custom_dir: PathBuf) -> Self {
        Self { grf_path, custom_dir, grf_archive: None }
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
    /// 优先从自定义格式目录加载（支持资源覆盖/补丁），找不到则 fallback 到 GRF 归档
    pub fn load(&mut self, path: &str) -> Result<Vec<u8>, AssetError> {
        // 第一优先级：自定义格式目录
        let custom_path = self.custom_dir.join(path);
        if custom_path.exists() {
            return std::fs::read(&custom_path).map_err(AssetError::Io);
        }

        // 第二优先级：GRF 归档（延迟打开）
        // 先将 grf_path 克隆出来，避免在调用 get_or_open_grf 时产生借用冲突
        let grf_path = self.grf_path.clone();
        if let Some(grf_path) = grf_path {
            let archive = self.get_or_open_grf(&grf_path)?;
            if let Some(entry) = archive.find_entry(path) {
                let entry = entry.clone();
                return archive.read_file(&entry);
            }
        }

        Err(AssetError::NotFound(path.to_string()))
    }

    /// 检查资源是否存在
    pub fn exists(&mut self, path: &str) -> bool {
        // 检查自定义目录
        let custom_path = self.custom_dir.join(path);
        if custom_path.exists() { return true; }

        // 检查 GRF 归档
        let grf_path = self.grf_path.clone();
        if let Some(grf_path) = grf_path {
            if let Ok(archive) = self.get_or_open_grf(&grf_path) {
                return archive.find_entry(path).is_some();
            }
        }
        false
    }

    /// 获取或打开 GRF 归档（延迟加载）
    fn get_or_open_grf(&mut self, path: &Path) -> Result<&mut GrfArchive, AssetError> {
        if self.grf_archive.is_none() {
            self.grf_archive = Some(GrfArchive::open(path)?);
        }
        Ok(self.grf_archive.as_mut().unwrap())
    }
}
